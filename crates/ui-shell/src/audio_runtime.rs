//! Host de reproducción usado por el transporte de escritorio.
//!
//! La preparación de instrumentos y el scheduler corren fuera del callback. El
//! callback PipeWire sólo procesa el plan ya compilado y memoria preasignada.

use estudio_daw_application::{AudioProfileSettings, TransportClock};
use estudio_daw_audio_engine::{
    render_plan_exchange, AudioNode, AudioNodeError, RenderPlanBuilder, RenderPlanControl,
    SampleRingBuffer,
};
use estudio_daw_audio_platform::{
    run_pipewire_input_until, run_pipewire_output_until, PipeWireStreamConfig, WavCaptureRecorder,
};
use estudio_daw_midi_engine::{LiveMidiOutputWorker, RecordedMidiMessage};
use estudio_daw_project_model::{
    AudioClip, InstrumentConfig, PluginStateReference, Project, Track, TrackKind, TrackRole,
    TransportLoopRange,
};
use estudio_daw_runtime_diagnostics::{audio_devices, DeviceInfo};
use estudio_daw_synth::{
    midi_event_queue, SineSynthNode, SoundFontEventSender, SoundFontInstrumentWorker,
    SynthEventSender, SynthMidiEvent,
};
use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use vst3_host::{
    audio::AudioBuffers,
    midi::{MidiChannel, MidiEvent},
    Vst3Host,
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

#[derive(Default)]
struct TrackMeter {
    peak: AtomicU32,
    rms: AtomicU32,
}

const MASTER_METER_ID: &str = "__master__";

struct MasterOutputMeterNode {
    meter: Arc<TrackMeter>,
}

impl AudioNode for MasterOutputMeterNode {
    fn process(&mut self, interleaved: &mut [f32]) -> Result<(), AudioNodeError> {
        self.meter.update(interleaved);
        Ok(())
    }
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

struct AudioClipMixerNode {
    streams: Vec<AudioClipStream>,
    source_scratch: Vec<f32>,
    mix_scratch: Vec<f32>,
    frame_cursor: u64,
}

impl AudioClipMixerNode {
    fn new(streams: Vec<AudioClipStream>, max_samples: usize) -> Self {
        Self {
            streams,
            source_scratch: vec![0.0; max_samples],
            mix_scratch: vec![0.0; max_samples],
            frame_cursor: 0,
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

struct RoutedTrackSignal {
    sources: Vec<Box<dyn AudioNode>>,
    scratch: Vec<f32>,
    input_ring: Option<Arc<SampleRingBuffer>>,
    input_channels: Option<Vec<u16>>,
    output_index: Option<usize>,
    gain_left: f32,
    gain_right: f32,
    enabled: bool,
    is_master: bool,
    meter: Arc<TrackMeter>,
}

/// Resuelve una entrada de pista física o el retorno de una aplicación
/// standalone al contrato común de captura PipeWire, conservando el canalado
/// elegido por cada ruta.
fn track_audio_input(track: &Track) -> Option<(&str, &[u16])> {
    if let Some(route) = &track.input_route {
        return Some((&route.device_key, &route.channels));
    }
    match track.instrument.as_ref() {
        Some(InstrumentConfig::Standalone {
            audio_input: Some(route),
            ..
        }) => Some((&route.node_key, &route.channels)),
        _ => None,
    }
}

struct ProjectRoutingNode {
    tracks: Vec<RoutedTrackSignal>,
    order: Vec<usize>,
    global_sources: Vec<Box<dyn AudioNode>>,
    source_scratch: Vec<f32>,
    master_index: Option<usize>,
}

impl ProjectRoutingNode {
    fn new(
        project: &Project,
        mut sources_by_track: HashMap<String, Vec<Box<dyn AudioNode>>>,
        mut audio_streams: HashMap<String, Vec<AudioClipStream>>,
        global_sources: Vec<Box<dyn AudioNode>>,
        max_samples: usize,
        route_audibility: &[bool],
        output_indices: Vec<Option<usize>>,
        order: Vec<usize>,
        master_index: Option<usize>,
        input_rings: &mut HashMap<String, Arc<SampleRingBuffer>>,
        track_meters: &Arc<Mutex<HashMap<String, Arc<TrackMeter>>>>,
    ) -> Result<Self, String> {
        let mut tracks = Vec::with_capacity(project.tracks.len());
        for (index, track) in project.tracks.iter().enumerate() {
            let mut sources = sources_by_track.remove(&track.id).unwrap_or_default();
            if let Some(streams) = audio_streams.remove(&track.id) {
                sources.push(Box::new(AudioClipMixerNode::new(streams, max_samples)));
            }
            let audible = route_audibility.get(index).copied().unwrap_or(false);
            let enabled = if track.role == TrackRole::Master {
                track.mixer.active && !track.mixer.mute
            } else {
                track.mixer.active && audible && (!track.mixer.mute || track.mixer.solo)
            };
            let (gain_left, gain_right) = track_gain_pan(track);
            tracks.push(RoutedTrackSignal {
                sources,
                scratch: vec![0.0; max_samples],
                input_ring: input_rings.remove(&track.id),
                input_channels: track_audio_input(track).map(|(_, channels)| channels.to_vec()),
                output_index: output_indices[index],
                gain_left,
                gain_right,
                enabled,
                is_master: track.role == TrackRole::Master,
                meter: track_meter_for(track_meters, &track.id)?,
            });
        }
        Ok(Self {
            tracks,
            order,
            global_sources,
            source_scratch: vec![0.0; max_samples],
            master_index,
        })
    }
}

impl AudioNode for ProjectRoutingNode {
    fn process(&mut self, interleaved: &mut [f32]) -> Result<(), AudioNodeError> {
        if interleaved.len() % 2 != 0 || interleaved.len() > self.source_scratch.len() {
            return Err(AudioNodeError::InvalidBlockLength);
        }
        let sample_count = interleaved.len();
        interleaved.fill(0.0);
        for track in &mut self.tracks {
            track.scratch[..sample_count].fill(0.0);
            if let Some(ring) = &track.input_ring {
                self.source_scratch[..sample_count].fill(0.0);
                let read = ring.pop(&mut self.source_scratch[..sample_count]);
                for (frame_index, stereo) in self.source_scratch[..read].chunks_exact(2).enumerate()
                {
                    let output = &mut track.scratch[frame_index * 2..frame_index * 2 + 2];
                    match track.input_channels.as_deref() {
                        Some([channel]) => output.fill(stereo[usize::from(*channel)]),
                        Some([left, right]) => {
                            output[0] = stereo[usize::from(*left)];
                            output[1] = stereo[usize::from(*right)];
                        }
                        _ => output.copy_from_slice(stereo),
                    }
                }
            }
            for source in &mut track.sources {
                self.source_scratch[..sample_count].fill(0.0);
                source.process(&mut self.source_scratch[..sample_count])?;
                for (mixed, sample) in track
                    .scratch
                    .iter_mut()
                    .zip(self.source_scratch.iter())
                    .take(sample_count)
                {
                    *mixed += *sample;
                }
            }
        }
        for source in &mut self.global_sources {
            self.source_scratch[..sample_count].fill(0.0);
            source.process(&mut self.source_scratch[..sample_count])?;
            if let Some(master_index) = self.master_index {
                for (mixed, sample) in self.tracks[master_index]
                    .scratch
                    .iter_mut()
                    .zip(self.source_scratch.iter())
                    .take(sample_count)
                {
                    *mixed += *sample;
                }
            } else {
                for (output, sample) in interleaved
                    .iter_mut()
                    .zip(self.source_scratch[..sample_count].iter())
                {
                    *output += *sample;
                }
            }
        }
        for &index in &self.order {
            let track = &mut self.tracks[index];
            let gain_left = if track.enabled { track.gain_left } else { 0.0 };
            let gain_right = if track.enabled { track.gain_right } else { 0.0 };
            for stereo in track.scratch[..sample_count].chunks_exact_mut(2) {
                stereo[0] *= gain_left;
                stereo[1] *= gain_right;
            }
            track.meter.update(&track.scratch[..sample_count]);
            let destination = track.output_index;
            let is_master = track.is_master;
            if is_master || destination.is_none() {
                for (output, sample) in interleaved
                    .iter_mut()
                    .zip(track.scratch[..sample_count].iter())
                {
                    *output += *sample;
                }
            } else if let Some(destination) = destination {
                let (source, target) = if index < destination {
                    let (before, after) = self.tracks.split_at_mut(destination);
                    (
                        &before[index].scratch[..sample_count],
                        &mut after[0].scratch[..sample_count],
                    )
                } else {
                    let (before, after) = self.tracks.split_at_mut(index);
                    (
                        &after[0].scratch[..sample_count],
                        &mut before[destination].scratch[..sample_count],
                    )
                };
                for (output, sample) in target.iter_mut().zip(source.iter()) {
                    *output += *sample;
                }
            }
        }
        Ok(())
    }
}

fn track_output_topology(
    project: &Project,
) -> Result<(Vec<Option<usize>>, Vec<usize>, Option<usize>), String> {
    let track_indices: HashMap<&str, usize> = project
        .tracks
        .iter()
        .enumerate()
        .map(|(index, track)| (track.id.as_str(), index))
        .collect();
    let master_index = project
        .tracks
        .iter()
        .position(|track| track.role == TrackRole::Master);
    let mut output_indices = Vec::with_capacity(project.tracks.len());
    for track in &project.tracks {
        let destination = if track.role == TrackRole::Master {
            None
        } else if let Some(output_id) = track.output_track_id.as_deref() {
            Some(*track_indices.get(output_id).ok_or_else(|| {
                format!("la pista '{}' apunta a una salida inexistente", track.name)
            })?)
        } else {
            master_index
        };
        output_indices.push(destination);
    }
    let mut depths = vec![0_usize; project.tracks.len()];
    for start in 0..project.tracks.len() {
        let mut current = start;
        let mut seen = std::collections::HashSet::new();
        while let Some(destination) = output_indices[current] {
            if !seen.insert(current) {
                return Err("el ruteo interno contiene un ciclo".into());
            }
            depths[start] = depths[start].saturating_add(1);
            current = destination;
        }
        if !seen.insert(current) {
            return Err("el ruteo interno contiene un ciclo".into());
        }
    }
    let mut order: Vec<_> = (0..project.tracks.len()).collect();
    order.sort_by(|left, right| depths[*right].cmp(&depths[*left]));
    Ok((output_indices, order, master_index))
}

fn route_audibility(project: &Project, outputs: &[Option<usize>]) -> Vec<bool> {
    let mut audible = vec![false; project.tracks.len()];
    let solos: Vec<_> = project
        .tracks
        .iter()
        .enumerate()
        .filter(|(_, track)| {
            track.role != TrackRole::Master && track.mixer.active && track.mixer.solo
        })
        .map(|(index, _)| index)
        .collect();
    if solos.is_empty() {
        audible.fill(true);
        return audible;
    }
    for solo in solos {
        let mut current = solo;
        let mut seen = std::collections::HashSet::new();
        loop {
            if !seen.insert(current) {
                break;
            }
            audible[current] = true;
            let Some(destination) = outputs.get(current).copied().flatten() else {
                break;
            };
            current = destination;
        }
        for candidate in 0..project.tracks.len() {
            let mut path = Vec::new();
            let mut current = candidate;
            let mut seen = std::collections::HashSet::new();
            loop {
                if current == solo {
                    for index in path {
                        audible[index] = true;
                    }
                    audible[solo] = true;
                    break;
                }
                if !seen.insert(current) {
                    break;
                }
                path.push(current);
                let Some(destination) = outputs.get(current).copied().flatten() else {
                    break;
                };
                current = destination;
            }
        }
    }
    audible
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
    Vst3 {
        track_id: String,
        sender: std::sync::mpsc::SyncSender<Vst3InstrumentCommand>,
    },
    Alsa {
        worker: LiveMidiOutputWorker,
        channel: Option<u8>,
    },
}

impl EventSender {
    fn send(&mut self, event: SynthMidiEvent) -> Result<(), String> {
        match self {
            Self::Sine(sender) => {
                let _ = sender.try_send(event);
                Ok(())
            }
            Self::SoundFont(sender) => {
                let _ = sender.try_send(event);
                Ok(())
            }
            Self::Vst3 { sender, .. } => sender
                .send(Vst3InstrumentCommand::Midi(event))
                .map_err(|_| "el worker aislado del VST3 terminó".to_owned()),
            Self::Alsa { worker, channel } => {
                let message = match event {
                    SynthMidiEvent::NoteOn {
                        channel: event_channel,
                        note,
                        velocity,
                    } => RecordedMidiMessage::NoteOn {
                        channel: channel.unwrap_or(event_channel),
                        note,
                        velocity,
                    },
                    SynthMidiEvent::NoteOff {
                        channel: event_channel,
                        note,
                    } => RecordedMidiMessage::NoteOff {
                        channel: channel.unwrap_or(event_channel),
                        note,
                        release_velocity: 0,
                    },
                    SynthMidiEvent::ControlChange {
                        channel: event_channel,
                        controller,
                        value,
                    } => RecordedMidiMessage::ControlChange {
                        channel: channel.unwrap_or(event_channel),
                        controller: u32::from(controller),
                        value: i32::from(value),
                    },
                    SynthMidiEvent::PitchBend {
                        channel: event_channel,
                        value,
                    } => RecordedMidiMessage::PitchBend {
                        channel: channel.unwrap_or(event_channel),
                        value: i32::from(value),
                    },
                    SynthMidiEvent::KeyPressure {
                        channel: event_channel,
                        note,
                        pressure,
                    } => RecordedMidiMessage::KeyPressure {
                        channel: channel.unwrap_or(event_channel),
                        note,
                        pressure,
                    },
                    SynthMidiEvent::ChannelPressure {
                        channel: event_channel,
                        pressure,
                    } => RecordedMidiMessage::ChannelPressure {
                        channel: channel.unwrap_or(event_channel),
                        pressure: i32::from(pressure),
                    },
                    SynthMidiEvent::ProgramChange {
                        channel: event_channel,
                        program,
                    } => RecordedMidiMessage::ProgramChange {
                        channel: channel.unwrap_or(event_channel),
                        program: i32::from(program),
                    },
                };
                // `EventSender` sólo se invoca desde el scheduler, fuera del
                // callback. Si una ráfaga llena la cola, esperar evita perder
                // Note Off y dejar una voz externa sostenida.
                worker
                    .send(message)
                    .map_err(|_| "el worker del puerto MIDI externo se desconectó".to_owned())
            }
        }
    }

    fn request_vst3_editor(
        &mut self,
        open: bool,
    ) -> Result<mpsc::Receiver<Result<(), String>>, String> {
        let Self::Vst3 { sender, .. } = self else {
            return Err("la pista seleccionada no usa un instrumento VST3".to_owned());
        };
        let (reply, result) = mpsc::sync_channel(1);
        let command = if open {
            Vst3InstrumentCommand::OpenEditor(reply)
        } else {
            Vst3InstrumentCommand::CloseEditor(reply)
        };
        sender.send(command).map_err(|_| {
            "el worker VST3 no puede recibir ahora el control de interfaz".to_owned()
        })?;
        Ok(result)
    }

    fn request_vst3_state(&mut self) -> Result<mpsc::Receiver<Result<Vec<u8>, String>>, String> {
        let Self::Vst3 { sender, .. } = self else {
            return Err("la pista seleccionada no usa un instrumento VST3".to_owned());
        };
        let (reply, result) = mpsc::sync_channel(1);
        sender
            .send(Vst3InstrumentCommand::SaveState(reply))
            .map_err(|_| "el worker VST3 no puede guardar ahora el estado".to_owned())?;
        Ok(result)
    }
}

enum Vst3InstrumentCommand {
    Midi(SynthMidiEvent),
    OpenEditor(mpsc::SyncSender<Result<(), String>>),
    CloseEditor(mpsc::SyncSender<Result<(), String>>),
    SaveState(mpsc::SyncSender<Result<Vec<u8>, String>>),
}

/// El plugin y su IPC viven en un worker aislado. El callback sólo consume el
/// ring PCM preasignado y completa con silencio si el worker se retrasa.
struct Vst3PcmNode {
    ring: Arc<SampleRingBuffer>,
    failed: Arc<AtomicBool>,
}

impl AudioNode for Vst3PcmNode {
    fn process(&mut self, interleaved: &mut [f32]) -> Result<(), AudioNodeError> {
        if interleaved.len() % 2 != 0 {
            return Err(AudioNodeError::InvalidBlockLength);
        }
        if self.failed.load(Ordering::Acquire) {
            interleaved.fill(0.0);
            return Err(AudioNodeError::WorkerFailed);
        }
        interleaved.fill(0.0);
        self.ring.pop(interleaved);
        Ok(())
    }
}

struct Vst3InstrumentWorker {
    stop: Arc<AtomicBool>,
    commands: std::sync::mpsc::SyncSender<Vst3InstrumentCommand>,
    thread: Option<JoinHandle<()>>,
}

impl Vst3InstrumentWorker {
    fn start(
        plugin: estudio_daw_project_model::PluginReference,
        state: Option<estudio_daw_project_model::PluginStateReference>,
        plugin_state_root: Option<PathBuf>,
        sample_rate: u32,
        block_frames: usize,
        queue_target_frames: usize,
        paused: Arc<AtomicBool>,
    ) -> Result<(Self, Vst3PcmNode), String> {
        let ring = Arc::new(SampleRingBuffer::new(
            queue_target_frames
                .max(block_frames)
                .saturating_mul(2)
                .max(2),
        ));
        let worker_ring = Arc::clone(&ring);
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let failed = Arc::new(AtomicBool::new(false));
        let worker_failed = Arc::clone(&failed);
        let (commands, receiver) = mpsc::sync_channel(1024);
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("estudio-daw-vst3-worker".into())
            .spawn(move || {
                let result = (|| -> Result<(), String> {
                    let mut host = Vst3Host::builder()
                        .sample_rate(f64::from(sample_rate))
                        .block_size(block_frames)
                        .with_process_isolation(true)
                        .build()
                        .map_err(|error| format!("no se pudo preparar el host VST3: {error}"))?;
                    let plugin_path = plugin.path.clone();
                    let mut plugin = host
                        .load_plugin_class(&plugin.path, &plugin.unique_id)
                        .map_err(|error| format!("no se pudo cargar {plugin_path}: {error}"))?;
                    if let Some(state) = state {
                        let root = plugin_state_root.ok_or_else(|| {
                            "guarda el proyecto para restaurar el estado del VST3; se conservó la asignación".to_owned()
                        })?;
                        let bytes = load_plugin_state_bytes(&root, &state)?;
                        plugin.load_state(&bytes).map_err(|error| {
                            format!(
                                "no se pudo restaurar el estado de {plugin_path}: {error}; se conservó la asignación"
                            )
                        })?;
                    }
                    plugin.start_processing().map_err(|error| {
                        format!("no se pudo iniciar el procesamiento del VST3: {error}")
                    })?;
                    let _ = ready_sender.send(Ok(()));
                    let mut buffers = AudioBuffers::new(0, 2, block_frames, f64::from(sample_rate));
                    let mut interleaved = vec![0.0_f32; block_frames.saturating_mul(2)];
                    let mut pending_midi = Vec::new();
                    while !worker_stop.load(Ordering::Acquire) {
                        while let Ok(command) = receiver.try_recv() {
                            match command {
                                Vst3InstrumentCommand::Midi(event) => {
                                    pending_midi.push(event);
                                }
                                Vst3InstrumentCommand::OpenEditor(reply) => {
                                    // IsolatedPlugin ignora el padre: el helper crea su ventana X11.
                                    let result = plugin
                                        .open_editor(vst3_host::WindowHandle::from_x11(0))
                                        .map_err(|error| error.to_string());
                                    let _ = reply.send(result);
                                }
                                Vst3InstrumentCommand::CloseEditor(reply) => {
                                    let result =
                                        plugin.close_editor().map_err(|error| error.to_string());
                                    let _ = reply.send(result);
                                }
                                Vst3InstrumentCommand::SaveState(reply) => {
                                    let result = plugin
                                        .save_state()
                                        .map_err(|error| error.to_string());
                                    let _ = reply.send(result);
                                }
                            }
                        }
                        if paused.load(Ordering::Acquire) {
                            thread::sleep(Duration::from_millis(1));
                            continue;
                        }
                        for event in pending_midi.drain(..) {
                            let channel = MidiChannel::from_index(midi_channel(event))
                                .ok_or_else(|| "canal MIDI VST3 fuera de rango".to_owned())?;
                            let event = match event {
                                SynthMidiEvent::NoteOn { note, velocity, .. } => {
                                    MidiEvent::NoteOn {
                                        channel,
                                        note,
                                        velocity,
                                    }
                                }
                                SynthMidiEvent::NoteOff { note, .. } => MidiEvent::NoteOff {
                                    channel,
                                    note,
                                    velocity: 0,
                                },
                                SynthMidiEvent::ControlChange {
                                    controller, value, ..
                                } => MidiEvent::ControlChange {
                                    channel,
                                    controller,
                                    value,
                                },
                                SynthMidiEvent::PitchBend { value, .. } => MidiEvent::PitchBend {
                                    channel,
                                    value: (i32::from(value) + 8192).clamp(0, 16383) as u16,
                                },
                                SynthMidiEvent::KeyPressure { note, pressure, .. } => {
                                    MidiEvent::PolyAftertouch {
                                        channel,
                                        note,
                                        pressure,
                                    }
                                }
                                SynthMidiEvent::ChannelPressure { pressure, .. } => {
                                    MidiEvent::ChannelAftertouch { channel, pressure }
                                }
                                SynthMidiEvent::ProgramChange { program, .. } => {
                                    MidiEvent::ProgramChange { channel, program }
                                }
                            };
                            plugin.send_midi_event(event).map_err(|error| {
                                format!("no se pudo enviar MIDI al VST3: {error}")
                            })?;
                        }
                        if let Err(error) = plugin.process_audio(&mut buffers) {
                            if !worker_stop.load(Ordering::Acquire) {
                                return Err(format!("falló el procesamiento VST3: {error}"));
                            }
                            break;
                        }
                        if buffers.outputs.is_empty() {
                            return Err("el VST3 no expone buses de salida de audio".to_owned());
                        }
                        for frame in 0..block_frames {
                            interleaved[frame * 2] = buffers.outputs[0][frame];
                            interleaved[frame * 2 + 1] = buffers
                                .outputs
                                .get(1)
                                .map_or(buffers.outputs[0][frame], |channel| channel[frame]);
                        }
                        let mut offset = 0;
                        while offset < interleaved.len() && !worker_stop.load(Ordering::Acquire) {
                            offset += worker_ring.push(&interleaved[offset..]);
                            if offset < interleaved.len() {
                                thread::sleep(Duration::from_millis(1));
                            }
                        }
                    }
                    let _ = plugin.stop_processing();
                    Ok(())
                })();
                if let Err(error) = result {
                    worker_failed.store(true, Ordering::Release);
                    let _ = ready_sender.send(Err(error));
                }
            })
            .map_err(|error| error.to_string())?;
        match ready_receiver.recv_timeout(Duration::from_secs(30)) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                let _ = thread.join();
                return Err(error);
            }
            Err(error) => {
                stop.store(true, Ordering::Release);
                let _ = thread.join();
                return Err(format!("el host VST3 no respondió al iniciar: {error}"));
            }
        }
        let target_samples = block_frames.saturating_mul(4).min(ring.capacity()).max(2);
        let deadline = Instant::now() + Duration::from_secs(5);
        while ring.available() < target_samples && Instant::now() < deadline {
            if failed.load(Ordering::Acquire) {
                stop.store(true, Ordering::Release);
                let _ = thread.join();
                return Err("el worker VST3 falló durante el prebúfer inicial".to_owned());
            }
            thread::sleep(Duration::from_millis(1));
        }
        if ring.available() < target_samples {
            stop.store(true, Ordering::Release);
            let _ = thread.join();
            return Err("el VST3 no pudo llenar su prebúfer inicial a tiempo".to_owned());
        }
        Ok((
            Self {
                stop,
                commands,
                thread: Some(thread),
            },
            Vst3PcmNode { ring, failed },
        ))
    }

    fn command_sender(&self) -> std::sync::mpsc::SyncSender<Vst3InstrumentCommand> {
        self.commands.clone()
    }
}

impl Drop for Vst3InstrumentWorker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn plugin_state_relative_path(track_id: &str) -> String {
    let safe: String = track_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character
            } else {
                '_'
            }
        })
        .collect();
    format!("plugin-state/{safe}.bin")
}

