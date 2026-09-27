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
use estudio_daw_project_model::{
    AudioClip, InstrumentConfig, Project, TrackKind, TransportLoopRange,
};
use estudio_daw_runtime_diagnostics::{audio_devices, DeviceInfo};
use estudio_daw_synth::{
    midi_event_queue, InstrumentMixerNode, SineSynthNode, SoundFontEventSender,
    SoundFontInstrumentWorker, SynthEventSender, SynthMidiEvent,
};
use std::{
    collections::HashMap,
    io::Read,
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        mpsc, Arc, Mutex,
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
    gain_left: f32,
    gain_right: f32,
}

struct TrackProcessingNode {
    source: Box<dyn estudio_daw_audio_engine::AudioNode>,
    gain_left: f32,
    gain_right: f32,
    meter: Arc<TrackMeter>,
}

#[derive(Default)]
struct TrackMeter {
    peak: AtomicU32,
    rms: AtomicU32,
}

impl TrackMeter {
    fn update(&self, samples: &[f32]) {
        if samples.is_empty() {
            return;
        }
        let mut peak = 0.0_f32;
        let mut sum_squares = 0.0_f64;
        for &sample in samples {
            if sample.is_finite() {
                let magnitude = sample.abs();
                peak = peak.max(magnitude);
                sum_squares += f64::from(sample) * f64::from(sample);
            }
        }
        let rms = (sum_squares / samples.len() as f64).sqrt() as f32;
        // Fast attack, gentle block-rate release keeps the meter readable without
        // allocating or locking in the audio callback.
        let previous_peak = f32::from_bits(self.peak.load(Ordering::Relaxed));
        let previous_rms = f32::from_bits(self.rms.load(Ordering::Relaxed));
        self.peak
            .store(peak.max(previous_peak * 0.88).to_bits(), Ordering::Release);
        self.rms
            .store(rms.max(previous_rms * 0.92).to_bits(), Ordering::Release);
    }
}

impl AudioNode for TrackProcessingNode {
    fn process(&mut self, interleaved: &mut [f32]) -> Result<(), AudioNodeError> {
        self.source.process(interleaved)?;
        if interleaved.len() % 2 != 0 {
            return Err(AudioNodeError::InvalidBlockLength);
        }
        for stereo in interleaved.chunks_exact_mut(2) {
            stereo[0] *= self.gain_left;
            stereo[1] *= self.gain_right;
        }
        self.meter.update(interleaved);
        Ok(())
    }
}

struct AudioClipMixerNode {
    streams: Vec<AudioClipStream>,
    source_scratch: Vec<f32>,
    mix_scratch: Vec<f32>,
    frame_cursor: u64,
    meter: Arc<TrackMeter>,
}

impl AudioClipMixerNode {
    fn new(streams: Vec<AudioClipStream>, max_samples: usize, meter: Arc<TrackMeter>) -> Self {
        Self {
            streams,
            source_scratch: vec![0.0; max_samples],
            mix_scratch: vec![0.0; max_samples],
            frame_cursor: 0,
            meter,
        }
    }
}

impl AudioNode for AudioClipMixerNode {
    fn process(&mut self, interleaved: &mut [f32]) -> Result<(), AudioNodeError> {
        if interleaved.len() % 2 != 0 || interleaved.len() > self.source_scratch.len() {
            return Err(AudioNodeError::InvalidBlockLength);
        }
        let frames = interleaved.len() / 2;
        let block_start = self.frame_cursor;
        self.mix_scratch[..interleaved.len()].fill(0.0);
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
            self.source_scratch[..active_samples].fill(0.0);
            stream.ring.pop(&mut self.source_scratch[..active_samples]);
            for frame in 0..active_frames {
                let clip_frame = clip_frame_offset + frame as u64;
                let mut gain = 1.0;
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
                self.mix_scratch[output_offset] +=
                    self.source_scratch[input_offset] * gain * stream.gain_left;
                self.mix_scratch[output_offset + 1] +=
                    self.source_scratch[input_offset + 1] * gain * stream.gain_right;
            }
        }
        self.meter.update(&self.mix_scratch[..interleaved.len()]);
        let sample_count = interleaved.len();
        for (output, track_sample) in interleaved
            .iter_mut()
            .zip(self.mix_scratch[..sample_count].iter())
        {
            *output += *track_sample;
        }
        self.frame_cursor = self.frame_cursor.saturating_add(frames as u64);
        Ok(())
    }
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
    end_position_ticks: Option<u64>,
}

struct MetronomeNode {
    enabled: Arc<AtomicBool>,
    sample_rate: f64,
    frames_per_beat: f64,
    phase_offset_frames: f64,
    beat_offset: u64,
    beats_per_bar: u32,
    frame_cursor: u64,
    next_beat_ordinal: u64,
    next_beat_frame: u64,
    click_start_frame: Option<u64>,
    accented_click: bool,
}

