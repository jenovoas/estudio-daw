//! Worker de síntesis SoundFont separado del hilo de audio.
//!
//! FluidSynth genera bloques en un hilo dedicado. El callback sólo consume un
//! ring SPSC reservado antes de abrir PipeWire; si el worker no alcanza, el
//! nodo entrega silencio y contabiliza el underrun sin bloquear ni asignar.

use crate::{FluidSynthEngine, SynthMidiEvent};
use estudio_daw_audio_engine::{AudioNode, AudioNodeError, SampleRingBuffer};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use thiserror::Error;

const FRAMES_PER_CHUNK: usize = 256;
const RING_SAMPLES: usize = 4096;
const MIDI_QUEUE_CAPACITY: usize = 256;

fn effective_queue_target_frames(requested: usize) -> usize {
    requested.max(FRAMES_PER_CHUNK).min(RING_SAMPLES / 2)
}

#[derive(Debug, PartialEq, Eq)]
enum WorkerCommand {
    Midi(SynthMidiEvent),
    Stop,
}

#[derive(Debug, Error)]
pub enum SoundFontWorkerError {
    #[error("falló la inicialización del instrumento SoundFont: {0}")]
    Initialization(String),
    #[error("no se pudo crear el hilo de síntesis: {0}")]
    Thread(String),
}

/// Fuente de audio estéreo; el proceso en tiempo real no llama a FluidSynth.
pub struct FluidSynthPcmNode {
    ring: Arc<SampleRingBuffer>,
    underrun_samples: Arc<AtomicUsize>,
    current_queue_samples: Arc<AtomicUsize>,
    peak_queue_samples: Arc<AtomicUsize>,
}

impl FluidSynthPcmNode {
    pub fn underrun_samples(&self) -> usize {
        self.underrun_samples.load(Ordering::Relaxed)
    }
}

impl AudioNode for FluidSynthPcmNode {
    fn process(&mut self, interleaved: &mut [f32]) -> Result<(), AudioNodeError> {
        if interleaved.len() % 2 != 0 {
            return Err(AudioNodeError::InvalidBlockLength);
        }
        // Measure occupancy before this block consumes PCM. Relaxed atomics
        // keep the diagnostic lock-free; these values never gate audio work.
        let queued_samples = self.ring.available();
        self.current_queue_samples
            .store(queued_samples, Ordering::Relaxed);
        self.peak_queue_samples
            .fetch_max(queued_samples, Ordering::Relaxed);
        interleaved.fill(0.0);
        let read = self.ring.pop(interleaved);
        self.underrun_samples
            .fetch_add(interleaved.len() - read, Ordering::Relaxed);
        Ok(())
    }
}

/// Control no-RT del worker. Se debe mantener vivo mientras el nodo esté en
/// el RenderPlan; al destruirlo se solicita parada y se espera al hilo.
pub struct SoundFontInstrumentWorker {
    commands: SoundFontEventSender,
    thread: Option<JoinHandle<()>>,
    dropped_events: Arc<AtomicUsize>,
    worker_errors: Arc<AtomicUsize>,
    underrun_samples: Arc<AtomicUsize>,
    current_queue_samples: Arc<AtomicUsize>,
    peak_queue_samples: Arc<AtomicUsize>,
}

/// Occupación observada del puente PCM estéreo. Los frames no son una medida
/// de latencia acústica total; su duración equivalente cuantifica sólo el
/// audio ya renderizado que espera delante del callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PcmQueueMetrics {
    pub current_frames: usize,
    pub peak_frames: usize,
    pub capacity_frames: usize,
}

/// Extremo clonable para los threads MIDI/scheduler; su Drop no detiene el
/// worker, que permanece vivo bajo ownership del plan de reproducción.
#[derive(Clone)]
pub struct SoundFontEventSender {
    commands: SyncSender<WorkerCommand>,
    dropped_events: Arc<AtomicUsize>,
}

fn command_queue(
    dropped_events: Arc<AtomicUsize>,
) -> (SoundFontEventSender, Receiver<WorkerCommand>) {
    let (commands, receiver) = mpsc::sync_channel(MIDI_QUEUE_CAPACITY);
    (
        SoundFontEventSender {
            commands,
            dropped_events,
        },
        receiver,
    )
}

impl SoundFontEventSender {
    pub fn try_send(&self, event: SynthMidiEvent) -> bool {
        match self.commands.try_send(WorkerCommand::Midi(event)) {
            Ok(()) => true,
            Err(_) => {
                self.dropped_events.fetch_add(1, Ordering::Relaxed);
                false
            }
        }
    }