fn load_plugin_state_bytes(root: &Path, state: &PluginStateReference) -> Result<Vec<u8>, String> {
    let relative = Path::new(&state.path);
    if relative.is_absolute() || relative.components().any(|part| part.as_os_str() == "..") {
        return Err(format!(
            "la ruta de estado VST3 '{}' no es portable; se conservó la asignación",
            state.path
        ));
    }
    let path = root.join(relative);
    if !path.is_file() {
        return Err(format!(
            "no está el estado VST3 '{}'; se conservó el instrumento asignado",
            state.path
        ));
    }
    if let Some(expected) = state.sha256.as_ref() {
        let actual = estudio_daw_media_adapter::content_hash(&path).map_err(|error| {
            format!(
                "no se pudo comprobar el estado VST3 '{}': {error}",
                state.path
            )
        })?;
        if &actual != expected {
            return Err(format!(
                "el estado VST3 '{}' no coincide con la firma guardada; se conservó la asignación",
                state.path
            ));
        }
    }
    std::fs::read(&path)
        .map_err(|error| format!("no se pudo leer el estado VST3 '{}': {error}", state.path))
}

fn write_plugin_state_file(
    root: &Path,
    track_id: &str,
    bytes: &[u8],
) -> Result<PluginStateReference, String> {
    let relative = plugin_state_relative_path(track_id);
    let path = root.join(&relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("no se pudo preparar plugin-state: {error}"))?;
    }
    std::fs::write(&path, bytes)
        .map_err(|error| format!("no se pudo guardar el estado VST3 '{relative}': {error}"))?;
    let sha256 = estudio_daw_media_adapter::content_hash(&path)
        .map_err(|error| format!("no se pudo firmar el estado VST3 '{relative}': {error}"))?;
    Ok(PluginStateReference {
        path: relative,
        sha256: Some(sha256),
    })
}