impl MetronomeNode {
    fn new(
        enabled: Arc<AtomicBool>,
        sample_rate: u32,
        tempo_bpm: f64,
        time_signature: &estudio_daw_project_model::TimeSignature,
        start_position_ticks: u64,
    ) -> Self {
        let ticks_per_quarter = u64::from(estudio_daw_application::TICKS_PER_QUARTER);
        let beat_ticks = (ticks_per_quarter.saturating_mul(4)
            / u64::from(time_signature.denominator.max(1)))
        .max(1);
        let frames_per_beat = f64::from(sample_rate) * 60.0 / tempo_bpm
            * (4.0 / f64::from(time_signature.denominator.max(1)));
        let phase_offset_frames =
            (start_position_ticks % beat_ticks) as f64 * frames_per_beat / beat_ticks as f64;
        let next_beat_ordinal = u64::from(phase_offset_frames != 0.0);
        let next_beat_frame = ((next_beat_ordinal as f64 * frames_per_beat - phase_offset_frames)
            .ceil()
            .max(0.0)) as u64;
        Self {
            enabled,
            sample_rate: f64::from(sample_rate),
            frames_per_beat,
            phase_offset_frames,
            beat_offset: start_position_ticks / beat_ticks,
            beats_per_bar: time_signature.numerator.max(1),
            frame_cursor: 0,
            next_beat_ordinal,
            next_beat_frame,
            click_start_frame: None,
            accented_click: false,
        }
    }
}

impl AudioNode for MetronomeNode {
    fn process(&mut self, interleaved: &mut [f32]) -> Result<(), AudioNodeError> {
        if interleaved.len() % 2 != 0 {
            return Err(AudioNodeError::InvalidBlockLength);
        }
        if !self.enabled.load(Ordering::Acquire) {
            self.frame_cursor = self
                .frame_cursor
                .saturating_add((interleaved.len() / 2) as u64);
            self.next_beat_ordinal = (((self.frame_cursor as f64 + self.phase_offset_frames)
                / self.frames_per_beat)
                .floor() as u64)
                .saturating_add(1);
            self.next_beat_frame = ((self.next_beat_ordinal as f64 * self.frames_per_beat
                - self.phase_offset_frames)
                .ceil()
                .max(0.0)) as u64;
            self.click_start_frame = None;
            return Ok(());
        }

        for (frame_index, stereo) in interleaved.chunks_exact_mut(2).enumerate() {
            let current_frame = self.frame_cursor.saturating_add(frame_index as u64);
            if current_frame >= self.next_beat_frame {
                let absolute_beat = self.beat_offset.saturating_add(self.next_beat_ordinal);
                self.accented_click = absolute_beat % u64::from(self.beats_per_bar) == 0;
                self.click_start_frame = Some(current_frame);
                self.next_beat_ordinal = self.next_beat_ordinal.saturating_add(1);
                self.next_beat_frame = ((self.next_beat_ordinal as f64 * self.frames_per_beat
                    - self.phase_offset_frames)
                    .ceil()
                    .max(0.0)) as u64;
            }
            let Some(click_start) = self.click_start_frame else {
                continue;
            };
            let elapsed_frames = current_frame.saturating_sub(click_start);
            let click_frames = (self.sample_rate * 0.012) as u64;
            if elapsed_frames >= click_frames {
                self.click_start_frame = None;
                continue;
            }
            let frequency = if self.accented_click { 1_320.0 } else { 880.0 };
            let phase =
                std::f64::consts::TAU * frequency * elapsed_frames as f64 / self.sample_rate;
            let envelope = 1.0 - elapsed_frames as f64 / click_frames.max(1) as f64;
            let amplitude = if self.accented_click { 0.28 } else { 0.18 };
            let sample = (phase.sin() * envelope * amplitude) as f32;
            stereo[0] += sample;
            stereo[1] += sample;
        }
        self.frame_cursor = self
            .frame_cursor
            .saturating_add((interleaved.len() / 2) as u64);
        Ok(())
    }
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
        let position = self
            .start_position_ticks
            .saturating_add(self.clock.position_ticks());
        self.position_ticks.store(
            self.end_position_ticks
                .map_or(position, |end| position.min(end)),
            Ordering::Release,
        );
        Ok(())
    }
}

/// Silencia el resto del bloque al llegar a B, aunque el coordinador de bucle
/// publique el plan siguiente unos milisegundos después.
struct LoopBoundaryGateNode {
    frame_cursor: u64,
    end_after_frames: u64,
}

impl AudioNode for LoopBoundaryGateNode {
    fn process(&mut self, samples: &mut [f32]) -> Result<(), AudioNodeError> {
        if samples.len() % 2 != 0 {
            return Err(AudioNodeError::InvalidBlockLength);
        }
        let frames = (samples.len() / 2) as u64;
        let allowed = self
            .end_after_frames
            .saturating_sub(self.frame_cursor)
            .min(frames) as usize;
        samples[allowed * 2..].fill(0.0);
        self.frame_cursor = self.frame_cursor.saturating_add(frames);
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
    /// Tiempo relativo conservado para acotar eventos al final del loop y para
    /// inspección determinista del scheduler.
    at: Duration,
    /// Posición absoluta de sesión (960 PPQ) alcanzada por el plan de audio.
    at_tick: u64,
    sequence: u64,
    sender: usize,
    midi: SynthMidiEvent,
}

struct PlaybackSession {
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    connected: Arc<AtomicBool>,
    schedule: mpsc::Sender<SchedulerCommand>,
    thread: JoinHandle<Result<(), String>>,
    loop_thread: Option<JoinHandle<()>>,
    loop_error: Arc<std::sync::Mutex<Option<String>>>,
    project_model: Arc<LoopProjectModel>,
    active_project_revision: Arc<AtomicU64>,
}

struct LoopProjectModel {
    project: std::sync::Mutex<Project>,
    revision: AtomicU64,
}

impl LoopProjectModel {
    fn replace(&self, project: &Project) -> Result<u64, String> {
        let mut current = self
            .project
            .lock()
            .map_err(|_| "el estado del proyecto de reproducción quedó bloqueado".to_owned())?;
        if *current != *project {
            *current = project.clone();
            return Ok(self
                .revision
                .fetch_add(1, Ordering::AcqRel)
                .saturating_add(1));
        }
        Ok(self.revision.load(Ordering::Acquire))
    }