    pub fn dropped_events(&self) -> usize {
        self.dropped_events.load(Ordering::Relaxed)
    }
}

impl SoundFontInstrumentWorker {
    /// Abre el banco y selecciona el preset antes de publicar la fuente PCM.
    /// Tanto la carga pesada como las llamadas de síntesis se realizan dentro
    /// del nuevo worker, nunca en la interfaz ni en el callback de audio.
    pub fn start(
        soundfont_path: impl Into<String>,
        sample_rate: u32,
        bank: u16,
        program: u8,
    ) -> Result<(Self, FluidSynthPcmNode), SoundFontWorkerError> {
        Self::start_with_queue_target_frames(soundfont_path, sample_rate, bank, program, 512)
    }

    /// Abre un worker con un objetivo configurable de PCM pendiente.
    /// El ring conserva capacidad preasignada de 2048 frames; el objetivo se
    /// limita a esa capacidad y nunca se modifica desde el callback.
    pub fn start_with_queue_target_frames(
        soundfont_path: impl Into<String>,
        sample_rate: u32,
        bank: u16,
        program: u8,
        queue_target_frames: usize,
    ) -> Result<(Self, FluidSynthPcmNode), SoundFontWorkerError> {
        Self::start_with_queue_target_frames_paused(
            soundfont_path,
            sample_rate,
            bank,
            program,
            queue_target_frames,
            Arc::new(AtomicBool::new(false)),
        )
    }