fn midi_channel(event: SynthMidiEvent) -> u8 {
    match event {
        SynthMidiEvent::NoteOn { channel, .. }
        | SynthMidiEvent::NoteOff { channel, .. }
        | SynthMidiEvent::ControlChange { channel, .. }
        | SynthMidiEvent::PitchBend { channel, .. }
        | SynthMidiEvent::KeyPressure { channel, .. }
        | SynthMidiEvent::ChannelPressure { channel, .. }
        | SynthMidiEvent::ProgramChange { channel, .. } => channel,
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
    input_stop: Arc<AtomicBool>,
    input_threads: Vec<JoinHandle<()>>,
    input_rings: HashMap<String, Arc<SampleRingBuffer>>,
    input_recordings: Vec<PendingInputRecording>,
}

struct PendingInputRecording {
    track_id: String,
    track_name: String,
    start_tick: u64,
    channel_selection: Vec<u16>,
    path: PathBuf,
    recorder: Arc<WavCaptureRecorder>,
}

pub(crate) struct FinishedInputRecording {
    pub track_id: String,
    pub track_name: String,
    pub start_tick: u64,
    pub channel_selection: Vec<u16>,
    pub path: PathBuf,
    pub sample_rate_hz: u32,
    pub frames: u64,
    pub dropped_samples: u64,
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

struct ManagedStandaloneProcess {
    application_path: String,
    wine_prefix: Option<String>,
    child: Child,
}

enum SchedulerCommand {
    Replace(PlaybackSchedule),
    Panic,
    Vst3Editor {
        track_id: String,
        open: bool,
        reply: mpsc::Sender<Result<(), String>>,
    },
    Vst3SaveState {
        track_id: String,
        reply: mpsc::Sender<Result<Vec<u8>, String>>,
    },
}

pub struct AudioRuntimeHost {
    playback: Option<PlaybackSession>,
    plan_control: Option<Arc<std::sync::Mutex<RenderPlanControl>>>,
    position_ticks: Arc<AtomicU64>,
    metronome_enabled: Arc<AtomicBool>,
    track_meters: Arc<Mutex<HashMap<String, Arc<TrackMeter>>>>,
    standalone_processes: HashMap<String, ManagedStandaloneProcess>,
    plugin_state_root: Option<PathBuf>,
}

impl Default for AudioRuntimeHost {
    fn default() -> Self {
        Self {
            playback: None,
            plan_control: None,
            position_ticks: Arc::new(AtomicU64::new(0)),
            metronome_enabled: Arc::new(AtomicBool::new(false)),
            track_meters: Arc::new(Mutex::new(HashMap::new())),
            standalone_processes: HashMap::new(),
            plugin_state_root: None,
        }
    }
}

impl AudioRuntimeHost {
    pub fn is_connected(&self) -> bool {
        self.playback
            .as_ref()
            .is_some_and(|playback| playback.connected.load(Ordering::Acquire))
    }

    pub fn is_recording(&self) -> bool {
        self.playback
            .as_ref()
            .is_some_and(|playback| !playback.input_recordings.is_empty())
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
        let mut values: HashMap<String, (f32, f32)> = meters
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
            .collect();
        if let Some(master) = meters.get(MASTER_METER_ID) {
            values.insert(
                MASTER_METER_ID.to_owned(),
                (
                    f32::from_bits(master.peak.load(Ordering::Acquire)),
                    f32::from_bits(master.rms.load(Ordering::Acquire)),
                ),
            );
        }
        Ok(values)
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

    /// Abre o cierra la ventana nativa del instrumento VST3 de una pista activa.
    /// El plugin vive en el helper; esta llamada espera a que responda la creación GUI.
    pub fn set_vst3_editor(&mut self, track_id: &str, open: bool) -> Result<(), String> {
        let playback = self
            .playback
            .as_ref()
            .filter(|playback| playback.connected.load(Ordering::Acquire))
            .ok_or_else(|| {
                "dale a Play para cargar el instrumento y abrir sus controles".to_owned()
            })?;
        let (reply, result) = mpsc::channel();
        playback
            .schedule
            .send(SchedulerCommand::Vst3Editor {
                track_id: track_id.to_owned(),
                open,
                reply,
            })
            .map_err(|_| "el scheduler MIDI terminó antes de recibir la orden GUI".to_owned())?;
        result
            .recv_timeout(Duration::from_secs(35))
            .map_err(|error| {
                format!("el instrumento no respondió al abrir sus controles: {error}")
            })?
    }

    /// Captura y escribe el estado binario de cada VST3 activo junto al proyecto.
    pub fn persist_vst3_states(
        &self,
        root: &Path,
    ) -> Result<Vec<(String, PluginStateReference)>, String> {
        let captured = self.capture_vst3_states()?;
        captured
            .into_iter()
            .map(|(track_id, bytes)| {
                write_plugin_state_file(root, &track_id, &bytes).map(|state| (track_id, state))
            })
            .collect()
    }

    /// Captura el estado binario de cada VST3 activo. El helper debe seguir vivo.
    pub fn capture_vst3_states(&self) -> Result<Vec<(String, Vec<u8>)>, String> {
        let Some(playback) = self
            .playback
            .as_ref()
            .filter(|playback| playback.connected.load(Ordering::Acquire))
        else {
            return Ok(Vec::new());
        };
        let project = playback.project_model.snapshot()?.0;
        let mut captured = Vec::new();
        for track in &project.tracks {
            let Some(InstrumentConfig::Vst3 { .. }) = track.instrument else {
                continue;
            };
            let (reply, result) = mpsc::channel();
            if playback
                .schedule
                .send(SchedulerCommand::Vst3SaveState {
                    track_id: track.id.clone(),
                    reply,
                })
                .is_err()
            {
                continue;
            }
            let bytes = match result.recv_timeout(Duration::from_secs(35)) {
                Ok(Ok(bytes)) => bytes,
                Ok(Err(error)) if error.contains("no tiene un instrumento VST3 activo") => {
                    continue;
                }
                Ok(Err(error)) => {
                    return Err(format!(
                        "no se pudo guardar el estado de '{}': {error}",
                        track.name
                    ));
                }
                Err(error) => {
                    return Err(format!(
                        "el instrumento de '{}' no entregó su estado: {error}",
                        track.name
                    ));
                }
            };
            captured.push((track.id.clone(), bytes));
        }
        Ok(captured)
    }

    pub fn play(
        &mut self,
        project: &Project,
        profile: AudioProfileSettings,
        start_position_ticks: u64,
        backend_device_key: &str,
        recording_directory: Option<PathBuf>,
        plugin_state_root: Option<PathBuf>,
    ) -> Result<(), String> {
        self.ensure_standalone_applications(project)?;
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
        let playback_node = playback_node_for_key(backend_device_key)?;
        self.stop()?;
        self.plugin_state_root = plugin_state_root;
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
        let input_rings: HashMap<_, _> = project
            .tracks
            .iter()
            .filter(|track| track_audio_input(track).is_some())
            .map(|track| {
                (
                    track.id.clone(),
                    Arc::new(SampleRingBuffer::new(
                        config
                            .period_frames
                            .saturating_mul(config.channels as usize)
                            .saturating_mul(4),
                    )),
                )
            })
            .collect();
        let input_stop = Arc::new(AtomicBool::new(false));
        let mut input_threads: Vec<JoinHandle<()>> = Vec::new();
        let mut input_recordings = Vec::new();
        if let Some(directory) = recording_directory.as_ref() {
            for track in project
                .tracks
                .iter()
                .filter(|track| track.record_armed && track.input_route.is_some())
            {
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                let safe_id: String = track
                    .id
                    .chars()
                    .map(|character| {
                        if character.is_ascii_alphanumeric() || character == '-' {
                            character
                        } else {
                            '_'
                        }
                    })
                    .collect();
                let path = directory.join(format!("{}-{timestamp}.wav", safe_id));
                let recorder = WavCaptureRecorder::new(
                    path.clone(),
                    config.sample_rate,
                    config.channels as u16,
                    (config.sample_rate as usize).saturating_mul(2),
                )
                .map_err(|error| {
                    format!(
                        "no se pudo preparar la grabación de '{}': {error}",
                        track.name
                    )
                })?;
                input_recordings.push(PendingInputRecording {
                    track_id: track.id.clone(),
                    track_name: track.name.clone(),
                    start_tick: start_position_ticks,
                    channel_selection: track
                        .input_route
                        .as_ref()
                        .expect("filtro de pista armada con ruta")
                        .channels
                        .clone(),
                    path,
                    recorder: Arc::new(recorder),
                });
            }
        }
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
            input_rings.clone(),
            self.plugin_state_root.clone(),
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
                input_rings.clone(),
                self.plugin_state_root.clone(),
            )?)
        } else {
            None
        };
        for track in project
            .tracks
            .iter()
            .filter(|track| track_audio_input(track).is_some())
        {
            let (device_key, _) = track_audio_input(track).expect("filtro de ruta");
            let target = device_key
                .strip_prefix("pipewire:")
                .ok_or_else(|| {
                    format!(
                        "la entrada de audio de '{}' no pertenece a PipeWire",
                        track.name
                    )
                })?
                .to_owned();
            let available = audio_devices()
                .map_err(|error| {
                    format!("no se pudieron consultar las entradas PipeWire: {error}")
                })?
                .into_iter()
                .any(|device| {
                    device.name == target
                        && device.media_class.to_ascii_lowercase().contains("audio")
                        && !device.name.to_ascii_lowercase().contains(".monitor")
                        && !device.media_class.to_ascii_lowercase().contains("monitor")
                });
            if !available {
                input_stop.store(true, Ordering::Release);
                for input_thread in input_threads.drain(..) {
                    let _ = input_thread.join();
                }
                return Err(format!(
                    "la entrada de audio seleccionada para '{}' ya no está disponible: {target}",
                    track.name
                ));
            }
            let ring = Arc::clone(input_rings.get(&track.id).expect("ring por pista"));
            let recorder = input_recordings
                .iter()
                .find(|recording| recording.track_id == track.id)
                .map(|recording| Arc::clone(&recording.recorder));
            let stop_input = Arc::clone(&input_stop);
            let (input_ready_tx, input_ready_rx) = mpsc::sync_channel(1);
            let input_config = config;
            let handle_result = thread::Builder::new()
                .name(format!("estudio-daw-input-{}", track.id))
                .spawn(move || {
                    let _ = run_pipewire_input_until(
                        input_config,
                        target,
                        ring,
                        recorder,
                        stop_input,
                        input_ready_tx,
                    );
                })
                .map_err(|error| error.to_string());
            let handle = match handle_result {
                Ok(handle) => handle,
                Err(error) => {
                    input_stop.store(true, Ordering::Release);
                    for input_thread in input_threads.drain(..) {
                        let _ = input_thread.join();
                    }
                    return Err(error);
                }
            };
            match input_ready_rx.recv_timeout(Duration::from_secs(5)) {
                Ok(Ok(())) => input_threads.push(handle),
                Ok(Err(error)) => {
                    input_stop.store(true, Ordering::Release);
                    let _ = handle.join();
                    for input_thread in input_threads.drain(..) {
                        let _ = input_thread.join();
                    }
                    return Err(format!(
                        "no se pudo abrir la entrada de '{}': {error}",
                        track.name
                    ));
                }
                Err(error) => {
                    input_stop.store(true, Ordering::Release);
                    let _ = handle.join();
                    for input_thread in input_threads.drain(..) {
                        let _ = input_thread.join();
                    }
                    return Err(format!(
                        "la entrada de '{}' no respondió: {error}",
                        track.name
                    ));
                }
            }
        }
        let (schedule_tx, schedule_rx) = mpsc::channel();

        let stop = Arc::new(AtomicBool::new(false));
        let connected = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker_paused = Arc::clone(&paused);
        let worker_connected = Arc::clone(&connected);
        let worker_position = Arc::clone(&self.position_ticks);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let thread_result = thread::Builder::new()
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
                    .map_err(|_| "el scheduler MIDI terminó inesperadamente".to_owned())
                    .and_then(|result| result);
                result.and(scheduler_result)
            })
            .map_err(|error| error.to_string());
        let thread = match thread_result {
            Ok(thread) => thread,
            Err(error) => {
                input_stop.store(true, Ordering::Release);
                for input_thread in input_threads.drain(..) {
                    let _ = input_thread.join();
                }
                return Err(error);
            }
        };

        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => {
                if stop.load(Ordering::Acquire) {
                    input_stop.store(true, Ordering::Release);
                    for input_thread in input_threads.drain(..) {
                        let _ = input_thread.join();
                    }
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
                    input_stop: Arc::clone(&input_stop),
                    input_threads,
                    input_rings,
                    input_recordings,
                });
                self.plan_control = Some(Arc::clone(&control));
                if let (Some(range), Some(prepared)) = (loop_range, prepared_loop) {
                    let loop_stop = Arc::clone(&self.playback.as_ref().unwrap().stop);
                    let loop_paused = Arc::clone(&paused);
                    let loop_position = Arc::clone(&self.position_ticks);
                    let loop_metronome = Arc::clone(&self.metronome_enabled);
                    let loop_track_meters = Arc::clone(&self.track_meters);
                    let loop_input_rings = self.playback.as_ref().unwrap().input_rings.clone();
                    let loop_control = Arc::clone(&control);
                    let loop_schedule = self.playback.as_ref().unwrap().schedule.clone();
                    let loop_project = Arc::clone(&project_model);
                    let loop_active_revision = Arc::clone(&active_project_revision);
                    let loop_profile = profile;
                    let loop_plugin_state_root = self.plugin_state_root.clone();
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
                                loop_input_rings,
                                loop_active_revision,
                                loop_plugin_state_root,
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
                input_stop.store(true, Ordering::Release);
                for input_thread in input_threads.drain(..) {
                    let _ = input_thread.join();
                }
                let _ = thread.join();
                Err(error)
            }
            Err(error) => {
                stop.store(true, Ordering::Release);
                input_stop.store(true, Ordering::Release);
                for input_thread in input_threads.drain(..) {
                    let _ = input_thread.join();
                }
                let _ = thread.join();
                Err(format!("PipeWire no confirmó el stream: {error}"))
            }
        }
    }

    fn ensure_standalone_applications(&mut self, project: &Project) -> Result<(), String> {
        for track in &project.tracks {
            let Some(InstrumentConfig::Standalone {
                application_path,
                wine_prefix,
                ..
            }) = track.instrument.as_ref()
            else {
                continue;
            };
            self.open_standalone_application(&track.id, application_path, wine_prefix.as_deref())
                .map_err(|error| {
                    format!(
                        "no se pudo abrir el instrumento standalone de '{}': {error}",
                        track.name
                    )
                })?;
        }
        Ok(())
    }

    pub fn open_standalone_application(
        &mut self,
        track_id: &str,
        application_path: &str,
        wine_prefix: Option<&str>,
    ) -> Result<(), String> {
        if let Some(process) = self.standalone_processes.get_mut(track_id) {
            match process.child.try_wait() {
                Ok(None)
                    if process.application_path == application_path
                        && process.wine_prefix.as_deref() == wine_prefix =>
                {
                    return Ok(())
                }
                Ok(None) => {
                    return Err(
                        "la pista ya tiene abierto otro instrumento; ciérralo antes de cambiar su ejecutable o prefijo Wine".into(),
                    )
                }
                Ok(Some(_)) => {}
                Err(error) => {
                    return Err(format!(
                        "no se pudo comprobar el proceso abierto del instrumento: {error}"
                    ))
                }
            }
        }
        self.standalone_processes.remove(track_id);
        if process_has_executable(application_path, wine_prefix) {
            return Ok(());
        }
        let mut command = if application_path.to_ascii_lowercase().ends_with(".exe") {
            let mut command = Command::new("wine");
            command.arg(application_path);
            command
        } else {
            Command::new(application_path)
        };
        if let Some(prefix) = wine_prefix {
            command.env("WINEPREFIX", prefix);
        }
        let child = command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| error.to_string())?;
        self.standalone_processes.insert(
            track_id.to_owned(),
            ManagedStandaloneProcess {
                application_path: application_path.to_owned(),
                wine_prefix: wine_prefix.map(str::to_owned),
                child,
            },
        );
        Ok(())
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
            playback.input_rings.clone(),
            self.plugin_state_root.clone(),
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

    pub fn stop(&mut self) -> Result<Vec<FinishedInputRecording>, String> {
        let Some(playback) = self.playback.take() else {
            self.position_ticks.store(0, Ordering::Release);
            self.plugin_state_root = None;
            return Ok(Vec::new());
        };
        playback.stop.store(true, Ordering::Release);
        playback.input_stop.store(true, Ordering::Release);
        playback.connected.store(false, Ordering::Release);
        if let Some(loop_thread) = playback.loop_thread {
            let _ = loop_thread.join();
        }
        let thread_result = playback
            .thread
            .join()
            .map_err(|_| "el hilo de reproducción terminó inesperadamente".to_owned())
            .and_then(|result| result);
        for input_thread in playback.input_threads {
            let _ = input_thread.join();
        }
        let mut finished_recordings = Vec::with_capacity(playback.input_recordings.len());
        for recording in playback.input_recordings {
            let recorder = Arc::try_unwrap(recording.recorder)
                .map_err(|_| "el stream de entrada conserva una grabadora activa".to_owned())?;
            let report = recorder.finish().map_err(|error| error.to_string())?;
            finished_recordings.push(FinishedInputRecording {
                track_id: recording.track_id,
                track_name: recording.track_name,
                start_tick: recording.start_tick,
                channel_selection: recording.channel_selection,
                path: recording.path,
                sample_rate_hz: PipeWireStreamConfig::default().sample_rate,
                frames: report.captured_samples / 2,
                dropped_samples: report.dropped_samples,
            });
        }
        self.position_ticks.store(0, Ordering::Release);
        self.plan_control = None;
        self.plugin_state_root = None;
        thread_result?;
        Ok(finished_recordings)
    }
}