    fn snapshot(&self) -> Result<(Project, u64), String> {
        let current = self
            .project
            .lock()
            .map_err(|_| "el estado del proyecto de reproducción quedó bloqueado".to_owned())?;
        let project = current.clone();
        let revision = self.revision.load(Ordering::Acquire);
        drop(current);
        Ok((project, revision))
    }
}

struct PlaybackSchedule {
    events: Vec<ScheduledEvent>,
    senders: Vec<EventSender>,
}

enum SchedulerCommand {
    Replace(PlaybackSchedule),
    Panic,
}

pub struct AudioRuntimeHost {
    playback: Option<PlaybackSession>,
    plan_control: Option<Arc<std::sync::Mutex<RenderPlanControl>>>,
    position_ticks: Arc<AtomicU64>,
    metronome_enabled: Arc<AtomicBool>,
    track_meters: Arc<Mutex<HashMap<String, Arc<TrackMeter>>>>,
}

impl Default for AudioRuntimeHost {
    fn default() -> Self {
        Self {
            playback: None,
            plan_control: None,
            position_ticks: Arc::new(AtomicU64::new(0)),
            metronome_enabled: Arc::new(AtomicBool::new(false)),
            track_meters: Arc::new(Mutex::new(HashMap::new())),
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

    pub fn set_metronome(&self, enabled: bool) {
        self.metronome_enabled.store(enabled, Ordering::Release);
    }

    pub fn track_meter_values(&self) -> Result<HashMap<String, (f32, f32)>, String> {
        let meters = self
            .track_meters
            .lock()
            .map_err(|_| "los medidores de pista quedaron bloqueados".to_owned())?;
        Ok(meters
            .iter()
            .map(|(track_id, meter)| {
                (
                    track_id.clone(),
                    (
                        f32::from_bits(meter.peak.load(Ordering::Acquire)),
                        f32::from_bits(meter.rms.load(Ordering::Acquire)),
                    ),
                )
            })
            .collect())
    }

    fn reset_track_meters(&self) -> Result<(), String> {
        let mut meters = self
            .track_meters
            .lock()
            .map_err(|_| "los medidores de pista quedaron bloqueados".to_owned())?;
        meters.clear();
        Ok(())
    }

    pub fn position_ticks_checked(&self) -> Result<u64, String> {
        if let Some(playback) = &self.playback {
            if let Some(error) = playback
                .loop_error
                .lock()
                .map_err(|_| "el estado del bucle quedó bloqueado".to_owned())?
                .clone()
            {
                return Err(format!("falló la repetición A/B: {error}"));
            }
        }
        Ok(self.position_ticks())
    }

    /// Apaga las notas sostenidas en todos los canales de las pistas MIDI activas.
    pub fn panic(&mut self) -> Result<(), String> {
        let playback = self
            .playback
            .as_ref()
            .filter(|playback| playback.connected.load(Ordering::Acquire))
            .ok_or_else(|| "el pánico MIDI requiere el transporte en Play o pausa".to_owned())?;
        playback
            .schedule
            .send(SchedulerCommand::Panic)
            .map_err(|_| "el scheduler MIDI terminó antes de recibir el pánico".to_owned())
    }

    pub fn play(
        &mut self,
        project: &Project,
        profile: AudioProfileSettings,
        start_position_ticks: u64,
    ) -> Result<(), String> {
        if let Some(playback) = self
            .playback
            .as_ref()
            .filter(|playback| playback.connected.load(Ordering::Acquire))
        {
            let current_revision = playback.project_model.replace(project)?;
            let was_paused = playback.paused.load(Ordering::Acquire);
            let pause_flag = Arc::clone(&playback.paused);
            let active_revision = Arc::clone(&playback.active_project_revision);
            if was_paused {
                pause_flag.store(false, Ordering::Release);
                if active_revision.load(Ordering::Acquire) != current_revision {
                    self.seek(project, profile, self.position_ticks())?;
                    active_revision.store(current_revision, Ordering::Release);
                }
            }
            return Ok(());
        }
        self.stop()?;
        self.reset_track_meters()?;
        let loop_range = project.transport.loop_range;
        let start_position_ticks = loop_range.map_or(start_position_ticks, |range| {
            if start_position_ticks < range.start_tick || start_position_ticks >= range.end_tick {
                range.start_tick
            } else {
                start_position_ticks
            }
        });
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
        let project_model = Arc::new(LoopProjectModel {
            project: std::sync::Mutex::new(project.clone()),
            revision: AtomicU64::new(0),
        });
        let active_project_revision = Arc::new(AtomicU64::new(0));
        let (plan, senders, schedule) = build_project_playback_with_end(
            project,
            config.sample_rate,
            max_samples,
            profile.playback_safety_frames as usize,
            Arc::clone(&paused),
            Arc::clone(&self.position_ticks),
            Arc::clone(&self.metronome_enabled),
            Arc::clone(&self.track_meters),
            start_position_ticks,
            loop_range.map(|range| range.end_tick),
        )?;
        let (control, processor) = render_plan_exchange(plan);
        let control = Arc::new(std::sync::Mutex::new(control));
        // La primera vuelta alternativa queda decodificada antes de abrir el
        // stream; las siguientes se preparan mientras su vuelta está sonando.
        let prepared_loop = if let Some(range) = loop_range {
            Some(build_project_playback_with_end(
                project,
                config.sample_rate,
                max_samples,
                profile.playback_safety_frames as usize,
                Arc::clone(&paused),
                Arc::clone(&self.position_ticks),
                Arc::clone(&self.metronome_enabled),
                Arc::clone(&self.track_meters),
                range.start_tick,
                Some(range.end_tick),
            )?)
        } else {
            None
        };
        let (schedule_tx, schedule_rx) = mpsc::channel();

        let stop = Arc::new(AtomicBool::new(false));
        let connected = Arc::new(AtomicBool::new(false));
        let playback_node = preferred_playback_node();
        let worker_stop = Arc::clone(&stop);
        let worker_paused = Arc::clone(&paused);
        let worker_connected = Arc::clone(&connected);
        let worker_position = Arc::clone(&self.position_ticks);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("estudio-daw-playback".into())
            .spawn(move || {
                let scheduler_stop = Arc::clone(&worker_stop);
                let scheduler_position = Arc::clone(&worker_position);
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
                            scheduler_position,
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
                    paused: Arc::clone(&paused),
                    connected,
                    schedule: schedule_tx,
                    thread,
                    loop_thread: None,
                    loop_error: Arc::new(std::sync::Mutex::new(None)),
                    project_model: Arc::clone(&project_model),
                    active_project_revision: Arc::clone(&active_project_revision),
                });
                self.plan_control = Some(Arc::clone(&control));
                if let (Some(range), Some(prepared)) = (loop_range, prepared_loop) {
                    let loop_stop = Arc::clone(&self.playback.as_ref().unwrap().stop);
                    let loop_paused = Arc::clone(&paused);
                    let loop_position = Arc::clone(&self.position_ticks);
                    let loop_metronome = Arc::clone(&self.metronome_enabled);
                    let loop_track_meters = Arc::clone(&self.track_meters);
                    let loop_control = Arc::clone(&control);
                    let loop_schedule = self.playback.as_ref().unwrap().schedule.clone();
                    let loop_project = Arc::clone(&project_model);
                    let loop_active_revision = Arc::clone(&active_project_revision);
                    let loop_profile = profile;
                    let loop_error = Arc::clone(&self.playback.as_ref().unwrap().loop_error);
                    let handle = thread::Builder::new()
                        .name("estudio-daw-loop-coordinator".into())
                        .spawn(move || {
                            if let Err(error) = coordinate_loop(
                                loop_project,
                                loop_profile,
                                config.sample_rate,
                                max_samples,
                                range,
                                prepared,
                                loop_control,
                                loop_schedule,
                                loop_stop,
                                loop_paused,
                                loop_position,
                                loop_metronome,
                                loop_track_meters,
                                loop_active_revision,
                            ) {
                                if let Ok(mut state) = loop_error.lock() {
                                    *state = Some(error);
                                }
                            }
                        })
                        .map_err(|error| error.to_string());
                    let handle = match handle {
                        Ok(handle) => handle,
                        Err(error) => {
                            self.stop()?;
                            return Err(error);
                        }
                    };
                    self.playback.as_mut().unwrap().loop_thread = Some(handle);
                }
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

    /// Actualiza el modelo fuente del coordinador y recompila el plan activo
    /// mientras suena. Si está pausado, conserva el cambio para aplicarlo al
    /// reanudar, cuando PipeWire vuelva a procesar bloques.
    pub fn refresh_project(
        &mut self,
        project: &Project,
        profile: AudioProfileSettings,
    ) -> Result<bool, String> {
        let Some(playback) = self
            .playback
            .as_ref()
            .filter(|playback| playback.connected.load(Ordering::Acquire))
        else {
            return Ok(false);
        };
        let project_model = Arc::clone(&playback.project_model);
        let active_revision = Arc::clone(&playback.active_project_revision);
        let paused = playback.paused.load(Ordering::Acquire);
        let revision = project_model.replace(project)?;
        if paused {
            return Ok(false);
        }
        self.seek(project, profile, self.position_ticks())?;
        active_revision.store(revision, Ordering::Release);
        Ok(true)
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
        // Serializa la publicación con el coordinador A/B; la preparación sigue
        // fuera del callback y PipeWire nunca adquiere este bloqueo.
        let control = control
            .lock()
            .map_err(|_| "el control del plan quedó bloqueado".to_owned())?;
        let sample_rate = PipeWireStreamConfig::default().sample_rate;
        let max_samples = (profile.device_period_frames as usize).saturating_mul(2);
        let range = project.transport.loop_range;
        let position_ticks = range.map_or(position_ticks, |range| {
            if position_ticks < range.start_tick || position_ticks >= range.end_tick {
                range.start_tick
            } else {
                position_ticks
            }
        });
        let (plan, senders, events) = build_project_playback_with_end(
            project,
            sample_rate,
            max_samples,
            profile.playback_safety_frames as usize,
            Arc::clone(&playback.paused),
            Arc::clone(&self.position_ticks),
            Arc::clone(&self.metronome_enabled),
            Arc::clone(&self.track_meters),
            position_ticks,
            range.map(|range| range.end_tick),
        )?;
        if control.publish(plan).is_err() {
            return Err("el motor aún está aplicando un cambio anterior de plan".to_owned());
        }
        let deadline = Instant::now() + Duration::from_millis(250);
        while !control.reap_retired() {
            if Instant::now() >= deadline {
                return Err("PipeWire no confirmó el cambio de plan tras la búsqueda".to_owned());
            }
            thread::sleep(Duration::from_millis(1));
        }
        self.position_ticks.store(position_ticks, Ordering::Release);
        if playback
            .schedule
            .send(SchedulerCommand::Replace(PlaybackSchedule {
                events,
                senders,
            }))
            .is_err()
        {
            return Err("el scheduler terminó antes de recibir la búsqueda".to_owned());
        }
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), String> {
        let Some(playback) = self.playback.take() else {
            self.position_ticks.store(0, Ordering::Release);
            return Ok(());
        };
        playback.stop.store(true, Ordering::Release);
        playback.connected.store(false, Ordering::Release);
        if let Some(loop_thread) = playback.loop_thread {
            let _ = loop_thread.join();
        }
        playback
            .thread
            .join()
            .map_err(|_| "el hilo de reproducción terminó inesperadamente".to_owned())??;
        self.position_ticks.store(0, Ordering::Release);
        self.plan_control = None;
        Ok(())
    }
}

fn track_is_audible(track: &estudio_daw_project_model::Track, has_solo: bool) -> bool {
    track.mixer.active && (!has_solo || track.mixer.solo) && (!track.mixer.mute || track.mixer.solo)
}

fn track_gain_pan(track: &estudio_daw_project_model::Track) -> (f32, f32) {
    let gain = 10.0_f32.powf(track.mixer.gain_db / 20.0);
    let left = if track.mixer.pan > 0.0 {
        1.0 - track.mixer.pan
    } else {
        1.0
    };
    let right = if track.mixer.pan < 0.0 {
        1.0 + track.mixer.pan
    } else {
        1.0
    };
    (gain * left, gain * right)
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

#[cfg(test)]
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
    build_project_playback_with_end(
        project,
        sample_rate,
        max_samples,
        queue_target_frames,
        paused,
        position_ticks,
        Arc::new(AtomicBool::new(false)),
        Arc::new(Mutex::new(HashMap::new())),
        start_position_ticks,
        None,
    )
}

fn track_meter_for(
    meters: &Arc<Mutex<HashMap<String, Arc<TrackMeter>>>>,
    track_id: &str,
) -> Result<Arc<TrackMeter>, String> {
    let mut meters = meters
        .lock()
        .map_err(|_| "los medidores de pista quedaron bloqueados".to_owned())?;
    Ok(Arc::clone(
        meters
            .entry(track_id.to_owned())
            .or_insert_with(|| Arc::new(TrackMeter::default())),
    ))
}

fn build_project_playback_with_end(
    project: &Project,
    sample_rate: u32,
    max_samples: usize,
    queue_target_frames: usize,
    paused: Arc<AtomicBool>,
    position_ticks: Arc<AtomicU64>,
    metronome_enabled: Arc<AtomicBool>,
    track_meters: Arc<Mutex<HashMap<String, Arc<TrackMeter>>>>,
    start_position_ticks: u64,
    end_position_ticks: Option<u64>,
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
    let has_solo = project.tracks.iter().any(|track| {
        track.role != estudio_daw_project_model::TrackRole::Master
            && track.mixer.active
            && track.mixer.solo
    });
    let mut sources: Vec<Box<dyn estudio_daw_audio_engine::AudioNode>> = Vec::new();
    sources.push(Box::new(MetronomeNode::new(
        metronome_enabled,
        sample_rate,
        bpm,
        &project.transport.time_signature,
        start_position_ticks,
    )));
    let mut workers = Vec::new();
    let mut senders = Vec::new();
    let mut schedule = Vec::new();
    let mut sequence = 0_u64;

    for track in &project.tracks {
        if !matches!(&track.kind, TrackKind::Midi) {
            continue;
        }
        if !track_is_audible(track, has_solo) {
            continue;
        }
        let (gain_left, gain_right) = track_gain_pan(track);
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
            let mut prior_channel_state = Vec::<SynthMidiEvent>::new();
            let mut prior_key_pressure = Vec::<(u8, u8, u8)>::new();
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
                        controller,
                        value,
                    } if *channel < 16 && *controller <= 127 => {
                        let controller_id = *controller as u8;
                        let value = (*value).clamp(0, 127) as u8;
                        if let Some((_, _, previous)) =
                            prior_controllers
                                .iter_mut()
                                .find(|(active_channel, controller, _)| {
                                    active_channel == channel && *controller == controller_id
                                })
                        {
                            *previous = value;
                        } else {
                            prior_controllers.push((*channel, controller_id, value));
                        }
                        match controller_id {
                            64 if value < 64 => {
                                active_notes.retain(|(active_channel, _, _, key_down)| {
                                    active_channel != channel || *key_down
                                });
                            }
                            123 => active_notes
                                .retain(|(active_channel, _, _, _)| active_channel != channel),
                            _ => {}
                        }
                    }
                    RecordedMidiMessage::PitchBend { channel, value }
                        if *channel < 16 && (-8_192..=8_191).contains(value) =>
                    {
                        prior_channel_state.retain(|state| {
                            !matches!(state, SynthMidiEvent::PitchBend { channel: active, .. } if active == channel)
                        });
                        prior_channel_state.push(SynthMidiEvent::PitchBend {
                            channel: *channel,
                            value: *value as i16,
                        });
                    }
                    RecordedMidiMessage::ChannelPressure { channel, pressure }
                        if *channel < 16 && (0..=127).contains(pressure) =>
                    {
                        prior_channel_state.retain(|state| {
                            !matches!(state, SynthMidiEvent::ChannelPressure { channel: active, .. } if active == channel)
                        });
                        prior_channel_state.push(SynthMidiEvent::ChannelPressure {
                            channel: *channel,
                            pressure: *pressure as u8,
                        });
                    }
                    RecordedMidiMessage::ProgramChange { channel, program }
                        if *channel < 16 && (0..=127).contains(program) =>
                    {
                        prior_channel_state.retain(|state| {
                            !matches!(state, SynthMidiEvent::ProgramChange { channel: active, .. } if active == channel)
                        });
                        prior_channel_state.push(SynthMidiEvent::ProgramChange {
                            channel: *channel,
                            program: *program as u8,
                        });
                    }
                    RecordedMidiMessage::KeyPressure {
                        channel,
                        note,
                        pressure,
                    } if *channel < 16 && *note < 128 && *pressure < 128 => {
                        if let Some((_, _, current)) = prior_key_pressure.iter_mut().find(
                            |(active_channel, active_note, _)| {
                                active_channel == channel && active_note == note
                            },
                        ) {
                            *current = *pressure;
                        } else {
                            prior_key_pressure.push((*channel, *note, *pressure));
                        }
                    }
                    _ => {}
                }
            }
            for (channel, controller, value) in prior_controllers {
                schedule.push(ScheduledEvent {
                    at: Duration::ZERO,
                    at_tick: start_position_ticks,
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
            for midi in prior_channel_state {
                schedule.push(ScheduledEvent {
                    at: Duration::ZERO,
                    at_tick: start_position_ticks,
                    sequence,
                    sender: sender_index,
                    midi,
                });
                sequence = sequence.saturating_add(1);
                has_events = true;
            }
            for &(channel, note, velocity, key_down) in &active_notes {
                schedule.push(ScheduledEvent {
                    at: Duration::ZERO,
                    at_tick: start_position_ticks,
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
                        at_tick: start_position_ticks,
                        sequence,
                        sender: sender_index,
                        midi: SynthMidiEvent::NoteOff { channel, note },
                    });
                    sequence = sequence.saturating_add(1);
                }
                has_events = true;
            }
            for (channel, note, pressure) in prior_key_pressure {
                if active_notes
                    .iter()
                    .any(|(active_channel, active_note, _, _)| {
                        *active_channel == channel && *active_note == note
                    })
                {
                    schedule.push(ScheduledEvent {
                        at: Duration::ZERO,
                        at_tick: start_position_ticks,
                        sequence,
                        sender: sender_index,
                        midi: SynthMidiEvent::KeyPressure {
                            channel,
                            note,
                            pressure,
                        },
                    });
                    sequence = sequence.saturating_add(1);
                    has_events = true;
                }
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
                    at_tick: micros_to_session_ticks(absolute_micros, bpm),
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
                let meter = track_meter_for(&track_meters, &track.id)?;
                sources.push(Box::new(TrackProcessingNode {
                    source: Box::new(node),
                    gain_left,
                    gain_right,
                    meter,
                }));
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
                let meter = track_meter_for(&track_meters, &track.id)?;
                sources.push(Box::new(TrackProcessingNode {
                    source: Box::new(node),
                    gain_left,
                    gain_right,
                    meter,
                }));
                workers.push(worker);
            }
        }
    }

    let mut audio_streams: HashMap<String, Vec<AudioClipStream>> = HashMap::new();
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
        let track = project
            .tracks
            .iter()
            .find(|track| track.id == clip.track_id)
            .ok_or_else(|| {
                format!(
                    "la región '{}' pertenece a una pista inexistente",
                    clip.name
                )
            })?;
        if !track_is_audible(track, has_solo) {
            continue;
        }
        let (track_gain_left, track_gain_right) = track_gain_pan(track);
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
        audio_streams
            .entry(clip.track_id.clone())
            .or_default()
            .push(AudioClipStream {
                ring,
                start_frame: clip_start_frame.saturating_sub(start_position_frame),
                duration_frames,
                clip_offset_frames,
                fade_in_frames: to_output_frames(clip.fade_in_samples),
                fade_out_frames: to_output_frames(clip.fade_out_samples),
                gain_left: 10.0_f32.powf(clip.gain_db / 20.0) * track_gain_left,
                gain_right: 10.0_f32.powf(clip.gain_db / 20.0) * track_gain_right,
            });
        decoder_pumps.push(pump);
    }
    let target_samples = ((sample_rate as usize / 10) * 2).max(2);
    for pump in &decoder_pumps {
        pump.wait_until_primed(target_samples, Duration::from_secs(5))?;
    }

