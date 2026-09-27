//! Host de reproducción usado por el transporte de escritorio.
//!
//! La preparación de instrumentos y el scheduler corren fuera del callback. El
//! callback PipeWire sólo procesa el plan ya compilado y memoria preasignada.

use estudio_daw_application::{AudioProfileSettings, TransportClock};
use estudio_daw_audio_engine::{
    render_plan_exchange, AudioNode, AudioNodeError, RenderPlanBuilder, RenderPlanControl,
    SampleRingBuffer,
};
use estudio_daw_audio_platform::{run_pipewire_output_until, PipeWireStreamConfig};
use estudio_daw_midi_engine::RecordedMidiMessage;
use estudio_daw_project_model::{AudioClip, InstrumentConfig, Project, TrackKind};
use estudio_daw_runtime_diagnostics::{audio_devices, DeviceInfo};
use estudio_daw_synth::{
    midi_event_queue, InstrumentMixerNode, SineSynthNode, SoundFontEventSender,
    SoundFontInstrumentWorker, SynthEventSender, SynthMidiEvent,
};
use std::{
    io::Read,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc, Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const MAX_AUDIO_CLIP_STREAMS: usize = 64;

struct AudioDecodePump {
    stop: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
    failure: Arc<std::sync::Mutex<Option<String>>>,
    ring: Arc<SampleRingBuffer>,
    control: estudio_daw_media_adapter::AudioPcmDecoderControl,
    thread: Option<JoinHandle<()>>,
}

impl AudioDecodePump {
    fn start(
        mut decoder: estudio_daw_media_adapter::AudioPcmDecoder,
        ring: Arc<SampleRingBuffer>,
    ) -> Result<Self, String> {
        let control = decoder.control();
        let stop = Arc::new(AtomicBool::new(false));
        let finished = Arc::new(AtomicBool::new(false));
        let failure = Arc::new(std::sync::Mutex::new(None));
        let worker_stop = Arc::clone(&stop);
        let worker_finished = Arc::clone(&finished);
        let worker_failure = Arc::clone(&failure);
        let worker_ring = Arc::clone(&ring);
        let thread = thread::Builder::new()
            .name("estudio-daw-audio-decoder".into())
            .spawn(move || {
                let mut bytes = vec![0_u8; 32 * 1024 + 4];
                let mut samples = Vec::<f32>::with_capacity(bytes.len() / 4);
                let mut carry = 0;
                let result =
                    loop {
                        if worker_stop.load(Ordering::Acquire) {
                            break Ok(());
                        }
                        let count = match decoder.read(&mut bytes[carry..]) {
                            Ok(count) => count,
                            Err(error) => break Err(error.to_string()),
                        };
                        let total = carry + count;
                        let aligned = total - total % 4;
                        samples.clear();
                        samples.extend(bytes[..aligned].chunks_exact(4).map(|chunk| {
                            f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
                        }));
                        let mut offset = 0;
                        while offset < samples.len() {
                            if worker_stop.load(Ordering::Acquire) {
                                break;
                            }
                            offset += worker_ring.push(&samples[offset..]);
                            if offset < samples.len() {
                                thread::sleep(Duration::from_millis(1));
                            }
                        }
                        carry = total - aligned;
                        if carry > 0 {
                            bytes.copy_within(aligned..total, 0);
                        }
                        if count == 0 {
                            break Ok(());
                        }
                    };
                if let Err(error) = result {
                    if let Ok(mut failure) = worker_failure.lock() {
                        *failure = Some(error);
                    }
                }
                worker_finished.store(true, Ordering::Release);
            })
            .map_err(|error| error.to_string())?;
        Ok(Self {
            stop,
            finished,
            failure,
            ring,
            control,
            thread: Some(thread),
        })
    }

    fn wait_until_primed(&self, target_samples: usize, timeout: Duration) -> Result<(), String> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(error) = self
                .failure
                .lock()
                .map_err(|_| "el estado del decodificador quedó bloqueado".to_owned())?
                .as_ref()
            {
                return Err(format!("no se pudo decodificar la región: {error}"));
            }
            let available = self.ring.available();
            if available >= target_samples
                || (self.finished.load(Ordering::Acquire) && available > 0)
            {
                return Ok(());
            }
            if self.finished.load(Ordering::Acquire) {
                return Err("el decodificador no produjo muestras de audio".into());
            }
            if Instant::now() >= deadline {
                return Err("el decodificador tardó demasiado en preparar audio".into());
            }
            thread::sleep(Duration::from_millis(2));
        }
    }
}