fn track_is_audible(track: &estudio_daw_project_model::Track, included_by_solo: bool) -> bool {
    track.mixer.active && included_by_solo && (!track.mixer.mute || track.mixer.solo)
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

fn playback_node_for_key(device_key: &str) -> Result<Option<String>, String> {
    if device_key == "pipewire:default" {
        return Ok(preferred_playback_node());
    }
    let Some(node) = device_key.strip_prefix("pipewire:") else {
        return Err(format!("backend de audio no compatible: {device_key}"));
    };
    let node = audio_devices()
        .map_err(|error| format!("no se pudieron consultar las salidas PipeWire: {error}"))?
        .into_iter()
        .find(|device| {
            device.name == node && device.media_class.to_ascii_lowercase().contains("sink")
        })
        .map(|device| device.name)
        .ok_or_else(|| format!("la salida PipeWire seleccionada ya no está disponible: {node}"))?;
    Ok(Some(node))
}

fn preferred_playback_node() -> Option<String> {
    audio_devices()
        .ok()?
        .into_iter()
        .find(is_audiobox_sink)
        .map(|device| device.name)
}

fn process_has_executable(path: &str, wine_prefix: Option<&str>) -> bool {
    let application_path = PathBuf::from(path);
    let Some(file_name) = application_path.file_name() else {
        return false;
    };
    let file_name = file_name.to_string_lossy().to_ascii_lowercase();
    let expected_prefix = wine_prefix
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".wine")));
    std::fs::read_dir("/proc")
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .bytes()
                .all(|byte| byte.is_ascii_digit())
        })
        .any(|entry| {
            let process_path = entry.path();
            let Some(cmdline) = std::fs::read(process_path.join("cmdline")).ok() else {
                return false;
            };
            let cmdline = String::from_utf8_lossy(&cmdline).to_ascii_lowercase();
            if !cmdline.contains(&file_name) {
                return false;
            }
            let Some(expected_prefix) = expected_prefix.as_ref() else {
                return wine_prefix.is_none();
            };
            let environment = std::fs::read(process_path.join("environ")).ok();
            let process_prefix = environment.as_deref().and_then(|environment| {
                environment
                    .split(|byte| *byte == 0)
                    .find_map(|variable| variable.strip_prefix(b"WINEPREFIX="))
            });
            match process_prefix {
                Some(prefix) => {
                    String::from_utf8_lossy(prefix).as_ref()
                        == expected_prefix.to_string_lossy().as_ref()
                }
                None => wine_prefix.is_none(),
            }
        })
}