    let loop_end_frames = end_position_ticks.map(|end_tick| {
        let duration_micros = ticks_to_micros(
            end_tick.saturating_sub(start_position_ticks),
            estudio_daw_application::TICKS_PER_QUARTER,
            bpm,
        );
        ((u128::from(duration_micros) * u128::from(sample_rate)) / 1_000_000)
            .min(u128::from(u64::MAX)) as u64
    });
    if let Some(end_frames) = loop_end_frames {
        let loop_duration = Duration::from_secs_f64(end_frames as f64 / f64::from(sample_rate));
        schedule.retain(|event| event.at < loop_duration);
    }
    schedule.sort_by_key(|event| (event.at_tick, event.sequence));
    let mut builder = RenderPlanBuilder::new();
    builder.add_node(InstrumentMixerNode::new(sources, max_samples));
    for (track_id, streams) in audio_streams {
        let meter = track_meter_for(&track_meters, &track_id)?;
        builder.add_node(AudioClipMixerNode::new(streams, max_samples, meter));
    }
    if let Some(end_after_frames) = loop_end_frames {
        builder.add_node(LoopBoundaryGateNode {
            frame_cursor: 0,
            end_after_frames,
        });
    }
    builder.add_node(TransportPositionNode {
        clock: TransportClock::default(),
        start_position_ticks,
        sample_rate,
        tempo_bpm: bpm,
        position_ticks,
        end_position_ticks,
    });
    let mut plan = builder.build();
    for worker in workers {
        plan.retain_resource(worker);
    }
    for pump in decoder_pumps {
        plan.retain_resource(pump);
    }
    Ok((plan, senders, schedule))
}

#[allow(clippy::too_many_arguments)]
fn coordinate_loop(
    project_model: Arc<LoopProjectModel>,
    profile: AudioProfileSettings,
    sample_rate: u32,
    max_samples: usize,
    range: TransportLoopRange,
    mut prepared: (
        estudio_daw_audio_engine::RenderPlan,
        Vec<EventSender>,
        Vec<ScheduledEvent>,
    ),
    control: Arc<std::sync::Mutex<RenderPlanControl>>,
    schedule: mpsc::Sender<SchedulerCommand>,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    position_ticks: Arc<AtomicU64>,
    metronome_enabled: Arc<AtomicBool>,
    track_meters: Arc<Mutex<HashMap<String, Arc<TrackMeter>>>>,
    active_project_revision: Arc<AtomicU64>,
) -> Result<(), String> {
    let mut prepared_revision = project_model.revision.load(Ordering::Acquire);
    while !stop.load(Ordering::Acquire) {
        if paused.load(Ordering::Acquire) || position_ticks.load(Ordering::Acquire) < range.end_tick
        {
            thread::sleep(Duration::from_millis(1));
            continue;
        }
        if prepared_revision != project_model.revision.load(Ordering::Acquire) {
            (prepared, prepared_revision) = prepare_latest_loop_plan(
                &project_model,
                sample_rate,
                max_samples,
                profile,
                range,
                Arc::clone(&paused),
                Arc::clone(&position_ticks),
                Arc::clone(&metronome_enabled),
                Arc::clone(&track_meters),
            )?;
        }
        let (plan, senders, events) = prepared;
        let control = control
            .lock()
            .map_err(|_| "el control del plan quedó bloqueado".to_owned())?;
        if control.publish(plan).is_err() {
            drop(control);
            thread::sleep(Duration::from_millis(1));
            // A competing seek can occupy the exchange; rebuild after it settles.
            (prepared, prepared_revision) = prepare_latest_loop_plan(
                &project_model,
                sample_rate,
                max_samples,
                profile,
                range,
                Arc::clone(&paused),
                Arc::clone(&position_ticks),
                Arc::clone(&metronome_enabled),
                Arc::clone(&track_meters),
            )?;
            continue;
        }
        let deadline = Instant::now() + Duration::from_millis(500);
        let mut reclaimed = false;
        while !stop.load(Ordering::Acquire) && Instant::now() < deadline {
            if control.reap_retired() {
                reclaimed = true;
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        if !reclaimed {
            return Err("PipeWire no confirmó el cambio de vuelta".to_owned());
        }
        position_ticks.store(range.start_tick, Ordering::Release);
        if schedule
            .send(SchedulerCommand::Replace(PlaybackSchedule {
                events,
                senders,
            }))
            .is_err()
        {
            return Err("el scheduler MIDI terminó antes del cambio de vuelta".to_owned());
        }
        drop(control);
        active_project_revision.store(prepared_revision, Ordering::Release);
        (prepared, prepared_revision) = prepare_latest_loop_plan(
            &project_model,
            sample_rate,
            max_samples,
            profile,
            range,
            Arc::clone(&paused),
            Arc::clone(&position_ticks),
            Arc::clone(&metronome_enabled),
            Arc::clone(&track_meters),
        )?;
    }
    Ok(())
}

fn prepare_latest_loop_plan(
    project_model: &LoopProjectModel,
    sample_rate: u32,
    max_samples: usize,
    profile: AudioProfileSettings,
    range: TransportLoopRange,
    paused: Arc<AtomicBool>,
    position_ticks: Arc<AtomicU64>,
    metronome_enabled: Arc<AtomicBool>,
    track_meters: Arc<Mutex<HashMap<String, Arc<TrackMeter>>>>,
) -> Result<
    (
        (
            estudio_daw_audio_engine::RenderPlan,
            Vec<EventSender>,
            Vec<ScheduledEvent>,
        ),
        u64,
    ),
    String,
> {
    loop {
        let (project, revision) = project_model.snapshot()?;
        let prepared = build_project_playback_with_end(
            &project,
            sample_rate,
            max_samples,
            profile.playback_safety_frames as usize,
            Arc::clone(&paused),
            Arc::clone(&position_ticks),
            Arc::clone(&metronome_enabled),
            Arc::clone(&track_meters),
            range.start_tick,
            Some(range.end_tick),
        )
        .map_err(|error| format!("no se pudo preparar la siguiente vuelta A/B: {error}"))?;
        if project_model.revision.load(Ordering::Acquire) == revision {
            return Ok((prepared, revision));
        }
    }
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
        } if *channel < 16 && *controller <= 127 => Some(SynthMidiEvent::ControlChange {
            channel: *channel,
            controller: *controller as u8,
            value: (*value).clamp(0, 127) as u8,
        }),
        RecordedMidiMessage::PitchBend { channel, value }
            if *channel < 16 && (-8_192..=8_191).contains(value) =>
        {
            Some(SynthMidiEvent::PitchBend {
                channel: *channel,
                value: *value as i16,
            })
        }
        RecordedMidiMessage::KeyPressure {
            channel,
            note,
            pressure,
        } if *channel < 16 && *note < 128 && *pressure < 128 => Some(SynthMidiEvent::KeyPressure {
            channel: *channel,
            note: *note,
            pressure: *pressure,
        }),
        RecordedMidiMessage::ChannelPressure { channel, pressure }
            if *channel < 16 && (0..=127).contains(pressure) =>
        {
            Some(SynthMidiEvent::ChannelPressure {
                channel: *channel,
                pressure: *pressure as u8,
            })
        }
        RecordedMidiMessage::ProgramChange { channel, program }
            if *channel < 16 && (0..=127).contains(program) =>
        {
            Some(SynthMidiEvent::ProgramChange {
                channel: *channel,
                program: *program as u8,
            })
        }
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

fn micros_to_session_ticks(micros: u64, bpm: f64) -> u64 {
    let milli_bpm = (bpm * 1_000.0).round().max(1.0) as u128;
    (u128::from(micros)
        .saturating_mul(u128::from(estudio_daw_application::TICKS_PER_QUARTER))
        .saturating_mul(milli_bpm)
        / 60_000_000_000)
        .min(u128::from(u64::MAX)) as u64
}

fn schedule_events(
    mut schedule: PlaybackSchedule,
    updates: mpsc::Receiver<SchedulerCommand>,
    stop: Arc<AtomicBool>,
    position_ticks: Arc<AtomicU64>,
) {
    let sort_schedule = |schedule: &mut PlaybackSchedule| {
        schedule
            .events
            .sort_by_key(|event| (event.at_tick, event.sequence));
    };
    sort_schedule(&mut schedule);
    let mut index = 0;
    loop {
        if stop.load(Ordering::Acquire) {
            return;
        }
        if let Ok(command) = updates.try_recv() {
            match command {
                SchedulerCommand::Replace(mut replacement) => {
                    sort_schedule(&mut replacement);
                    schedule = replacement;
                    index = 0;
                }
                SchedulerCommand::Panic => send_all_notes_off(&mut schedule.senders),
            }
        }
        let Some(event) = schedule.events.get(index) else {
            if let Ok(command) = updates.recv_timeout(Duration::from_millis(2)) {
                apply_scheduler_command(command, &mut schedule, &mut index, &sort_schedule);
            }
            continue;
        };
        if position_ticks.load(Ordering::Acquire) < event.at_tick {
            if let Ok(command) = updates.recv_timeout(Duration::from_millis(1)) {
                apply_scheduler_command(command, &mut schedule, &mut index, &sort_schedule);
            }
            continue;
        }
        if let Some(sender) = schedule.senders.get_mut(event.sender) {
            sender.send(event.midi);
        }
        index += 1;
    }
}

fn apply_scheduler_command(
    command: SchedulerCommand,
    schedule: &mut PlaybackSchedule,
    index: &mut usize,
    sort_schedule: &impl Fn(&mut PlaybackSchedule),
) {
    match command {
        SchedulerCommand::Replace(mut replacement) => {
            sort_schedule(&mut replacement);
            *schedule = replacement;
            *index = 0;
        }
        SchedulerCommand::Panic => send_all_notes_off(&mut schedule.senders),
    }
}

fn send_all_notes_off(senders: &mut [EventSender]) {
    for sender in senders {
        for channel in 0..16 {
            sender.send(SynthMidiEvent::ControlChange {
                channel,
                controller: 64,
                value: 0,
            });
            sender.send(SynthMidiEvent::ControlChange {
                channel,
                controller: 123,
                value: 0,
            });
        }
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
            group_name: None,
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