impl Drop for AudioDecodePump {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.control.terminate();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct AudioClipStream {
    ring: Arc<SampleRingBuffer>,
    start_frame: u64,
    duration_frames: u64,
    clip_offset_frames: u64,
    fade_in_frames: u64,
    fade_out_frames: u64,
    gain: f32,
}

struct AudioClipMixerNode {
    streams: Vec<AudioClipStream>,
    scratch: Vec<f32>,
    frame_cursor: u64,
}

/// Publica la posición musical desde los mismos bloques que consume PipeWire.
/// El nodo corre primero en el plan y conserva el resto fraccional entre
/// callbacks; la lectura desde Tauri usa un atómico y nunca toca el callback.
struct TransportPositionNode {
    clock: TransportClock,
    start_position_ticks: u64,
    sample_rate: u32,
    tempo_bpm: f64,
    position_ticks: Arc<AtomicU64>,
}

impl AudioNode for TransportPositionNode {
    fn process(&mut self, interleaved: &mut [f32]) -> Result<(), AudioNodeError> {
        if interleaved.len() % 2 != 0 {
            return Err(AudioNodeError::InvalidBlockLength);
        }
        self.clock.advance_frames(
            (interleaved.len() / 2) as u64,
            self.sample_rate,
            self.tempo_bpm,
        );
        self.position_ticks.store(
            self.start_position_ticks
                .saturating_add(self.clock.position_ticks()),
            Ordering::Release,
        );
        Ok(())
    }
}

impl AudioClipMixerNode {
    fn new(streams: Vec<AudioClipStream>, max_samples: usize) -> Self {
        Self {
            streams,
            scratch: vec![0.0; max_samples],
            frame_cursor: 0,
        }
    }
}

impl AudioNode for AudioClipMixerNode {
    fn process(&mut self, interleaved: &mut [f32]) -> Result<(), AudioNodeError> {
        if interleaved.len() % 2 != 0 || interleaved.len() > self.scratch.len() {
            return Err(AudioNodeError::InvalidBlockLength);
        }
        let frames = interleaved.len() / 2;
        let block_start = self.frame_cursor;
        for stream in &self.streams {
            let block_offset_frames = stream.start_frame.saturating_sub(block_start);
            if block_offset_frames >= frames as u64 {
                continue;
            }
            let block_offset = block_offset_frames as usize;
            let clip_frame_offset = stream
                .clip_offset_frames
                .saturating_add(block_start.saturating_sub(stream.start_frame));
            if clip_frame_offset >= stream.duration_frames {
                continue;
            }
            let remaining_clip_frames =
                usize::try_from(stream.duration_frames - clip_frame_offset).unwrap_or(usize::MAX);
            let active_frames = frames
                .saturating_sub(block_offset)
                .min(remaining_clip_frames);
            let active_samples = active_frames * 2;
            self.scratch[..active_samples].fill(0.0);
            stream.ring.pop(&mut self.scratch[..active_samples]);
            for frame in 0..active_frames {
                let clip_frame = clip_frame_offset + frame as u64;
                let mut gain = stream.gain;
                if stream.fade_in_frames > 0 && clip_frame < stream.fade_in_frames {
                    gain *= clip_frame as f32 / stream.fade_in_frames as f32;
                }
                if stream.fade_out_frames > 0
                    && clip_frame
                        >= stream
                            .duration_frames
                            .saturating_sub(stream.fade_out_frames)
                {
                    gain *= stream.duration_frames.saturating_sub(clip_frame + 1) as f32
                        / stream.fade_out_frames as f32;
                }
                let output_offset = (block_offset + frame) * 2;
                let input_offset = frame * 2;
                interleaved[output_offset] += self.scratch[input_offset] * gain;
                interleaved[output_offset + 1] += self.scratch[input_offset + 1] * gain;
            }
        }
        self.frame_cursor = self.frame_cursor.saturating_add(frames as u64);
        Ok(())
    }
}

enum EventSender {
    Sine(SynthEventSender),
    SoundFont(SoundFontEventSender),
}

impl EventSender {
    fn send(&mut self, event: SynthMidiEvent) {
        match self {
            Self::Sine(sender) => {
                let _ = sender.try_send(event);
            }
            Self::SoundFont(sender) => {
                let _ = sender.try_send(event);
            }
        }
    }
}

struct ScheduledEvent {
    at: Duration,
    sequence: u64,
    sender: usize,
    midi: SynthMidiEvent,
}

struct PlaybackSession {
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    connected: Arc<AtomicBool>,
    schedule: mpsc::Sender<PlaybackSchedule>,
    thread: JoinHandle<Result<(), String>>,
}

struct PlaybackSchedule {
    events: Vec<ScheduledEvent>,
    senders: Vec<EventSender>,
}

pub struct AudioRuntimeHost {
    playback: Option<PlaybackSession>,
    plan_control: Option<RenderPlanControl>,
    position_ticks: Arc<AtomicU64>,
}

impl Default for AudioRuntimeHost {
    fn default() -> Self {
        Self {
            playback: None,
            plan_control: None,
            position_ticks: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl AudioRuntimeHost {
    pub fn is_connected(&self) -> bool {
        self.playback
            .as_ref()
            .is_some_and(|playback| playback.connected.load(Ordering::Acquire))
    }

    pub fn position_ticks(&self) -> u64 {
        self.position_ticks.load(Ordering::Acquire)
    }

    pub fn play(
        &mut self,
        project: &Project,
        profile: AudioProfileSettings,
        start_position_ticks: u64,
    ) -> Result<(), String> {
        if let Some(playback) = &self.playback {
            if playback.connected.load(Ordering::Acquire) {
                playback.paused.store(false, Ordering::Release);
                return Ok(());
            }
        }
        self.stop()?;
        self.position_ticks
            .store(start_position_ticks, Ordering::Release);
        let config = PipeWireStreamConfig {
            period_frames: profile.device_period_frames as usize,
            ..PipeWireStreamConfig::default()
        };
        let max_samples = config
            .period_frames
            .saturating_mul(config.channels as usize);
        let paused = Arc::new(AtomicBool::new(false));
        let (plan, senders, schedule) = build_project_playback(
            project,
            config.sample_rate,
            max_samples,
            profile.playback_safety_frames as usize,
            Arc::clone(&paused),
            Arc::clone(&self.position_ticks),
            start_position_ticks,
        )?;
        let (control, processor) = render_plan_exchange(plan);
        let (schedule_tx, schedule_rx) = mpsc::channel();

        let stop = Arc::new(AtomicBool::new(false));
        let connected = Arc::new(AtomicBool::new(false));
        let playback_node = preferred_playback_node();
        let worker_stop = Arc::clone(&stop);
        let worker_paused = Arc::clone(&paused);
        let worker_connected = Arc::clone(&connected);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("estudio-daw-playback".into())
            .spawn(move || {
                let scheduler_stop = Arc::clone(&worker_stop);
                let scheduler_paused = Arc::clone(&worker_paused);
                let scheduler = thread::Builder::new()
                    .name("estudio-daw-midi-scheduler".into())
                    .spawn(move || {
                        schedule_events(
                            PlaybackSchedule {
                                events: schedule,
                                senders,
                            },
                            schedule_rx,
                            scheduler_stop,
                            scheduler_paused,
                        )
                    })
                    .map_err(|error| error.to_string())?;

                let ready_error = ready_tx.clone();
                let result = run_pipewire_output_until(
                    config,
                    processor,
                    playback_node,
                    Arc::clone(&worker_stop),
                    Arc::clone(&worker_paused),
                    ready_tx,
                )
                .map_err(|error| error.to_string());
                worker_stop.store(true, Ordering::Release);
                worker_connected.store(false, Ordering::Release);
                if let Err(error) = &result {
                    let _ = ready_error.send(Err(error.clone()));
                }
                let scheduler_result = scheduler
                    .join()
                    .map_err(|_| "el scheduler MIDI terminó inesperadamente".to_owned());
                result.and(scheduler_result)
            })
            .map_err(|error| error.to_string())?;

        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => {
                if stop.load(Ordering::Acquire) {
                    let _ = thread.join();
                    return Err("el stream PipeWire terminó durante el inicio".to_owned());
                }
                connected.store(true, Ordering::Release);
                self.playback = Some(PlaybackSession {
                    stop,
                    paused,
                    connected,
                    schedule: schedule_tx,
                    thread,
                });
                self.plan_control = Some(control);
                Ok(())
            }
            Ok(Err(error)) => {
                let _ = thread.join();
                Err(error)
            }
            Err(error) => {
                stop.store(true, Ordering::Release);
                let _ = thread.join();
                Err(format!("PipeWire no confirmó el stream: {error}"))
            }
        }
    }

    pub fn pause(&mut self, paused: bool) {
        if let Some(playback) = &self.playback {
            playback.paused.store(paused, Ordering::Release);
        }
    }

    /// Compila el estado desde la posición solicitada y publica el nuevo plan
    /// en un límite de bloque, manteniendo abierto el stream PipeWire.
    pub fn seek(
        &mut self,
        project: &Project,
        profile: AudioProfileSettings,
        position_ticks: u64,
    ) -> Result<(), String> {
        let playback = self
            .playback
            .as_ref()
            .filter(|playback| playback.connected.load(Ordering::Acquire))
            .ok_or_else(|| "la búsqueda requiere el transporte en reproducción".to_owned())?;
        if playback.paused.load(Ordering::Acquire) {
            return Err("reanuda el transporte antes de buscar mientras reproduce".to_owned());
        }
        let control = self
            .plan_control
            .as_ref()
            .ok_or_else(|| "no está disponible el control del plan de audio".to_owned())?;
        let sample_rate = PipeWireStreamConfig::default().sample_rate;
        let max_samples = (profile.device_period_frames as usize).saturating_mul(2);
        let (plan, senders, events) = build_project_playback(
            project,
            sample_rate,
            max_samples,
            profile.playback_safety_frames as usize,
            Arc::clone(&playback.paused),
            Arc::clone(&self.position_ticks),
            position_ticks,
        )?;
        if control.publish(plan).is_err() {
            return Err("el motor aún está aplicando un cambio anterior de plan".to_owned());
        }
        if playback
            .schedule
            .send(PlaybackSchedule { events, senders })
            .is_err()
        {
            return Err("el scheduler terminó antes de recibir la búsqueda".to_owned());
        }
        let deadline = Instant::now() + Duration::from_millis(250);
        while !control.reap_retired() {
            if Instant::now() >= deadline {
                return Err("PipeWire no confirmó el cambio de plan tras la búsqueda".to_owned());
            }
            thread::sleep(Duration::from_millis(1));
        }
        self.position_ticks.store(position_ticks, Ordering::Release);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), String> {
        let Some(playback) = self.playback.take() else {
            self.position_ticks.store(0, Ordering::Release);
            return Ok(());
        };
        playback.stop.store(true, Ordering::Release);
        playback.connected.store(false, Ordering::Release);
        playback
            .thread
            .join()
            .map_err(|_| "el hilo de reproducción terminó inesperadamente".to_owned())??;
        self.position_ticks.store(0, Ordering::Release);
        self.plan_control = None;
        Ok(())
    }
}

fn preferred_playback_node() -> Option<String> {
    audio_devices()
        .ok()?
        .into_iter()
        .find(is_audiobox_sink)
        .map(|device| device.name)
}

fn is_audiobox_sink(device: &DeviceInfo) -> bool {
    let descriptor = format!("{} {}", device.name, device.description).to_ascii_lowercase();
    device.media_class.to_ascii_lowercase().contains("sink")
        && (descriptor.contains("audiobox") || descriptor.contains("pre sonus"))
}

impl Drop for AudioRuntimeHost {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn build_project_playback(
    project: &Project,
    sample_rate: u32,
    max_samples: usize,
    queue_target_frames: usize,
    paused: Arc<AtomicBool>,
    position_ticks: Arc<AtomicU64>,
    start_position_ticks: u64,
) -> Result<
    (
        estudio_daw_audio_engine::RenderPlan,
        Vec<EventSender>,
        Vec<ScheduledEvent>,
    ),
    String,
> {
    let channels = 2;
    let bpm = if project.transport.tempo_bpm.is_finite() {
        project.transport.tempo_bpm.clamp(20.0, 999.0)
    } else {
        120.0
    };
    let start_position_micros = ticks_to_micros(
        start_position_ticks,
        estudio_daw_application::TICKS_PER_QUARTER,
        bpm,
    );
    let start_position_frame = ((u128::from(start_position_micros) * u128::from(sample_rate))
        / 1_000_000)
        .min(u128::from(u64::MAX)) as u64;
    let mut sources: Vec<Box<dyn estudio_daw_audio_engine::AudioNode>> = Vec::new();
    let mut workers = Vec::new();
    let mut senders = Vec::new();
    let mut schedule = Vec::new();
    let mut sequence = 0_u64;

    for track in &project.tracks {
        if !matches!(&track.kind, TrackKind::Midi) {
            continue;
        }
        let sender_index = senders.len();
        let mut has_events = false;
        for clip in project
            .midi_clips
            .iter()
            .filter(|clip| clip.track_id == track.id)
        {
            let clip_start_micros = ticks_to_micros(clip.start_tick, clip.take.ppq, bpm);
            let clip_duration_micros = ticks_to_micros(clip.duration_ticks, clip.take.ppq, bpm);
            let cursor_inside_clip = start_position_micros >= clip_start_micros
                && start_position_micros < clip_start_micros.saturating_add(clip_duration_micros);
            let mut prior_events: Vec<_> = if cursor_inside_clip {
                clip.take
                    .events
                    .iter()
                    .filter(|event| {
                        event.tick <= clip.duration_ticks
                            && ticks_to_micros(event.tick, clip.take.ppq, bpm)
                                < start_position_micros - clip_start_micros
                    })
                    .collect()
            } else {
                Vec::new()
            };
            prior_events.sort_by_key(|event| event.tick);
            let mut active_notes = Vec::<(u8, u8, u8, bool)>::new();
            let mut prior_controllers = Vec::<(u8, u8, u8)>::new();
            for event in prior_events {
                match &event.message {
                    RecordedMidiMessage::NoteOn {
                        channel,
                        note,
                        velocity,
                    } if *velocity > 0 => active_notes.push((*channel, *note, *velocity, true)),
                    RecordedMidiMessage::NoteOff { channel, note, .. }
                    | RecordedMidiMessage::NoteOn {
                        channel,
                        note,
                        velocity: 0,
                    } => {
                        if let Some(index) = active_notes.iter().rposition(
                            |(active_channel, active_note, _, key_down)| {
                                active_channel == channel && active_note == note && *key_down
                            },
                        ) {
                            let sustain_down = prior_controllers
                                .iter()
                                .rev()
                                .find(|(active_channel, controller, _)| {
                                    active_channel == channel && *controller == 64
                                })
                                .is_some_and(|(_, _, value)| *value >= 64);
                            if sustain_down {
                                active_notes[index].3 = false;
                            } else {
                                active_notes.remove(index);
                            }
                        }
                    }
                    RecordedMidiMessage::ControlChange {
                        channel,
                        controller: 64,
                        value,
                    } => {
                        let value = (*value).clamp(0, 127) as u8;
                        if let Some((_, _, previous)) =
                            prior_controllers
                                .iter_mut()
                                .find(|(active_channel, controller, _)| {
                                    active_channel == channel && *controller == 64
                                })
                        {
                            *previous = value;
                        } else {
                            prior_controllers.push((*channel, 64, value));
                        }
                        if value < 64 {
                            active_notes.retain(|(active_channel, _, _, key_down)| {
                                active_channel != channel || *key_down
                            });
                        }
                    }
                    RecordedMidiMessage::ControlChange {
                        channel,
                        controller: 123,
                        ..
                    } => active_notes.retain(|(active_channel, _, _, _)| active_channel != channel),
                    _ => {}
                }
            }
            for (channel, controller, value) in prior_controllers {
                schedule.push(ScheduledEvent {
                    at: Duration::ZERO,
                    sequence,
                    sender: sender_index,
                    midi: SynthMidiEvent::ControlChange {
                        channel,
                        controller,
                        value,
                    },
                });
                sequence = sequence.saturating_add(1);
                has_events = true;
            }
            for (channel, note, velocity, key_down) in active_notes {
                schedule.push(ScheduledEvent {
                    at: Duration::ZERO,
                    sequence,
                    sender: sender_index,
                    midi: SynthMidiEvent::NoteOn {
                        channel,
                        note,
                        velocity,
                    },
                });
                sequence = sequence.saturating_add(1);
                if !key_down {
                    schedule.push(ScheduledEvent {
                        at: Duration::ZERO,
                        sequence,
                        sender: sender_index,
                        midi: SynthMidiEvent::NoteOff { channel, note },
                    });
                    sequence = sequence.saturating_add(1);
                }
                has_events = true;
            }
            for event in &clip.take.events {
                // Include events exactly at the clip boundary (notably a
                // NoteOff at the final tick), but never schedule beyond it.
                if event.tick > clip.duration_ticks {
                    continue;
                }
                let Some(midi) = synth_event(&event.message) else {
                    continue;
                };
                let absolute_tick = clip.start_tick.saturating_add(event.tick);
                let absolute_micros = ticks_to_micros(absolute_tick, clip.take.ppq, bpm);
                if absolute_micros < start_position_micros {
                    continue;
                }
                schedule.push(ScheduledEvent {
                    at: Duration::from_micros(absolute_micros - start_position_micros),
                    sequence,
                    sender: sender_index,
                    midi,
                });
                sequence = sequence.saturating_add(1);
                has_events = true;
            }
        }
        if !has_events {
            schedule.retain(|event| event.sender != sender_index);
            continue;
        }

        match track.instrument.clone().unwrap_or(InstrumentConfig::Sine) {
            InstrumentConfig::Sine => {
                let (sender, receiver) = midi_event_queue();
                let node = SineSynthNode::new(sample_rate, channels, receiver)
                    .map_err(|error| error.to_string())?;
                sources.push(Box::new(node));
                senders.push(EventSender::Sine(sender));
            }
            InstrumentConfig::FluidSynth {
                soundfont,
                bank,
                program,
            } => {
                let (worker, node) =
                    SoundFontInstrumentWorker::start_with_queue_target_frames_paused(
                        soundfont.path,
                        sample_rate,
                        bank,
                        program,
                        queue_target_frames,
                        Arc::clone(&paused),
                    )
                    .map_err(|error| error.to_string())?;
                senders.push(EventSender::SoundFont(worker.event_sender()));
                sources.push(Box::new(node));
                workers.push(worker);
            }
        }
    }

    let mut audio_streams = Vec::new();
    let mut decoder_pumps = Vec::new();
    let audio_clips: Vec<&AudioClip> = project
        .audio_clips
        .iter()
        .filter(|clip| clip.source_id.is_some())
        .collect();
    if audio_clips.len() > MAX_AUDIO_CLIP_STREAMS {
        return Err(format!(
            "el proyecto tiene {} regiones con fuente; el límite simultáneo de reproducción es {}",
            audio_clips.len(),
            MAX_AUDIO_CLIP_STREAMS
        ));
    }
    for clip in audio_clips {
        let Some(source_id) = clip.source_id.as_deref() else {
            continue;
        };
        let source = project
            .audio_sources
            .iter()
            .find(|source| source.id == source_id && source.owner_track_id == clip.track_id)
            .ok_or_else(|| {
                format!(
                    "la región '{}' referencia una fuente inexistente",
                    clip.name
                )
            })?;
        let source_rate = source.sample_rate_hz.unwrap_or(clip.sample_rate);
        let to_output_frames = |source_samples: u64| -> u64 {
            ((u128::from(source_samples) * u128::from(sample_rate))
                / u128::from(source_rate.max(1)))
            .min(u128::from(u64::MAX)) as u64
        };
        let clip_start_micros = ticks_to_micros(clip.start_tick, 480, bpm);
        let clip_start_frame = ((u128::from(clip_start_micros) * u128::from(sample_rate))
            / 1_000_000)
            .min(u128::from(u64::MAX)) as u64;
        let clip_offset_frames = start_position_frame.saturating_sub(clip_start_frame);
        let duration_frames = to_output_frames(clip.duration_samples).max(1);
        if clip_offset_frames >= duration_frames {
            continue;
        }
        let consumed_source_samples = ((u128::from(clip_offset_frames)
            * u128::from(source_rate.max(1)))
            / u128::from(sample_rate.max(1)))
        .min(u128::from(u64::MAX)) as u64;
        let remaining_source_samples = clip
            .duration_samples
            .saturating_sub(consumed_source_samples);
        if remaining_source_samples == 0 {
            continue;
        }
        let decoder = estudio_daw_media_adapter::AudioPcmDecoder::spawn(
            &source.media.original_path,
            source_rate,
            source.channels.unwrap_or(clip.channels),
            &clip.source_channel_selection,
            clip.source_start_samples
                .saturating_add(consumed_source_samples),
            remaining_source_samples,
            sample_rate,
        )
        .map_err(|error| format!("no se pudo preparar '{}': {error}", clip.name))?;
        let ring = Arc::new(SampleRingBuffer::new(
            (sample_rate as usize).saturating_mul(2).max(2),
        ));
        let pump = AudioDecodePump::start(decoder, Arc::clone(&ring))?;
        audio_streams.push(AudioClipStream {
            ring,
            start_frame: clip_start_frame.saturating_sub(start_position_frame),
            duration_frames,
            clip_offset_frames,
            fade_in_frames: to_output_frames(clip.fade_in_samples),
            fade_out_frames: to_output_frames(clip.fade_out_samples),
            gain: 10.0_f32.powf(clip.gain_db / 20.0),
        });
        decoder_pumps.push(pump);
    }
    let target_samples = ((sample_rate as usize / 10) * 2).max(2);
    for pump in &decoder_pumps {
        pump.wait_until_primed(target_samples, Duration::from_secs(5))?;
    }

    schedule.sort_by_key(|event| (event.at, event.sequence));
    let mut builder = RenderPlanBuilder::new();
    builder.add_node(TransportPositionNode {
        clock: TransportClock::default(),
        start_position_ticks,
        sample_rate,
        tempo_bpm: bpm,
        position_ticks,
    });
    builder.add_node(InstrumentMixerNode::new(sources, max_samples));
    if !audio_streams.is_empty() {
        builder.add_node(AudioClipMixerNode::new(audio_streams, max_samples));
    }
    let mut plan = builder.build();
    for worker in workers {
        plan.retain_resource(worker);
    }
    for pump in decoder_pumps {
        plan.retain_resource(pump);
    }
    Ok((plan, senders, schedule))
}

fn synth_event(message: &RecordedMidiMessage) -> Option<SynthMidiEvent> {
    match message {
        RecordedMidiMessage::NoteOn {
            channel,
            note,
            velocity,
        } => Some(SynthMidiEvent::NoteOn {
            channel: *channel,
            note: *note,
            velocity: *velocity,
        }),
        RecordedMidiMessage::NoteOff { channel, note, .. } => Some(SynthMidiEvent::NoteOff {
            channel: *channel,
            note: *note,
        }),
        RecordedMidiMessage::ControlChange {
            channel,
            controller,
            value,
        } if *controller == 64 || *controller == 123 => Some(SynthMidiEvent::ControlChange {
            channel: *channel,
            controller: *controller as u8,
            value: (*value).clamp(0, 127) as u8,
        }),
        _ => None,
    }
}

fn ticks_to_micros(ticks: u64, ppq: u32, bpm: f64) -> u64 {
    let ppq = u128::from(ppq.max(1));
    let milli_bpm = (bpm * 1_000.0).round().max(1.0) as u128;
    (u128::from(ticks)
        .saturating_mul(60_000_000_000)
        .checked_div(ppq.saturating_mul(milli_bpm))
        .unwrap_or(u128::MAX))
    .min(u128::from(u64::MAX)) as u64
}

fn schedule_events(
    mut schedule: PlaybackSchedule,
    updates: mpsc::Receiver<PlaybackSchedule>,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
) {
    schedule
        .events
        .sort_by_key(|event| (event.at, event.sequence));
    let mut origin = Instant::now();
    let mut paused_total = Duration::ZERO;
    let mut pause_started = None;
    let mut index = 0;
    loop {
        if stop.load(Ordering::Acquire) {
            return;
        }
        if let Ok(mut replacement) = updates.try_recv() {
            replacement
                .events
                .sort_by_key(|event| (event.at, event.sequence));
            schedule = replacement;
            index = 0;
            origin = Instant::now();
            paused_total = Duration::ZERO;
            pause_started = None;
        }
        let Some(event) = schedule.events.get(index) else {
            if let Ok(replacement) = updates.recv_timeout(Duration::from_millis(2)) {
                schedule = replacement;
                schedule
                    .events
                    .sort_by_key(|event| (event.at, event.sequence));
                index = 0;
                origin = Instant::now();
                paused_total = Duration::ZERO;
                pause_started = None;
            }
            continue;
        };
        if paused.load(Ordering::Acquire) {
            pause_started.get_or_insert_with(Instant::now);
            thread::sleep(Duration::from_millis(1));
            continue;
        }
        if let Some(started) = pause_started.take() {
            paused_total = paused_total.saturating_add(started.elapsed());
        }
        let target = event.at.saturating_add(paused_total);
        let elapsed = origin.elapsed();
        if elapsed < target {
            if let Ok(replacement) =
                updates.recv_timeout((target - elapsed).min(Duration::from_millis(2)))
            {
                schedule = replacement;
                schedule
                    .events
                    .sort_by_key(|event| (event.at, event.sequence));
                index = 0;
                origin = Instant::now();
                paused_total = Duration::ZERO;
                pause_started = None;
            }
            continue;
        }
        if let Some(sender) = schedule.senders.get_mut(event.sender) {
            sender.send(event.midi);
        }
        index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use estudio_daw_midi_engine::{MidiSource, MidiTake, RecordedMidiEvent};
    use estudio_daw_project_model::{
        ImportProvenance, MidiClip, TimeSignature, Track, TrackChannelConfig, TrackMixerState,
        TrackRole, Transport,
    };

    fn midi_track(id: &str) -> Track {
        Track {
            id: id.into(),
            name: id.into(),
            kind: TrackKind::Midi,
            role: TrackRole::Instrument,
            output_track_id: None,
            channel_config: TrackChannelConfig::default(),
            color: "#58a6b8".into(),
            mixer: TrackMixerState::default(),
            notes: Vec::new(),
            audio_channels: None,
            media_source: None,
            instrument: Some(InstrumentConfig::Sine),
        }
    }

    fn project_with_two_clips() -> Project {
        let source = MidiSource { client: 1, port: 0 };
        let take = MidiTake {
            ppq: 960,
            tempo_bpm: 120,
            duration_micros: 500_000,
            events: vec![
                RecordedMidiEvent {
                    tick: 0,
                    micros_since_start: 0,
                    source: source.clone(),
                    message: RecordedMidiMessage::NoteOn {
                        channel: 0,
                        note: 60,
                        velocity: 100,
                    },
                },
                RecordedMidiEvent {
                    tick: 960,
                    micros_since_start: 500_000,
                    source,
                    message: RecordedMidiMessage::NoteOff {
                        channel: 0,
                        note: 60,
                        release_velocity: 0,
                    },
                },
            ],
        };
        Project {
            schema_version: "estudio-daw.project.v4".into(),
            project_id: "playback-test".into(),
            transport: Transport {
                tempo_bpm: 120.0,
                time_signature: TimeSignature {
                    numerator: 4,
                    denominator: 4,
                },
                loop_range: None,
            },
            tracks: vec![midi_track("track-1"), midi_track("track-2")],
            audio_sources: Vec::new(),
            audio_playlists: Vec::new(),
            scenes: Vec::new(),
            clip_slots: Vec::new(),
            midi_clips: vec![
                MidiClip {
                    id: "clip-1".into(),
                    name: "first".into(),
                    track_id: "track-1".into(),
                    start_tick: 960,
                    duration_ticks: 1920,
                    take: take.clone(),
                },
                MidiClip {
                    id: "clip-2".into(),
                    name: "second".into(),
                    track_id: "track-2".into(),
                    start_tick: 0,
                    duration_ticks: 1920,
                    take,
                },
            ],
            audio_clips: Vec::new(),
            import_provenance: ImportProvenance {
                format: "internal".into(),
                format_version: "1".into(),
                source_file: String::new(),
                warnings: Vec::new(),
            },
        }
    }

    #[test]
    fn compiles_midi_clips_from_multiple_tracks_at_project_tempo_and_clip_offsets() {
        let project = project_with_two_clips();
        let (plan, senders, schedule) = build_project_playback(
            &project,
            48_000,
            512,
            1024,
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicU64::new(0)),
            0,
        )
        .unwrap();

        assert_eq!(plan.node_count(), 2);
        assert_eq!(senders.len(), 2);
        assert_eq!(schedule.len(), 4);
        assert_eq!(schedule[0].at, Duration::ZERO);
        assert_eq!(schedule[1].at, Duration::from_millis(500));
        assert_eq!(schedule[2].at, Duration::from_millis(500));
        assert_eq!(schedule[3].at, Duration::from_secs(1));
        assert_ne!(schedule[1].sender, schedule[2].sender);
    }

    #[test]
    fn excludes_events_after_clip_duration_and_keeps_boundary_note_off() {
        let mut project = project_with_two_clips();
        for clip in &mut project.midi_clips {
            clip.duration_ticks = 960;
            clip.take.events.push(RecordedMidiEvent {
                tick: 961,
                micros_since_start: 500_521,
                source: MidiSource { client: 1, port: 0 },
                message: RecordedMidiMessage::NoteOn {
                    channel: 0,
                    note: 72,
                    velocity: 100,
                },
            });
        }
        let (_, _, schedule) = build_project_playback(
            &project,
            48_000,
            512,
            1024,
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicU64::new(0)),
            0,
        )
        .unwrap();
        assert_eq!(schedule.len(), 4);
        assert!(schedule.iter().any(|event| {
            event.at == Duration::from_millis(500)
                && matches!(event.midi, SynthMidiEvent::NoteOff { note: 60, .. })
        }));
    }

    #[test]
    fn maps_sustain_and_all_notes_off_controllers_for_clip_playback() {
        assert_eq!(
            synth_event(&RecordedMidiMessage::ControlChange {
                channel: 2,
                controller: 64,
                value: 127,
            }),
            Some(SynthMidiEvent::ControlChange {
                channel: 2,
                controller: 64,
                value: 127,
            })
        );
        assert_eq!(
            synth_event(&RecordedMidiMessage::ControlChange {
                channel: 2,
                controller: 123,
                value: 0,
            }),
            Some(SynthMidiEvent::ControlChange {
                channel: 2,
                controller: 123,
                value: 0,
            })
        );
        assert_eq!(ticks_to_micros(960, 960, 120.0), 500_000);
    }

    #[test]
    fn selects_audiobox_sink_without_confusing_monitor_source_or_other_devices() {
        let sink = DeviceInfo {
            id: 17,
            media_class: "Audio/Sink".into(),
            name: "alsa_output.usb-PreSonus_AudioBox_USB_96.analog-stereo".into(),
            description: "AudioBox USB 96".into(),
        };
        let monitor = DeviceInfo {
            id: 18,
            media_class: "Audio/Source".into(),
            name: "alsa_output.usb-PreSonus_AudioBox_USB_96.monitor".into(),
            description: "AudioBox USB 96 Monitor".into(),
        };
        assert!(is_audiobox_sink(&sink));
        assert!(!is_audiobox_sink(&monitor));
    }
}