fn pipewire_ports(direction: &str) -> Result<Vec<String>, String> {
    let output = Command::new("pw-link")
        .arg(direction)
        .output()
        .map_err(|error| format!("no se pudo consultar PipeWire con pw-link: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|port| !port.is_empty())
        .map(str::to_owned)
        .collect())
}

fn connect_pipewire_midi_output(source_client_name: &str, destination: &str) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let outputs = pipewire_ports("-o")?;
        let inputs = pipewire_ports("-i")?;
        let source = outputs
            .iter()
            .find(|port| port.contains(source_client_name) && port.contains("track-output"));
        if let (Some(source), true) = (source, inputs.iter().any(|port| port == destination)) {
            let output = Command::new("pw-link")
                .arg(source)
                .arg(destination)
                .output()
                .map_err(|error| format!("no se pudo crear el enlace MIDI de PipeWire: {error}"))?;
            if output.status.success() {
                return Ok(());
            }
            let error = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            return Err(if error.is_empty() {
                format!("pw-link rechazó el enlace {source} → {destination}")
            } else {
                error
            });
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "no apareció el puerto MIDI de Estudio DAW o el destino seleccionado '{destination}'"
            ));
        }
        thread::sleep(Duration::from_millis(100));
    }
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
        HashMap::new(),
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