    /// Abre el worker con un indicador de pausa compartido con el callback.
    /// Mientras esté activo, el worker atiende controles pero no avanza voces.
    pub fn start_with_queue_target_frames_paused(
        soundfont_path: impl Into<String>,
        sample_rate: u32,
        bank: u16,
        program: u8,
        queue_target_frames: usize,
        paused: Arc<AtomicBool>,
    ) -> Result<(Self, FluidSynthPcmNode), SoundFontWorkerError> {
        let ring = Arc::new(SampleRingBuffer::new(RING_SAMPLES));
        let dropped_events = Arc::new(AtomicUsize::new(0));
        let worker_errors = Arc::new(AtomicUsize::new(0));
        let underrun_samples = Arc::new(AtomicUsize::new(0));
        let current_queue_samples = Arc::new(AtomicUsize::new(0));
        let peak_queue_samples = Arc::new(AtomicUsize::new(0));
        let (commands, command_rx) = command_queue(Arc::clone(&dropped_events));
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<(), String>>(1);
        let worker_ring = Arc::clone(&ring);
        let worker_errors_inner = Arc::clone(&worker_errors);
        let worker_paused = Arc::clone(&paused);
        let worker_current_queue = Arc::clone(&current_queue_samples);
        let worker_peak_queue = Arc::clone(&peak_queue_samples);
        let path = soundfont_path.into();
        let queue_target_samples =
            effective_queue_target_frames(queue_target_frames).saturating_mul(2);
        let thread = thread::Builder::new()
            .name("estudio-soundfont-renderer".into())
            .spawn(move || {
                let result = FluidSynthEngine::open(&path, sample_rate).and_then(|mut engine| {
                    engine.select_preset(bank, program)?;
                    let mut block = [0.0_f32; FRAMES_PER_CHUNK * 2];
                    if let Err(error) = engine.render_interleaved(&mut block) {
                        return Err(error);
                    }
                    let _ = worker_ring.push(&block);
                    let queued = worker_ring.available();
                    worker_current_queue.store(queued, Ordering::Relaxed);
                    worker_peak_queue.fetch_max(queued, Ordering::Relaxed);
                    Ok(engine)
                });
                let mut engine = match result {
                    Ok(engine) => {
                        let _ = ready_tx.send(Ok(()));
                        engine
                    }
                    Err(error) => {
                        let _ = ready_tx.send(Err(error.to_string()));
                        return;
                    }
                };

                let mut block = [0.0_f32; FRAMES_PER_CHUNK * 2];
                // Avoid asking FluidSynth to release a note that the MIDI
                // stream never started (common with partial test takes).
                let mut active_notes = [[0_u8; 128]; 16];
                loop {
                    match command_rx.try_recv() {
                        Ok(WorkerCommand::Stop) | Err(TryRecvError::Disconnected) => break,
                        Ok(WorkerCommand::Midi(event)) => {
                            if apply_midi_event(&mut engine, event, &mut active_notes).is_err() {
                                worker_errors_inner.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        Err(TryRecvError::Empty) => {}
                    }

                    if worker_paused.load(Ordering::Acquire) {
                        match command_rx.recv_timeout(Duration::from_millis(1)) {
                            Ok(WorkerCommand::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                                break
                            }
                            Ok(WorkerCommand::Midi(event)) => {
                                if apply_midi_event(&mut engine, event, &mut active_notes).is_err()
                                {
                                    worker_errors_inner.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                            Err(mpsc::RecvTimeoutError::Timeout) => {}
                        }
                        continue;
                    }

                    // El ring reserva capacidad para tolerar scheduling, pero
                    // no se debe llenar: PCM antiguo delante de un NoteOn se
                    // convierte directamente en latencia audible.
                    let queued_samples = worker_ring.available();
                    if queued_samples + block.len() <= queue_target_samples
                        && worker_ring.capacity() - queued_samples >= block.len()
                    {
                        if engine.render_interleaved(&mut block).is_err() {
                            worker_errors_inner.fetch_add(1, Ordering::Relaxed);
                            block.fill(0.0);
                        }
                        let _ = worker_ring.push(&block);
                        let queued = worker_ring.available();
                        worker_current_queue.store(queued, Ordering::Relaxed);
                        worker_peak_queue.fetch_max(queued, Ordering::Relaxed);
                    } else {
                        // Fuera de RT: permite que MIDI/Stop se atiendan aunque
                        // el consumidor de audio esté pausado.
                        match command_rx.recv_timeout(Duration::from_millis(1)) {
                            Ok(WorkerCommand::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                                break
                            }
                            Ok(WorkerCommand::Midi(event)) => {
                                if apply_midi_event(&mut engine, event, &mut active_notes).is_err()
                                {
                                    worker_errors_inner.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                            Err(mpsc::RecvTimeoutError::Timeout) => {}
                        }
                    }
                }
            })
            .map_err(|error| SoundFontWorkerError::Thread(error.to_string()))?;

        match ready_rx.recv() {
            Ok(Ok(())) => Ok((
                Self {
                    commands,
                    thread: Some(thread),
                    dropped_events,
                    worker_errors,
                    underrun_samples: Arc::clone(&underrun_samples),
                    current_queue_samples: Arc::clone(&current_queue_samples),
                    peak_queue_samples: Arc::clone(&peak_queue_samples),
                },
                FluidSynthPcmNode {
                    ring,
                    underrun_samples,
                    current_queue_samples,
                    peak_queue_samples,
                },
            )),
            Ok(Err(error)) => {
                let _ = thread.join();
                Err(SoundFontWorkerError::Initialization(error))
            }
            Err(error) => {
                let _ = thread.join();
                Err(SoundFontWorkerError::Initialization(error.to_string()))
            }
        }
    }

    /// Emite MIDI desde el hilo de entrada/scheduler sin bloquear.
    pub fn try_send(&self, event: SynthMidiEvent) -> bool {
        self.commands.try_send(event)
    }

    pub fn event_sender(&self) -> SoundFontEventSender {
        self.commands.clone()
    }

    pub fn dropped_events(&self) -> usize {
        self.dropped_events.load(Ordering::Relaxed)
    }

    pub fn worker_errors(&self) -> usize {
        self.worker_errors.load(Ordering::Relaxed)
    }

    /// Número de muestras que el callback tuvo que completar con silencio.
    pub fn underrun_samples(&self) -> usize {
        self.underrun_samples.load(Ordering::Relaxed)
    }

    /// Instantánea y máximo observado antes de consumir un bloque de audio.
    /// El ring es intercalado estéreo, por eso las muestras se convierten a
    /// frames dividiendo por dos.
    pub fn pcm_queue_metrics(&self) -> PcmQueueMetrics {
        PcmQueueMetrics {
            current_frames: self.current_queue_samples.load(Ordering::Relaxed) / 2,
            peak_frames: self.peak_queue_samples.load(Ordering::Relaxed) / 2,
            capacity_frames: RING_SAMPLES / 2,
        }
    }
}

impl Drop for SoundFontInstrumentWorker {
    fn drop(&mut self) {
        // Drop ocurre fuera del callback. El envío bloqueante es intencional:
        // garantiza que Stop llegue incluso si la cola estaba llena.
        let _ = self.commands.commands.send(WorkerCommand::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn apply_midi_event(
    engine: &mut FluidSynthEngine,
    event: SynthMidiEvent,
    active_notes: &mut [[u8; 128]; 16],
) -> Result<(), crate::FluidSynthError> {
    match event {
        SynthMidiEvent::NoteOn {
            channel,
            note,
            velocity: 0,
        }
        | SynthMidiEvent::NoteOff { channel, note } => {
            if usize::from(channel) >= active_notes.len() || usize::from(note) >= 128 {
                return Err(crate::FluidSynthError::MidiEvent);
            }
            let active = &mut active_notes[channel as usize][note as usize];
            if *active == 0 {
                // Unmatched note-offs are harmless in MIDI and should not
                // become a backend error or disturb another active voice.
                return Ok(());
            }
            *active -= 1;
            return engine.note_off(channel, note);
        }
        SynthMidiEvent::NoteOn {
            channel,
            note,
            velocity,
        } => {
            if usize::from(channel) >= active_notes.len() || usize::from(note) >= 128 {
                return Err(crate::FluidSynthError::MidiEvent);
            }
            engine.note_on(channel, note, velocity)?;
            let active = &mut active_notes[channel as usize][note as usize];
            *active = active.saturating_add(1);
            return Ok(());
        }
        SynthMidiEvent::ControlChange {
            channel,
            controller,
            value,
        } => engine.control_change(channel, controller, value),
        SynthMidiEvent::PitchBend { channel, value } => engine.pitch_bend(channel, value),
        SynthMidiEvent::KeyPressure {
            channel,
            note,
            pressure,
        } => engine.key_pressure(channel, note, pressure),
        SynthMidiEvent::ChannelPressure { channel, pressure } => {
            engine.channel_pressure(channel, pressure)
        }
        SynthMidiEvent::ProgramChange { channel, program } => {
            engine.program_change(channel, program)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use estudio_daw_audio_engine::{render_plan_exchange, RenderPlanBuilder};

    #[test]
    fn failed_soundfont_preparation_keeps_the_active_render_plan_playable() {
        let _guard = crate::fluidsynth::FLUIDSYNTH_TEST_LOCK.lock().unwrap();
        let (mut midi_sender, midi_receiver) = crate::midi_event_queue();
        let mut initial = RenderPlanBuilder::new();
        initial.add_node(crate::SineSynthNode::new(48_000, 2, midi_receiver).unwrap());
        let (control, mut processor) = render_plan_exchange(initial.build());

        // La carga falla antes de publicar una sustitución; por diseño no toca
        // el plan activo ni su slot. La ruta usa un nombre único que no existe.
        let missing_font = std::env::temp_dir().join(format!(
            "estudio-daw-missing-{}-{}.sf2",
            std::process::id(),
            thread::current().name().unwrap_or("test")
        ));
        assert!(SoundFontInstrumentWorker::start(
            missing_font.to_string_lossy().into_owned(),
            48_000,
            0,
            0,
        )
        .is_err());

        assert!(midi_sender.try_send(SynthMidiEvent::NoteOn {
            channel: 0,
            note: 69,
            velocity: 100,
        }));
        let mut block = [0.0; 512];
        processor.process(&mut block).unwrap();
        assert!(block.iter().any(|sample| sample.abs() > 0.001));
        assert!(!control.reap_retired());
    }

    #[test]
    fn pcm_queue_target_is_clamped_to_chunk_and_preallocated_ring_capacity() {
        assert_eq!(effective_queue_target_frames(0), FRAMES_PER_CHUNK);
        assert_eq!(effective_queue_target_frames(512), 512);
        assert_eq!(effective_queue_target_frames(usize::MAX), RING_SAMPLES / 2);
    }

    #[test]
    fn worker_renders_soundfont_notes_into_the_callback_source() {
        let _guard = crate::fluidsynth::FLUIDSYNTH_TEST_LOCK.lock().unwrap();
        let path = "/usr/share/soundfonts/FluidR3_GM.sf2";
        if !std::path::Path::new(path).is_file() {
            return;
        }
        let (worker, mut node) =
            SoundFontInstrumentWorker::start_with_queue_target_frames(path, 48_000, 0, 0, 1_024)
                .unwrap();
        assert!(worker.try_send(SynthMidiEvent::NoteOn {
            channel: 0,
            note: 69,
            velocity: 110,
        }));

        let mut block = [0.0_f32; 128];
        let mut heard_note = false;
        for _ in 0..100 {
            node.process(&mut block).unwrap();
            heard_note |= block.iter().any(|sample| sample.abs() > 1.0e-5);
            if heard_note {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            heard_note,
            "SoundFont worker never delivered audible samples"
        );
        assert!(worker.try_send(SynthMidiEvent::ControlChange {
            channel: 0,
            controller: 64,
            value: 127,
        }));
        assert!(worker.try_send(SynthMidiEvent::NoteOff {
            channel: 0,
            note: 69,
        }));
        assert!(worker.try_send(SynthMidiEvent::ControlChange {
            channel: 0,
            controller: 64,
            value: 0,
        }));
        for _ in 0..20 {
            node.process(&mut block).unwrap();
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            worker.pcm_queue_metrics().peak_frames <= 1_024,
            "PCM prefetch exceeded the configured playback target"
        );
        assert_eq!(node.underrun_samples(), 0);
        assert_eq!(worker.worker_errors(), 0);
    }

    #[test]
    fn paused_worker_keeps_soundfont_voice_and_pcm_queue_stationary() {
        let _guard = crate::fluidsynth::FLUIDSYNTH_TEST_LOCK.lock().unwrap();
        let path = "/usr/share/soundfonts/FluidR3_GM.sf2";
        if !std::path::Path::new(path).is_file() {
            return;
        }
        let paused = Arc::new(AtomicBool::new(true));
        let (worker, _node) = SoundFontInstrumentWorker::start_with_queue_target_frames_paused(
            path, 48_000, 0, 0, 512, paused,
        )
        .unwrap();
        thread::sleep(Duration::from_millis(10));
        let first = worker.pcm_queue_metrics();
        thread::sleep(Duration::from_millis(10));
        let second = worker.pcm_queue_metrics();
        assert_eq!(first.current_frames, 256);
        assert_eq!(second.current_frames, first.current_frames);
        assert_eq!(second.peak_frames, first.peak_frames);
    }

    #[test]
    fn midi_command_queue_is_ordered_bounded_and_nonblocking() {
        let dropped = Arc::new(AtomicUsize::new(0));
        let (sender, receiver) = command_queue(Arc::clone(&dropped));
        let event = SynthMidiEvent::NoteOn {
            channel: 0,
            note: 60,
            velocity: 100,
        };
        assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
        for _ in 0..MIDI_QUEUE_CAPACITY {
            assert!(sender.try_send(event));
        }
        assert!(!sender.try_send(event));
        assert_eq!(sender.dropped_events(), 1);
        for _ in 0..MIDI_QUEUE_CAPACITY {
            assert_eq!(receiver.try_recv().unwrap(), WorkerCommand::Midi(event));
        }
        assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
        assert!(sender.try_send(SynthMidiEvent::ControlChange {
            channel: 0,
            controller: 64,
            value: 127,
        }));
        assert_eq!(
            receiver.try_recv().unwrap(),
            WorkerCommand::Midi(SynthMidiEvent::ControlChange {
                channel: 0,
                controller: 64,
                value: 127,
            })
        );
        drop(receiver);
        assert!(!sender.try_send(event));
        assert_eq!(dropped.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn pcm_source_counts_starvation_and_recovers_when_samples_arrive() {
        let ring = Arc::new(SampleRingBuffer::new(256));
        let underruns = Arc::new(AtomicUsize::new(0));
        let current_queue_samples = Arc::new(AtomicUsize::new(0));
        let peak_queue_samples = Arc::new(AtomicUsize::new(0));
        let mut source = FluidSynthPcmNode {
            ring: Arc::clone(&ring),
            underrun_samples: Arc::clone(&underruns),
            current_queue_samples: Arc::clone(&current_queue_samples),
            peak_queue_samples: Arc::clone(&peak_queue_samples),
        };
        let mut output = [1.0_f32; 128];
        source.process(&mut output).unwrap();
        assert!(output.iter().all(|sample| *sample == 0.0));
        assert_eq!(source.underrun_samples(), 128);

        let recovery = [0.25_f32; 128];
        assert_eq!(ring.push(&recovery), recovery.len());
        source.process(&mut output).unwrap();
        assert_eq!(output, recovery);
        assert_eq!(source.underrun_samples(), 128);
        assert_eq!(peak_queue_samples.load(Ordering::Relaxed), 128);
        assert_eq!(current_queue_samples.load(Ordering::Relaxed), 128);
    }
}
