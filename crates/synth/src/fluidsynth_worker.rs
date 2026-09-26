//! Worker de síntesis SoundFont separado del hilo de audio.
//!
//! FluidSynth genera bloques en un hilo dedicado. El callback sólo consume un
//! ring SPSC reservado antes de abrir PipeWire; si el worker no alcanza, el
//! nodo entrega silencio y contabiliza el underrun sin bloquear ni asignar.

use crate::{FluidSynthEngine, SynthMidiEvent};
use estudio_daw_audio_engine::{AudioNode, AudioNodeError, SampleRingBuffer};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
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
        let ring = Arc::new(SampleRingBuffer::new(RING_SAMPLES));
        let dropped_events = Arc::new(AtomicUsize::new(0));
        let worker_errors = Arc::new(AtomicUsize::new(0));
        let underrun_samples = Arc::new(AtomicUsize::new(0));
        let (commands, command_rx) = command_queue(Arc::clone(&dropped_events));
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<(), String>>(1);
        let worker_ring = Arc::clone(&ring);
        let worker_errors_inner = Arc::clone(&worker_errors);
        let path = soundfont_path.into();
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

                    // Al dejar espacio para un bloque completo, el worker
                    // limita latencia y evita tirar muestras del ring.
                    if worker_ring.capacity() - worker_ring.available() >= block.len() {
                        if engine.render_interleaved(&mut block).is_err() {
                            worker_errors_inner.fetch_add(1, Ordering::Relaxed);
                            block.fill(0.0);
                        }
                        let _ = worker_ring.push(&block);
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
                },
                FluidSynthPcmNode {
                    ring,
                    underrun_samples,
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_renders_soundfont_notes_into_the_callback_source() {
        let _guard = crate::fluidsynth::FLUIDSYNTH_TEST_LOCK.lock().unwrap();
        let path = "/usr/share/soundfonts/FluidR3_GM.sf2";
        if !std::path::Path::new(path).is_file() {
            return;
        }
        let (worker, mut node) = SoundFontInstrumentWorker::start(path, 48_000, 0, 0).unwrap();
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
        assert_eq!(node.underrun_samples(), 0);
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
        drop(receiver);
        assert!(!sender.try_send(event));
        assert_eq!(dropped.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn pcm_source_counts_starvation_and_recovers_when_samples_arrive() {
        let ring = Arc::new(SampleRingBuffer::new(256));
        let underruns = Arc::new(AtomicUsize::new(0));
        let mut source = FluidSynthPcmNode {
            ring: Arc::clone(&ring),
            underrun_samples: Arc::clone(&underruns),
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
    }
}