#[allow(clippy::too_many_arguments)]
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
    mut input_rings: HashMap<String, Arc<SampleRingBuffer>>,
    plugin_state_root: Option<PathBuf>,
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
    let (output_indices, route_order, master_index) = track_output_topology(project)?;
    let route_audibility = route_audibility(project, &output_indices);
    let mut sources_by_track: HashMap<String, Vec<Box<dyn AudioNode>>> = HashMap::new();
    let mut global_sources: Vec<Box<dyn AudioNode>> = Vec::new();
    global_sources.push(Box::new(MetronomeNode::new(
        metronome_enabled,
        sample_rate,
        bpm,
        &project.transport.time_signature,
        start_position_ticks,
    )));
    let mut workers = Vec::new();
    let mut vst3_workers = Vec::new();
    let mut senders = Vec::new();
    let mut schedule = Vec::new();
    let mut sequence = 0_u64;

    for (track_index, track) in project.tracks.iter().enumerate() {
        if !matches!(&track.kind, TrackKind::Midi) {
            continue;
        }
        if !track_is_audible(track, route_audibility[track_index]) {
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
                sources_by_track
                    .entry(track.id.clone())
                    .or_default()
                    .push(Box::new(node));
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
                sources_by_track
                    .entry(track.id.clone())
                    .or_default()
                    .push(Box::new(node));
                workers.push(worker);
            }
            InstrumentConfig::Vst3 { plugin, state } => {
                let (worker, node) = Vst3InstrumentWorker::start(
                    plugin,
                    state,
                    plugin_state_root.clone(),
                    sample_rate,
                    (max_samples / 2).max(1),
                    queue_target_frames,
                    Arc::clone(&paused),
                )
                .map_err(|error| {
                    format!(
                        "no se pudo preparar el instrumento VST3 '{}': {error}; no se sustituirá por otro instrumento",
                        track.name
                    )
                })?;
                senders.push(EventSender::Vst3 {
                    track_id: track.id.clone(),
                    sender: worker.command_sender(),
                });
                sources_by_track
                    .entry(track.id.clone())
                    .or_default()
                    .push(Box::new(node));
                vst3_workers.push(worker);
            }
            InstrumentConfig::Standalone { midi_output, .. } => {
                let route = midi_output.ok_or_else(|| {
                    format!(
                        "la pista '{}' tiene un instrumento standalone asignado, pero falta elegir su entrada MIDI",
                        track.name
                    )
                })?;
                let safe_track_id: String = track
                    .id
                    .chars()
                    .map(|character| {
                        if character.is_ascii_alphanumeric() || character == '-' {
                            character
                        } else {
                            '_'
                        }
                    })
                    .collect();
                let safe_track_id = safe_track_id.chars().take(16).collect::<String>();
                let client_name = format!(
                    "Estudio DAW External MIDI {safe_track_id} {}",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos()
                );
                let worker = LiveMidiOutputWorker::start(&client_name).map_err(|error| {
                    format!("no se pudo conectar el MIDI de '{}': {error}", track.name)
                })?;
                connect_pipewire_midi_output(&client_name, &route.port_name).map_err(|error| {
                    format!(
                        "no se pudo enlazar el MIDI de '{}' con Analog Lab: {error}",
                        track.name
                    )
                })?;
                senders.push(EventSender::Alsa {
                    worker,
                    channel: route.channel,
                });
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
        let Some(track_index) = project
            .tracks
            .iter()
            .position(|candidate| candidate.id == track.id)
        else {
            continue;
        };
        if !track_is_audible(track, route_audibility[track_index]) {
            continue;
        }
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
                gain_left: 10.0_f32.powf(clip.gain_db / 20.0),
                gain_right: 10.0_f32.powf(clip.gain_db / 20.0),
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
    builder.add_node(ProjectRoutingNode::new(
        project,
        sources_by_track,
        audio_streams,
        global_sources,
        max_samples,
        &route_audibility,
        output_indices,
        route_order,
        master_index,
        &mut input_rings,
        &track_meters,
    )?);
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
    builder.add_node(MasterOutputMeterNode {
        meter: track_meter_for(&track_meters, MASTER_METER_ID)?,
    });
    let mut plan = builder.build();
    for worker in workers {
        plan.retain_resource(worker);
    }
    for worker in vst3_workers {
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
    input_rings: HashMap<String, Arc<SampleRingBuffer>>,
    active_project_revision: Arc<AtomicU64>,
    plugin_state_root: Option<PathBuf>,
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
                input_rings.clone(),
                plugin_state_root.clone(),
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
                input_rings.clone(),
                plugin_state_root.clone(),
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
            input_rings.clone(),
            plugin_state_root.clone(),
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
    input_rings: HashMap<String, Arc<SampleRingBuffer>>,
    plugin_state_root: Option<PathBuf>,
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
            input_rings.clone(),
            plugin_state_root.clone(),
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
) -> Result<(), String> {
    let sort_schedule = |schedule: &mut PlaybackSchedule| {
        schedule
            .events
            .sort_by_key(|event| (event.at_tick, event.sequence));
    };
    sort_schedule(&mut schedule);
    let mut index = 0;
    loop {
        if stop.load(Ordering::Acquire) {
            return Ok(());
        }
        if let Ok(command) = updates.try_recv() {
            apply_scheduler_command(command, &mut schedule, &mut index, &sort_schedule);
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
            if let Err(error) = sender.send(event.midi) {
                stop.store(true, Ordering::Release);
                return Err(format!("falló la salida MIDI de la pista: {error}"));
            }
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
        SchedulerCommand::Vst3Editor {
            track_id,
            open,
            reply,
        } => {
            let sender = schedule.senders.iter_mut().find(|sender| {
                matches!(sender, EventSender::Vst3 { track_id: id, .. } if id == &track_id)
            });
            let result = sender
                .ok_or_else(|| {
                    "la pista seleccionada no tiene un instrumento VST3 activo".to_owned()
                })
                .and_then(|sender| sender.request_vst3_editor(open));
            match result {
                Ok(result) => {
                    thread::spawn(move || {
                        let result =
                            result
                                .recv_timeout(Duration::from_secs(32))
                                .unwrap_or_else(|error| {
                                    Err(format!(
                                        "el helper VST3 no respondió al control GUI: {error}"
                                    ))
                                });
                        let _ = reply.send(result);
                    });
                }
                Err(error) => {
                    let _ = reply.send(Err(error));
                }
            }
        }
        SchedulerCommand::Vst3SaveState { track_id, reply } => {
            let sender = schedule.senders.iter_mut().find(|sender| {
                matches!(sender, EventSender::Vst3 { track_id: id, .. } if id == &track_id)
            });
            let result = sender
                .ok_or_else(|| {
                    "la pista seleccionada no tiene un instrumento VST3 activo".to_owned()
                })
                .and_then(EventSender::request_vst3_state);
            match result {
                Ok(result) => {
                    thread::spawn(move || {
                        let result =
                            result
                                .recv_timeout(Duration::from_secs(32))
                                .unwrap_or_else(|error| {
                                    Err(format!("el helper VST3 no entregó el estado: {error}"))
                                });
                        let _ = reply.send(result);
                    });
                }
                Err(error) => {
                    let _ = reply.send(Err(error));
                }
            }
        }
    }
}

fn send_all_notes_off(senders: &mut [EventSender]) {
    for sender in senders {
        for channel in 0..16 {
            let _ = sender.send(SynthMidiEvent::ControlChange {
                channel,
                controller: 64,
                value: 0,
            });
            let _ = sender.send(SynthMidiEvent::ControlChange {
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

    #[test]
    #[ignore = "requiere ruta e ID de clase de un instrumento VST3 instalado localmente"]
    fn vst3_worker_returns_real_plugin_audio_outside_the_render_callback() {
        let path = std::env::var("ESTUDIO_DAW_TEST_VST3_PATH")
            .expect("define la ruta al bundle VST3 local");
        let unique_id =
            std::env::var("ESTUDIO_DAW_TEST_VST3_UID").expect("define el ID de clase VST3 local");
        let (worker, mut node) = Vst3InstrumentWorker::start(
            estudio_daw_project_model::PluginReference {
                format: "vst3".into(),
                path,
                unique_id,
                bridge: Some("yabridge".into()),
            },
            None,
            None,
            48_000,
            512,
            1_024,
            Arc::new(AtomicBool::new(false)),
        )
        .expect("Analog Lab V debe cargar en el helper aislado");
        worker
            .command_sender()
            .send(Vst3InstrumentCommand::Midi(SynthMidiEvent::NoteOn {
                channel: 0,
                note: 60,
                velocity: 100,
            }))
            .unwrap();

        let mut peak = 0.0_f32;
        for _ in 0..12 {
            thread::sleep(Duration::from_millis(5));
            let mut block = vec![0.0_f32; 1_024];
            node.process(&mut block).unwrap();
            peak = peak.max(block.iter().map(|sample| sample.abs()).fold(0.0, f32::max));
        }
        assert!(peak > 0.0001, "el VST3 produjo sólo silencio: peak={peak}");
        worker
            .command_sender()
            .send(Vst3InstrumentCommand::Midi(SynthMidiEvent::NoteOff {
                channel: 0,
                note: 60,
            }))
            .unwrap();
    }

    fn midi_track(id: &str) -> Track {
        Track {
            id: id.into(),
            name: id.into(),
            kind: TrackKind::Midi,
            role: TrackRole::Instrument,
            output_track_id: None,
            input_route: None,
            record_armed: false,
            channel_config: TrackChannelConfig::default(),
            color: "#58a6b8".into(),
            marker: String::new(),
            annotation: String::new(),
            group_name: None,
            mixer: TrackMixerState::default(),
            notes: Vec::new(),
            audio_channels: None,
            media_source: None,
            instrument: Some(InstrumentConfig::Sine),
        }
    }

    #[test]
    fn standalone_instrument_audio_return_uses_track_input_routing() {
        let mut track = midi_track("analog-lab");
        let channels = vec![0, 1];
        track.instrument = Some(InstrumentConfig::Standalone {
            application_path: "/wine/Analog Lab V.exe".into(),
            wine_prefix: Some("/wine-prefix".into()),
            midi_output: None,
            audio_input: Some(estudio_daw_project_model::ExternalAudioPort {
                node_key: "pipewire:AnalogLab-KeyLab-Test".into(),
                channels: channels.clone(),
            }),
        });
        let (node, selected_channels) = track_audio_input(&track).unwrap();
        assert_eq!(node, "pipewire:AnalogLab-KeyLab-Test");
        assert_eq!(selected_channels, channels);
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

        // The global metronome node remains in the graph while its atomic toggle is off.
        assert_eq!(plan.node_count(), 3);
        assert_eq!(senders.len(), 2);
        assert_eq!(schedule.len(), 4);
        assert_eq!(schedule[0].at, Duration::ZERO);
        assert_eq!(schedule[1].at, Duration::from_millis(500));
        assert_eq!(schedule[2].at, Duration::from_millis(500));
        assert_eq!(schedule[3].at, Duration::from_secs(1));
        assert_ne!(schedule[1].sender, schedule[2].sender);
    }

    #[test]
    fn does_not_silently_replace_external_plugin_with_builtin_synth() {
        let mut project = project_with_two_clips();
        project.tracks[0].instrument = Some(InstrumentConfig::Vst3 {
            plugin: estudio_daw_project_model::PluginReference {
                format: "vst3".into(),
                path: "/plugins/Analog Lab V.vst3".into(),
                unique_id: "Arturia.AnalogLabV".into(),
                bridge: Some("yabridge".into()),
            },
            state: None,
        });

        let result = build_project_playback(
            &project,
            48_000,
            512,
            1024,
            Arc::new(AtomicBool::new(true)),
            Arc::new(AtomicU64::new(0)),
            0,
        );
        let error = match result {
            Err(error) => error,
            Ok(_) => panic!("un VST externo no debe sustituirse por un instrumento integrado"),
        };
        assert!(error.contains("no se sustituirá por otro instrumento"));
    }

    #[test]
    fn writes_and_restores_plugin_state_relative_to_the_project() {
        let root = std::env::temp_dir().join(format!(
            "estudio-daw-plugin-state-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let saved = write_plugin_state_file(&root, "track/midi 1", b"preset-bytes").unwrap();
        assert_eq!(saved.path, "plugin-state/track_midi_1.bin");
        assert!(saved
            .sha256
            .as_deref()
            .is_some_and(|digest| digest.starts_with("sha256:")));
        assert_eq!(
            load_plugin_state_bytes(&root, &saved).unwrap(),
            b"preset-bytes"
        );
        let missing = load_plugin_state_bytes(
            &root,
            &PluginStateReference {
                path: "plugin-state/ausente.bin".into(),
                sha256: None,
            },
        )
        .unwrap_err();
        assert!(missing.contains("se conservó el instrumento asignado"));
        let _ = std::fs::remove_dir_all(root);
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
