//! Adaptador Tauri de escritorio.
//!
//! Esta capa traduce llamadas de la UI a la API de aplicación. No mueve audio
//! ni buffers de GPU por IPC y no accede al motor RT desde los comandos de UI.

mod audio_runtime;

use estudio_daw_application::{
    load_audio_runtime_settings, save_audio_runtime_settings, AudioRuntimeSettings,
    AudioRuntimeView, CommandAuthor, DomainCommand, ProjectApplication, ProjectCommand,
    SessionCommand, TransportState,
};
use estudio_daw_midi_engine::{MidiSource, MidiTake, RecordedMidiEvent, RecordedMidiMessage};
use estudio_daw_project_model::{
    ClipLaunchMode, ClipLaunchQuantization, ClipReference, ClipSlot, ImportProvenance,
    InstrumentConfig, MidiClip, Project, Scene, TimeSignature, Track, TrackChannelConfig,
    TrackInputRoute, TrackKind, TrackMixerState, TrackRole, Transport, TransportLoopRange,
};
use estudio_daw_runtime_diagnostics::audio_devices;
use serde::Serialize;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::State;

use audio_runtime::{AudioRuntimeHost, SessionLaunchView};

struct DesktopState {
    // ProjectApplication conserva su runtime/historial; el lock sólo se toma
    // desde comandos de UI, nunca desde el callback de audio.
    application: Mutex<Option<ProjectApplication>>,
    audio: Mutex<AudioRuntimeHost>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TrackSummary {
    id: String,
    name: String,
    kind: &'static str,
    role: &'static str,
    note_count: usize,
    instrument: Option<InstrumentConfig>,
    color: String,
    marker: String,
    annotation: String,
    input_channels: Option<u32>,
    input_route: Option<TrackInputRoute>,
    record_armed: bool,
    output_channels: u32,
    output_track_id: Option<String>,
    group_name: Option<String>,
    active: bool,
    mute: bool,
    solo: bool,
    gain_db: f32,
    pan: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TrackMeterSummary {
    peak: f32,
    rms: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AudioOutputDeviceSummary {
    key: String,
    name: String,
    description: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MidiOutputDeviceSummary {
    key: String,
    label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MidiClipSummary {
    id: String,
    name: String,
    track_id: String,
    ppq: u32,
    start_tick: u64,
    duration_ticks: u64,
    start_beats: f64,
    duration_beats: f64,
    note_count: usize,
    notes: Vec<MidiNoteSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AudioClipSummary {
    id: String,
    name: String,
    track_id: String,
    source_id: Option<String>,
    start_beats: f64,
    duration_beats: f64,
    source_start_samples: u64,
    duration_samples: u64,
    sample_rate_hz: u32,
    channels: u16,
    source_name: Option<String>,
    source_digest: Option<String>,
    gain_db: f32,
    fade_in_samples: u64,
    fade_out_samples: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SceneSummary {
    id: String,
    name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClipSlotSummary {
    id: String,
    scene_id: String,
    track_id: String,
    clip_kind: Option<&'static str>,
    clip_id: Option<String>,
    launch_quantization: ClipLaunchQuantization,
    launch_mode: ClipLaunchMode,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AudioImportMetadata {
    sample_rate_hz: u32,
    channels: u16,
    duration_seconds: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MidiNoteSummary {
    note_on_index: usize,
    note_off_index: usize,
    channel: u8,
    start_tick: u64,
    end_tick: u64,
    start_beats: f64,
    duration_beats: f64,
    key: u8,
    velocity: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiSnapshot {
    schema_version: &'static str,
    project_id: String,
    project_revision: u64,
    project_path: Option<String>,
    tempo_bpm: f64,
    transport_state: &'static str,
    loop_range: Option<TransportLoopRange>,
    track_count: usize,
    midi_clip_count: usize,
    audio_clip_count: usize,
    beats_per_bar: f64,
    tracks: Vec<TrackSummary>,
    midi_clips: Vec<MidiClipSummary>,
    audio_clips: Vec<AudioClipSummary>,
    scenes: Vec<SceneSummary>,
    clip_slots: Vec<ClipSlotSummary>,
    can_undo: bool,
    can_redo: bool,
    audio_engine_connected: bool,
}

/// Medición agregada para UI. `sequence` permite descartar mensajes atrasados;
/// `at_sample` mantiene la sincronía con el timeline sin enviar PCM.
#[allow(dead_code)] // Contrato preparado antes de conectar los medidores live.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MeterFrameV1 {
    schema_version: &'static str,
    sequence: u64,
    at_sample: u64,
    sample_rate_hz: u32,
    window_frames: u32,
    meters: Vec<MeterReadingV1>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MeterReadingV1 {
    channel_id: String,
    peak_dbfs: f32,
    rms_dbfs: f32,
    clipping: bool,
}

/// Referencia a un derivado visual, nunca una ruta local ni un handle de GPU.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Consumido por el endpoint de artefactos en la fase visual.
struct VisualizationArtifactRefV1 {
    schema_version: &'static str,
    artifact_id: String,
    kind: &'static str,
    project_revision: u64,
    source_digest: String,
    sample_rate_hz: u32,
    channel_count: u16,
    frame_count: u64,
    level_count: u8,
}

/// Una consulta de waveform retorna pares min/max ya reducidos en el backend.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Consumido por la lectura paginada de waveform.
struct WaveformChunkV1 {
    schema_version: &'static str,
    artifact_id: String,
    level: u8,
    first_bin: u64,
    bins: Vec<WaveformBinV1>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Payload pequeño de bins, separado del PCM original.
struct WaveformBinV1 {
    min: f32,
    max: f32,
}

/// Tiles de espectrograma se consultan como artefactos binarios acotados por
/// rango; no se serializan como PCM ni como un buffer de textura compartido.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Consumido por el protocolo de tiles de espectrograma.
struct SpectrogramTileRefV1 {
    schema_version: &'static str,
    artifact_id: String,
    tile_x: u32,
    tile_y: u16,
    width: u16,
    height: u16,
    encoding: &'static str,
}

fn summarize(application: &ProjectApplication, audio_engine_connected: bool) -> UiSnapshot {
    let domain = application.snapshot();
    let project: Project = domain.project.project;
    let beats_per_bar = f64::from(project.transport.time_signature.numerator) * 4.0
        / f64::from(project.transport.time_signature.denominator.max(1));
    let (can_undo, can_redo) = application.history_state();
    let tracks = project
        .tracks
        .iter()
        .map(|track| TrackSummary {
            id: track.id.clone(),
            name: track.name.clone(),
            kind: match &track.kind {
                TrackKind::Audio => "audio",
                TrackKind::Midi => "midi",
            },
            role: match track.role {
                TrackRole::Midi => "midi",
                TrackRole::Instrument => "instrument",
                TrackRole::Audio => "audio",
                TrackRole::Bus => "bus",
                TrackRole::Return => "return",
                TrackRole::Master => "master",
            },
            note_count: track.notes.len()
                + project
                    .midi_clips
                    .iter()
                    .filter(|clip| clip.track_id == track.id)
                    .flat_map(|clip| &clip.take.events)
                    .filter(|event| matches!(&event.message, RecordedMidiMessage::NoteOn { velocity, .. } if *velocity > 0))
                    .count(),
            instrument: track.instrument.clone(),
            color: track.color.clone(),
            marker: track.marker.clone(),
            annotation: track.annotation.clone(),
            input_channels: track.channel_config.input_channels,
            input_route: track.input_route.clone(),
            record_armed: track.record_armed,
            output_channels: track.channel_config.output_channels,
            output_track_id: track.output_track_id.clone(),
            group_name: track.group_name.clone(),
            active: track.mixer.active,
            mute: track.mixer.mute,
            solo: track.mixer.solo,
            gain_db: track.mixer.gain_db,
            pan: track.mixer.pan,
        })
        .collect();
    let midi_clips = project
        .midi_clips
        .iter()
        .map(|clip| {
            let ppq = f64::from(clip.take.ppq.max(1));
            MidiClipSummary {
                id: clip.id.clone(),
                name: clip.name.clone(),
                track_id: clip.track_id.clone(),
                ppq: clip.take.ppq,
                start_tick: clip.start_tick,
                duration_ticks: clip.duration_ticks,
                start_beats: clip.start_tick as f64 / ppq,
                duration_beats: clip.duration_ticks as f64 / ppq,
                note_count: clip
                    .take
                    .events
                    .iter()
                    .filter(|event| matches!(&event.message, RecordedMidiMessage::NoteOn { velocity, .. } if *velocity > 0))
                    .count(),
                notes: summarize_midi_notes(clip),
            }
        })
        .collect();
    let audio_clips = project
        .audio_clips
        .iter()
        .map(|clip| {
            let source = clip
                .source_id
                .as_ref()
                .and_then(|id| project.audio_sources.iter().find(|source| &source.id == id));
            AudioClipSummary {
                id: clip.id.clone(),
                name: clip.name.clone(),
                track_id: clip.track_id.clone(),
                source_id: clip.source_id.clone(),
                start_beats: clip.start_tick as f64 / 480.0,
                duration_beats: clip.duration_samples as f64 / f64::from(clip.sample_rate.max(1))
                    * project.transport.tempo_bpm
                    / 60.0,
                source_start_samples: clip.source_start_samples,
                duration_samples: clip.duration_samples,
                sample_rate_hz: clip.sample_rate,
                channels: clip.channels,
                source_name: source
                    .and_then(|source| source.media.original_path.file_name())
                    .map(|name| name.to_string_lossy().into_owned()),
                source_digest: source.map(|source| source.media.original_hash.clone()),
                gain_db: clip.gain_db,
                fade_in_samples: clip.fade_in_samples,
                fade_out_samples: clip.fade_out_samples,
            }
        })
        .collect();
    let scenes = project
        .scenes
        .iter()
        .map(|scene| SceneSummary {
            id: scene.id.clone(),
            name: scene.name.clone(),
        })
        .collect();
    let clip_slots = project
        .clip_slots
        .iter()
        .map(|slot| {
            let (clip_kind, clip_id) = match &slot.clip {
                Some(ClipReference::Midi(id)) => (Some("midi"), Some(id.clone())),
                Some(ClipReference::Audio(id)) => (Some("audio"), Some(id.clone())),
                None => (None, None),
            };
            ClipSlotSummary {
                id: slot.id.clone(),
                scene_id: slot.scene_id.clone(),
                track_id: slot.track_id.clone(),
                clip_kind,
                clip_id,
                launch_quantization: slot.launch_quantization,
                launch_mode: slot.launch_mode,
            }
        })
        .collect();

    UiSnapshot {
        schema_version: "ui-snapshot.v1",
        project_id: project.project_id,
        project_revision: domain.project.revision,
        project_path: application
            .project_path()
            .map(|path| path.display().to_string()),
        tempo_bpm: domain.transport.tempo_bpm,
        transport_state: match domain.transport.state {
            TransportState::Stopped => "stopped",
            TransportState::Playing => "playing",
            TransportState::Paused => "paused",
        },
        loop_range: project.transport.loop_range,
        track_count: project.tracks.len(),
        midi_clip_count: project.midi_clips.len(),
        audio_clip_count: project.audio_clips.len(),
        beats_per_bar,
        tracks,
        midi_clips,
        audio_clips,
        scenes,
        clip_slots,
        can_undo,
        can_redo,
        audio_engine_connected,
    }
}

#[tauri::command]
fn edit_audio_region(
    action: String,
    clip_id: String,
    start_tick: Option<u64>,
    source_start_samples: Option<u64>,
    duration_samples: Option<u64>,
    gain_db: Option<f32>,
    fade_in_samples: Option<u64>,
    fade_out_samples: Option<u64>,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero crea o abre un proyecto".to_owned())?;
    let command = match action.as_str() {
        "move" => ProjectCommand::MoveAudioClip {
            clip_id,
            start_tick: start_tick.ok_or_else(|| "falta la posición de la región".to_owned())?,
        },
        "trim" => ProjectCommand::TrimAudioClip {
            clip_id,
            source_start_samples: source_start_samples
                .ok_or_else(|| "falta el desplazamiento de fuente".to_owned())?,
            duration_samples: duration_samples
                .ok_or_else(|| "falta la duración de la región".to_owned())?,
            start_tick,
        },
        "gain" => ProjectCommand::SetAudioClipGain {
            clip_id,
            gain_db: gain_db.ok_or_else(|| "falta la ganancia de la región".to_owned())?,
        },
        "fades" => ProjectCommand::SetAudioClipFades {
            clip_id,
            fade_in_samples: fade_in_samples
                .ok_or_else(|| "falta el desvanecimiento inicial".to_owned())?,
            fade_out_samples: fade_out_samples
                .ok_or_else(|| "falta el desvanecimiento final".to_owned())?,
        },
        "remove" => ProjectCommand::RemoveAudioClip { clip_id },
        _ => return Err(format!("acción de región desconocida: {action}")),
    };
    application
        .execute_project(command)
        .map_err(|error| error.to_string())?;
    let project = application.snapshot().project.project;
    let mut audio = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?;
    let connected = audio.is_connected();
    if connected {
        let settings = load_audio_runtime_settings().map_err(|error| error.to_string())?;
        audio
            .refresh_project(&project, settings.active())
            .map_err(|error| format!("la edición del proyecto quedó aplicada, pero no se pudo actualizar el plan de audio: {error}"))?;
    }
    Ok(summarize(application, connected))
}

#[tauri::command]
fn quantize_midi_clip(
    clip_id: String,
    grid_ticks: u64,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    if grid_ticks == 0 {
        return Err("la rejilla de cuantización debe ser mayor que cero".into());
    }
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    application
        .execute_project(ProjectCommand::QuantizeMidiClip {
            clip_id,
            grid_ticks,
        })
        .map_err(|error| error.to_string())?;
    let project = application.snapshot().project.project;
    let mut audio = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?;
    let connected = audio.is_connected();
    if connected {
        let settings = load_audio_runtime_settings().map_err(|error| error.to_string())?;
        audio
            .refresh_project(&project, settings.active())
            .map_err(|error| {
                format!("el clip se cuantizó, pero no se pudo actualizar el plan de audio: {error}")
            })?;
    }
    Ok(summarize(application, connected))
}

#[tauri::command]
fn move_midi_clip(
    clip_id: String,
    start_tick: u64,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    application
        .execute_project(ProjectCommand::MoveMidiClip {
            clip_id,
            start_tick,
        })
        .map_err(|error| error.to_string())?;
    let project = application.snapshot().project.project;
    let mut audio = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?;
    let connected = audio.is_connected();
    if connected {
        let settings = load_audio_runtime_settings().map_err(|error| error.to_string())?;
        audio
            .refresh_project(&project, settings.active())
            .map_err(|error| {
                format!("el clip se movió, pero no se pudo actualizar el plan de audio: {error}")
            })?;
    }
    Ok(summarize(application, connected))
}

#[tauri::command]
fn duplicate_midi_clip(
    clip_id: String,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    application
        .execute_project(ProjectCommand::DuplicateMidiClip { clip_id })
        .map_err(|error| error.to_string())?;
    let project = application.snapshot().project.project;
    let mut audio = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?;
    let connected = audio.is_connected();
    if connected {
        let settings = load_audio_runtime_settings().map_err(|error| error.to_string())?;
        audio
            .refresh_project(&project, settings.active())
            .map_err(|error| {
                format!("el clip se duplicó, pero no se pudo actualizar el plan de audio: {error}")
            })?;
    }
    Ok(summarize(application, connected))
}

#[tauri::command]
fn split_midi_clip(
    clip_id: String,
    split_tick: u64,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    application
        .execute_project(ProjectCommand::SplitMidiClip {
            clip_id,
            split_tick,
        })
        .map_err(|error| error.to_string())?;
    let project = application.snapshot().project.project;
    let mut audio = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?;
    let connected = audio.is_connected();
    if connected {
        let settings = load_audio_runtime_settings().map_err(|error| error.to_string())?;
        audio
            .refresh_project(&project, settings.active())
            .map_err(|error| {
                format!("el clip se dividió, pero no se pudo actualizar el plan de audio: {error}")
            })?;
    }
    Ok(summarize(application, connected))
}

#[tauri::command]
fn add_midi_note(
    clip_id: String,
    start_tick: u64,
    duration_ticks: u64,
    key: u8,
    velocity: u8,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    application
        .execute_project(ProjectCommand::AddMidiNote {
            clip_id,
            start_tick,
            duration_ticks,
            key,
            velocity,
        })
        .map_err(|error| error.to_string())?;
    let project = application.snapshot().project.project;
    let mut audio = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?;
    let connected = audio.is_connected();
    if connected {
        let settings = load_audio_runtime_settings().map_err(|error| error.to_string())?;
        audio
            .refresh_project(&project, settings.active())
            .map_err(|error| {
                format!("la nota se añadió, pero no se pudo actualizar el plan de audio: {error}")
            })?;
    }
    Ok(summarize(application, connected))
}

#[tauri::command]
fn update_midi_note(
    clip_id: String,
    note_on_index: usize,
    note_off_index: usize,
    expected_start_tick: u64,
    expected_end_tick: u64,
    expected_key: u8,
    expected_channel: u8,
    start_tick: u64,
    duration_ticks: u64,
    key: u8,
    velocity: u8,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    application
        .execute_project(ProjectCommand::UpdateMidiNote {
            clip_id,
            note_on_index,
            note_off_index,
            expected_start_tick,
            expected_end_tick,
            expected_key,
            expected_channel,
            start_tick,
            duration_ticks,
            key,
            velocity,
        })
        .map_err(|error| error.to_string())?;
    let project = application.snapshot().project.project;
    let mut audio = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?;
    let connected = audio.is_connected();
    if connected {
        let settings = load_audio_runtime_settings().map_err(|error| error.to_string())?;
        audio
            .refresh_project(&project, settings.active())
            .map_err(|error| {
                format!("la nota se editó, pero no se pudo actualizar el plan de audio: {error}")
            })?;
    }
    Ok(summarize(application, connected))
}

#[tauri::command]
fn remove_midi_note(
    clip_id: String,
    note_on_index: usize,
    note_off_index: usize,
    expected_start_tick: u64,
    expected_end_tick: u64,
    expected_key: u8,
    expected_channel: u8,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    application
        .execute_project(ProjectCommand::RemoveMidiNote {
            clip_id,
            note_on_index,
            note_off_index,
            expected_start_tick,
            expected_end_tick,
            expected_key,
            expected_channel,
        })
        .map_err(|error| error.to_string())?;
    let project = application.snapshot().project.project;
    let mut audio = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?;
    let connected = audio.is_connected();
    if connected {
        let settings = load_audio_runtime_settings().map_err(|error| error.to_string())?;
        audio
            .refresh_project(&project, settings.active())
            .map_err(|error| {
                format!("la nota se borró, pero no se pudo actualizar el plan de audio: {error}")
            })?;
    }
    Ok(summarize(application, connected))
}

fn summarize_midi_notes(clip: &MidiClip) -> Vec<MidiNoteSummary> {
    let ppq = f64::from(clip.take.ppq.max(1));
    let mut active_notes = Vec::<(u8, u8, u64, u8, usize)>::new();
    let mut notes = Vec::new();
    for (event_index, event) in clip.take.events.iter().enumerate() {
        match &event.message {
            RecordedMidiMessage::NoteOn {
                channel,
                note,
                velocity,
            } if *velocity > 0 => active_notes.push((
                *channel,
                *note,
                event.tick,
                (*velocity).clamp(1, 127) as u8,
                event_index,
            )),
            RecordedMidiMessage::NoteOff { channel, note, .. }
            | RecordedMidiMessage::NoteOn { channel, note, .. } => {
                if let Some(index) =
                    active_notes
                        .iter()
                        .rposition(|(active_channel, active_note, _, _, _)| {
                            active_channel == channel && active_note == note
                        })
                {
                    let (channel, key, start_tick, velocity, note_on_index) =
                        active_notes.remove(index);
                    notes.push(MidiNoteSummary {
                        note_on_index,
                        note_off_index: event_index,
                        channel,
                        start_tick,
                        end_tick: event.tick,
                        start_beats: start_tick as f64 / ppq,
                        duration_beats: event.tick.saturating_sub(start_tick) as f64 / ppq,
                        key,
                        velocity,
                    });
                }
            }
            _ => {}
        }
    }
    notes.sort_by(|left, right| left.start_beats.total_cmp(&right.start_beats));
    notes
}

#[tauri::command]
fn new_project(state: State<'_, DesktopState>) -> Result<UiSnapshot, String> {
    replace_application(state, ProjectApplication::new(new_project_model()))
}

#[tauri::command]
fn add_track(kind: String, state: State<'_, DesktopState>) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero crea o abre un proyecto".to_owned())?;
    let (track_kind, name, instrument, role) = match kind.as_str() {
        "audio" => (TrackKind::Audio, "Audio", None, TrackRole::Audio),
        "bus" => (TrackKind::Audio, "Bus", None, TrackRole::Bus),
        "midi" => (
            TrackKind::Midi,
            "MIDI",
            Some(estudio_daw_project_model::InstrumentConfig::Sine),
            TrackRole::Instrument,
        ),
        _ => return Err(format!("tipo de pista desconocido: {kind}")),
    };
    let current = application.snapshot().project.project;
    let index = current.tracks.len();
    let same_kind_count = current
        .tracks
        .iter()
        .filter(|track| match &track.kind {
            TrackKind::Audio => track.role == role,
            TrackKind::Midi => track.role == TrackRole::Midi || track.role == TrackRole::Instrument,
        })
        .count();
    let track = estudio_daw_project_model::Track {
        id: format!("track-{}-{index}", unix_timestamp_millis()),
        name: format!("{name} {}", same_kind_count + 1),
        kind: track_kind,
        role,
        output_track_id: None,
        input_route: None,
        record_armed: false,
        channel_config: TrackChannelConfig {
            input_channels: if role == TrackRole::Audio {
                Some(2)
            } else {
                None
            },
            output_channels: 2,
        },
        color: "#58a6b8".into(),
        marker: String::new(),
        annotation: String::new(),
        group_name: None,
        mixer: TrackMixerState::default(),
        notes: Vec::new(),
        audio_channels: if role == TrackRole::Audio {
            Some(2)
        } else {
            None
        },
        media_source: None,
        instrument,
    };
    application
        .execute_project(ProjectCommand::AddTrack { track, index: None })
        .map_err(|error| error.to_string())?;
    let connected = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .is_connected();
    Ok(summarize(application, connected))
}

#[tauri::command]
fn remove_track(track_id: String, state: State<'_, DesktopState>) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero crea o abre un proyecto".to_owned())?;
    application
        .execute_project(ProjectCommand::RemoveTrack { track_id })
        .map_err(|error| error.to_string())?;
    let project = application.snapshot().project.project;
    let mut audio = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?;
    let connected = audio.is_connected();
    if connected {
        let settings = load_audio_runtime_settings().map_err(|error| error.to_string())?;
        audio
            .refresh_project(&project, settings.active())
            .map_err(|error| {
                format!("la pista se quitó, pero no se pudo actualizar el audio: {error}")
            })?;
    }
    Ok(summarize(application, connected))
}

#[tauri::command]
fn duplicate_track(track_id: String, state: State<'_, DesktopState>) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero crea o abre un proyecto".to_owned())?;
    let snapshot = application.snapshot();
    let tracks = &snapshot.project.project.tracks;
    let source = tracks
        .iter()
        .find(|track| track.id == track_id)
        .ok_or_else(|| format!("no existe la pista {track_id}"))?;
    let index = tracks
        .iter()
        .position(|track| track.id == track_id)
        .unwrap_or(tracks.len())
        + 1;
    let name = format!("{} copia", source.name);
    let mut new_track_id = format!("{track_id}-copy-{}", unix_timestamp_millis());
    let mut suffix = 1_u32;
    while tracks.iter().any(|track| track.id == new_track_id) {
        new_track_id = format!("{track_id}-copy-{}-{suffix}", unix_timestamp_millis());
        suffix += 1;
    }
    application
        .execute_project(ProjectCommand::DuplicateTrack {
            track_id,
            new_track_id,
            name,
            index: Some(index),
        })
        .map_err(|error| error.to_string())?;
    let project = application.snapshot().project.project;
    let mut audio = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?;
    let connected = audio.is_connected();
    if connected {
        let settings = load_audio_runtime_settings().map_err(|error| error.to_string())?;
        audio
            .refresh_project(&project, settings.active())
            .map_err(|error| {
                format!("la pista se duplicó, pero no se pudo actualizar el audio: {error}")
            })?;
    }
    Ok(summarize(application, connected))
}

#[tauri::command]
fn move_track(
    track_id: String,
    index: usize,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero crea o abre un proyecto".to_owned())?;
    application
        .execute_project(ProjectCommand::MoveTrack { track_id, index })
        .map_err(|error| error.to_string())?;
    let project = application.snapshot().project.project;
    let mut audio = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?;
    let connected = audio.is_connected();
    if connected {
        let settings = load_audio_runtime_settings().map_err(|error| error.to_string())?;
        audio
            .refresh_project(&project, settings.active())
            .map_err(|error| {
                format!("el orden cambió, pero no se pudo actualizar el audio: {error}")
            })?;
    }
    Ok(summarize(application, connected))
}

#[tauri::command]
fn import_audio(
    path: String,
    track_id: String,
    copy_into_project: bool,
    start_tick: u64,
    source_channel_selection: Vec<u16>,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let original_path = PathBuf::from(path);
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero crea o abre un proyecto".to_owned())?;
    let project = application.snapshot().project.project;
    let track = project
        .tracks
        .iter()
        .find(|track| track.id == track_id && track.role == TrackRole::Audio)
        .ok_or_else(|| "elige una pista de audio existente".to_owned())?;
    let metadata = estudio_daw_media_adapter::inspect_audio_metadata(&original_path)
        .map_err(|error| error.to_string())?;
    let inspected_source = estudio_daw_media_adapter::inspect_media_source(&original_path)
        .map_err(|error| error.to_string())?;
    if project.audio_sources.iter().any(|source| {
        source.owner_track_id == track_id
            && source.media.original_hash == inspected_source.original_hash
    }) {
        return Err("este archivo ya está importado en la pista elegida".to_owned());
    }
    if source_channel_selection.is_empty()
        || source_channel_selection.len()
            > usize::try_from(track.channel_config.output_channels).unwrap_or(2)
        || source_channel_selection
            .iter()
            .any(|channel| *channel >= metadata.channels)
    {
        return Err(format!(
            "la selección de canales no es válida para una fuente de {} canales y una pista de {}",
            metadata.channels, track.channel_config.output_channels
        ));
    }
    let source_path = if copy_into_project {
        let project_path = application
            .project_path()
            .ok_or_else(|| "guarda el proyecto antes de copiar medios a su carpeta".to_owned())?;
        let media_dir = project_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("media");
        std::fs::create_dir_all(&media_dir).map_err(|error| error.to_string())?;
        let file_name = original_path
            .file_name()
            .ok_or_else(|| "la ruta seleccionada no tiene nombre de archivo".to_owned())?;
        let short_hash = inspected_source
            .original_hash
            .strip_prefix("sha256:")
            .unwrap_or(&inspected_source.original_hash);
        let destination = media_dir.join(format!(
            "{}-{}",
            &short_hash[..short_hash.len().min(12)],
            file_name.to_string_lossy()
        ));
        if !destination.exists() {
            std::fs::copy(&original_path, &destination).map_err(|error| error.to_string())?;
        }
        destination
    } else {
        original_path.clone()
    };
    let source = if copy_into_project {
        estudio_daw_media_adapter::inspect_media_source(&source_path)
            .map_err(|error| error.to_string())?
    } else {
        inspected_source
    };
    let name = source_path
        .file_stem()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Audio".into());
    application
        .execute_project(ProjectCommand::ImportAudio {
            track_id,
            name,
            source,
            start_tick,
            duration_samples: metadata.duration_samples,
            sample_rate: metadata.sample_rate_hz,
            channels: metadata.channels,
            source_channel_selection,
        })
        .map_err(|error| error.to_string())?;
    let connected = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .is_connected();
    Ok(summarize(application, connected))
}

#[tauri::command]
fn audio_waveform(
    source_id: String,
    state: State<'_, DesktopState>,
) -> Result<Vec<[f32; 2]>, String> {
    let application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_ref()
        .ok_or_else(|| "primero crea o abre un proyecto".to_owned())?;
    let project = application.snapshot().project.project;
    let source = project
        .audio_sources
        .iter()
        .find(|source| source.id == source_id)
        .ok_or_else(|| "la fuente de audio ya no está en el proyecto".to_owned())?;
    let bins = estudio_daw_media_adapter::audio_waveform(&source.media.original_path, 512)
        .map_err(|error| error.to_string())?;
    Ok(bins.into_iter().map(|(min, max)| [min, max]).collect())
}

#[tauri::command]
fn audio_preview(source_id: String, state: State<'_, DesktopState>) -> Result<String, String> {
    let path = {
        let application = state
            .application
            .lock()
            .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
        let application = application
            .as_ref()
            .ok_or_else(|| "primero crea o abre un proyecto".to_owned())?;
        let project = application.snapshot().project.project;
        project
            .audio_sources
            .iter()
            .find(|source| source.id == source_id)
            .map(|source| source.media.original_path.clone())
            .ok_or_else(|| "la fuente de audio ya no está en el proyecto".to_owned())?
    };
    encode_audio_preview(path)
}

#[tauri::command]
fn inspect_audio_file(path: String) -> Result<AudioImportMetadata, String> {
    let metadata = estudio_daw_media_adapter::inspect_audio_metadata(path)
        .map_err(|error| error.to_string())?;
    Ok(AudioImportMetadata {
        sample_rate_hz: metadata.sample_rate_hz,
        channels: metadata.channels,
        duration_seconds: metadata.duration_samples as f64 / f64::from(metadata.sample_rate_hz),
    })
}

#[tauri::command]
fn audio_preview_file(path: String) -> Result<String, String> {
    encode_audio_preview(PathBuf::from(path))
}

fn encode_audio_preview(path: PathBuf) -> Result<String, String> {
    let ogg =
        estudio_daw_media_adapter::audio_preview_ogg(path).map_err(|error| error.to_string())?;
    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        ogg,
    ))
}

#[tauri::command]
fn demo_midi_project(state: State<'_, DesktopState>) -> Result<UiSnapshot, String> {
    let mut project = new_project_model();
    project.project_id = format!("demo-midi-{}", unix_timestamp_millis());
    let mut application = ProjectApplication::new(project);
    application
        .execute_project(ProjectCommand::AttachMidiTake {
            take: demo_midi_take(),
            name: "Melodía de prueba".into(),
        })
        .map_err(|error| error.to_string())?;
    replace_application(state, application)
}

fn replace_application(
    state: State<'_, DesktopState>,
    application: ProjectApplication,
) -> Result<UiSnapshot, String> {
    let snapshot = summarize(&application, false);
    let mut current = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .stop()?;
    *current = Some(application);
    Ok(snapshot)
}

fn demo_midi_take() -> MidiTake {
    const PPQ: u32 = 480;
    const TEMPO_BPM: u32 = 120;
    let tick_to_micros =
        |tick: u64| tick.saturating_mul(60_000_000) / (u64::from(PPQ) * u64::from(TEMPO_BPM));
    let notes = [
        (0, 60),
        (480, 62),
        (960, 64),
        (1440, 67),
        (1920, 64),
        (2400, 62),
        (2880, 60),
    ];
    let mut events = Vec::with_capacity(notes.len() * 2);
    for (tick, note) in notes {
        events.push(RecordedMidiEvent {
            tick,
            micros_since_start: tick_to_micros(tick),
            source: MidiSource { client: 0, port: 0 },
            message: RecordedMidiMessage::NoteOn {
                channel: 0,
                note,
                velocity: 96,
            },
        });
        events.push(RecordedMidiEvent {
            tick: tick + 360,
            micros_since_start: tick_to_micros(tick + 360),
            source: MidiSource { client: 0, port: 0 },
            message: RecordedMidiMessage::NoteOff {
                channel: 0,
                note,
                release_velocity: 0,
            },
        });
    }
    events.sort_by_key(|event| event.tick);
    MidiTake {
        ppq: PPQ,
        tempo_bpm: TEMPO_BPM,
        duration_micros: 3_500_000,
        events,
    }
}

fn new_project_model() -> Project {
    Project {
        schema_version: "estudio-daw.project.v5".into(),
        project_id: format!("proyecto-{}", unix_timestamp_millis()),
        transport: Transport {
            tempo_bpm: 120.0,
            time_signature: TimeSignature {
                numerator: 4,
                denominator: 4,
            },
            loop_range: None,
        },
        tracks: vec![Track {
            id: "midi-1".into(),
            name: "MIDI 1".into(),
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
        }],
        audio_sources: Vec::new(),
        audio_playlists: Vec::new(),
        scenes: Vec::new(),
        clip_slots: Vec::new(),
        midi_clips: Vec::new(),
        audio_clips: Vec::new(),
        import_provenance: ImportProvenance {
            format: "estudio-daw".into(),
            format_version: "2".into(),
            source_file: String::new(),
            warnings: Vec::new(),
        },
    }
}

fn unix_timestamp_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[tauri::command]
fn open_project(path: String, state: State<'_, DesktopState>) -> Result<UiSnapshot, String> {
    // Abrir fuera del lock evita bloquear otras llamadas durante lectura/parsing.
    let application = ProjectApplication::open(PathBuf::from(path)).map_err(|e| e.to_string())?;
    let mut current = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .stop()?;
    *current = Some(application);
    let snapshot = summarize(current.as_ref().expect("proyecto recién abierto"), false);
    Ok(snapshot)
}

#[tauri::command]
fn project_snapshot(state: State<'_, DesktopState>) -> Result<UiSnapshot, String> {
    let application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_ref()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    let connected = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .is_connected();
    Ok(summarize(application, connected))
}

#[tauri::command]
fn transport_position(state: State<'_, DesktopState>) -> Result<u64, String> {
    Ok(state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .position_ticks_checked()?)
}

#[tauri::command]
fn track_meters(
    state: State<'_, DesktopState>,
) -> Result<HashMap<String, TrackMeterSummary>, String> {
    state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .track_meter_values()
        .map(|meters| {
            meters
                .into_iter()
                .map(|(track_id, (peak, rms))| (track_id, TrackMeterSummary { peak, rms }))
                .collect()
        })
}

#[tauri::command]
fn set_loop_range(
    start_tick: Option<u64>,
    end_tick: Option<u64>,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let range = match (start_tick, end_tick) {
        (Some(start_tick), Some(end_tick)) => Some(TransportLoopRange {
            start_tick,
            end_tick,
        }),
        (None, None) => None,
        _ => return Err("define los puntos A y B, o limpia ambos".to_owned()),
    };
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero crea o abre un proyecto".to_owned())?;
    application
        .execute_project(ProjectCommand::SetTransportLoopRange { range })
        .map_err(|error| error.to_string())?;
    let connected = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .is_connected();
    Ok(summarize(application, connected))
}

#[tauri::command]
fn add_scene(state: State<'_, DesktopState>) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    let project = application.snapshot().project.project;
    let scene = Scene {
        id: format!("scene-{}", unix_timestamp_nanos()),
        name: format!("Escena {}", project.scenes.len() + 1),
    };
    application
        .execute_project(ProjectCommand::AddScene { scene, index: None })
        .map_err(|error| error.to_string())?;
    let connected = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .is_connected();
    Ok(summarize(application, connected))
}

#[tauri::command]
fn rename_scene(
    scene_id: String,
    name: String,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    application
        .execute_project(ProjectCommand::RenameScene { scene_id, name })
        .map_err(|error| error.to_string())?;
    let connected = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .is_connected();
    Ok(summarize(application, connected))
}

#[tauri::command]
fn remove_scene(scene_id: String, state: State<'_, DesktopState>) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    application
        .execute_project(ProjectCommand::RemoveScene { scene_id })
        .map_err(|error| error.to_string())?;
    let connected = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .is_connected();
    Ok(summarize(application, connected))
}

#[tauri::command]
fn move_scene(
    scene_id: String,
    index: usize,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    application
        .execute_project(ProjectCommand::MoveScene { scene_id, index })
        .map_err(|error| error.to_string())?;
    let connected = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .is_connected();
    Ok(summarize(application, connected))
}

#[tauri::command]
fn set_clip_slot(
    scene_id: String,
    track_id: String,
    clip_kind: Option<String>,
    clip_id: Option<String>,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    let project = application.snapshot().project.project;
    let existing = project
        .clip_slots
        .iter()
        .find(|slot| slot.scene_id == scene_id && slot.track_id == track_id);
    let clip = match (clip_kind.as_deref(), clip_id) {
        (None, None) => None,
        (Some("midi"), Some(id)) => Some(ClipReference::Midi(id)),
        (Some("audio"), Some(id)) => Some(ClipReference::Audio(id)),
        _ => return Err("elige un clip MIDI/audio válido o deja la casilla vacía".into()),
    };
    if clip.is_none() {
        if let Some(existing) = existing {
            application
                .execute_project(ProjectCommand::RemoveClipSlot {
                    slot_id: existing.id.clone(),
                })
                .map_err(|error| error.to_string())?;
        }
    } else {
        let slot = ClipSlot {
            id: existing.map_or_else(
                || format!("slot-{}", unix_timestamp_nanos()),
                |slot| slot.id.clone(),
            ),
            scene_id,
            track_id,
            clip,
            launch_quantization: existing
                .map(|slot| slot.launch_quantization)
                .unwrap_or_default(),
            launch_mode: existing.map(|slot| slot.launch_mode).unwrap_or_default(),
        };
        application
            .execute_project(ProjectCommand::SetClipSlot { slot })
            .map_err(|error| error.to_string())?;
    }
    let connected = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .is_connected();
    Ok(summarize(application, connected))
}

#[tauri::command]
fn set_session_launch_quantization(
    scene_id: String,
    track_id: String,
    launch_quantization: ClipLaunchQuantization,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    let mut slot = application
        .snapshot()
        .project
        .project
        .clip_slots
        .into_iter()
        .find(|slot| slot.scene_id == scene_id && slot.track_id == track_id)
        .ok_or_else(|| "asigna un clip a la casilla antes de cambiar su cuantización".to_owned())?;
    if slot.clip.is_none() {
        return Err("asigna un clip a la casilla antes de cambiar su cuantización".to_owned());
    }
    slot.launch_quantization = launch_quantization;
    application
        .execute_project(ProjectCommand::SetClipSlot { slot })
        .map_err(|error| error.to_string())?;
    let connected = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .is_connected();
    Ok(summarize(application, connected))
}

#[tauri::command]
fn set_session_launch_mode(
    scene_id: String,
    track_id: String,
    launch_mode: ClipLaunchMode,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    let mut slot = application
        .snapshot()
        .project
        .project
        .clip_slots
        .into_iter()
        .find(|slot| slot.scene_id == scene_id && slot.track_id == track_id)
        .ok_or_else(|| "asigna un clip a la casilla antes de cambiar su modo".to_owned())?;
    if slot.clip.is_none() {
        return Err("asigna un clip a la casilla antes de cambiar su modo".to_owned());
    }
    slot.launch_mode = launch_mode;
    application
        .execute_project(ProjectCommand::SetClipSlot { slot })
        .map_err(|error| error.to_string())?;
    let connected = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .is_connected();
    Ok(summarize(application, connected))
}

#[tauri::command]
fn launch_session_slot(
    scene_id: String,
    track_id: String,
    grid_ticks: u64,
    respect_clip_quantization: bool,
    state: State<'_, DesktopState>,
) -> Result<SessionLaunchView, String> {
    let application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_ref()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    let project = application.snapshot().project.project;
    state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .launch_session_slot(
            &project,
            &scene_id,
            &track_id,
            grid_ticks,
            respect_clip_quantization,
            None,
        )
}

#[tauri::command]
fn launch_session_scene(
    scene_id: String,
    grid_ticks: u64,
    state: State<'_, DesktopState>,
) -> Result<Vec<SessionLaunchView>, String> {
    let application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_ref()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    let project = application.snapshot().project.project;
    state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .launch_session_scene(&project, &scene_id, grid_ticks)
}

#[tauri::command]
fn stop_session_track(track_id: String, state: State<'_, DesktopState>) -> Result<(), String> {
    state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .stop_session_track(&track_id)
}

#[tauri::command]
fn session_launches(state: State<'_, DesktopState>) -> Result<Vec<SessionLaunchView>, String> {
    let mut audio = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?;
    audio.session_launches()
}

fn unix_timestamp_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[tauri::command]
fn save_project(state: State<'_, DesktopState>) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    let audio = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?;
    persist_vst3_plugin_states(application, &audio)?;
    application.save().map_err(|e| e.to_string())?;
    Ok(summarize(application, audio.is_connected()))
}

#[tauri::command]
fn save_project_as(path: String, state: State<'_, DesktopState>) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    let audio = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?;
    application
        .save_as(PathBuf::from(path))
        .map_err(|e| e.to_string())?;
    persist_vst3_plugin_states(application, &audio)?;
    application.save().map_err(|e| e.to_string())?;
    Ok(summarize(application, audio.is_connected()))
}

#[tauri::command]
fn set_transport(
    command: String,
    position_ticks: Option<u64>,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    if command == "metronome-on" || command == "metronome-off" {
        let audio = state
            .audio
            .lock()
            .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?;
        audio.set_metronome(command == "metronome-on");
        return Ok(summarize(application, audio.is_connected()));
    }
    if command == "panic" {
        let mut audio = state
            .audio
            .lock()
            .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?;
        audio.panic()?;
        return Ok(summarize(application, audio.is_connected()));
    }
    if command == "seek" {
        let position_ticks =
            position_ticks.ok_or_else(|| "la búsqueda requiere una posición musical".to_owned())?;
        let settings = load_audio_runtime_settings().map_err(|error| error.to_string())?;
        let project = application.snapshot().project.project;
        let mut audio = state
            .audio
            .lock()
            .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?;
        audio.seek(&project, settings.active(), position_ticks)?;
        return Ok(summarize(application, audio.is_connected()));
    }
    let recording = command == "record";
    let session_command = match command.as_str() {
        "play" | "record" => SessionCommand::Play,
        "pause" => SessionCommand::Pause,
        "stop" => SessionCommand::Stop,
        _ => return Err(format!("comando de transporte desconocido: {command}")),
    };
    let mut audio = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?;
    if command == "pause" && audio.is_recording() {
        return Err(
            "detén la grabación antes de pausar; así se conserva la continuidad de la toma".into(),
        );
    }
    let recording_directory = if recording {
        if audio.is_connected() {
            return Err("detén el transporte antes de iniciar una grabación".into());
        }
        let project = application.snapshot().project.project;
        if project.transport.loop_range.is_some() {
            return Err("desactiva el rango A/B antes de grabar; la grabación por secciones aún no está disponible".into());
        }
        if !project
            .tracks
            .iter()
            .any(|track| track.record_armed && track.input_route.is_some())
        {
            return Err(
                "asigna una entrada y arma al menos una pista de audio antes de grabar".into(),
            );
        }
        let project_path = application
            .project_path()
            .ok_or_else(|| "guarda el proyecto antes de grabar audio".to_owned())?;
        let directory = project_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("media")
            .join("recordings");
        std::fs::create_dir_all(&directory)
            .map_err(|error| format!("no se pudo preparar media/recordings: {error}"))?;
        Some(directory)
    } else {
        None
    };
    let settings = if matches!(session_command, SessionCommand::Play) {
        Some(load_audio_runtime_settings().map_err(|error| error.to_string())?)
    } else {
        None
    };
    application
        .execute(DomainCommand::Session(session_command), CommandAuthor::User)
        .map_err(|e| e.to_string())?;
    match session_command {
        SessionCommand::Play => {
            let project = application.snapshot().project.project;
            let settings = settings
                .as_ref()
                .ok_or_else(|| "faltan preferencias para iniciar audio".to_owned())?;
            let active_profile = settings.active();
            if let Err(error) = audio.play(
                &project,
                active_profile,
                position_ticks.unwrap_or_default(),
                &settings.backend_device_key,
                recording_directory,
                application
                    .project_path()
                    .and_then(|path| path.parent())
                    .map(PathBuf::from),
            ) {
                // Keep the domain transport stopped if device/backend startup
                // fails after the command was accepted.
                let rollback = application.execute(
                    DomainCommand::Session(SessionCommand::Stop),
                    CommandAuthor::User,
                );
                return match rollback {
                    Ok(_) => Err(error),
                    Err(rollback_error) => Err(format!(
                        "{error}; además no se pudo revertir el transporte: {rollback_error}"
                    )),
                };
            }
        }
        SessionCommand::Pause => audio.pause(true),
        SessionCommand::Stop => {
            persist_vst3_plugin_states(application, &audio)?;
            for recording in audio.stop()? {
                import_finished_recording(application, recording)?;
            }
        }
        _ => unreachable!("el adaptador sólo acepta play/pause/stop"),
    }
    Ok(summarize(application, audio.is_connected()))
}

fn persist_vst3_plugin_states(
    application: &mut ProjectApplication,
    audio: &AudioRuntimeHost,
) -> Result<(), String> {
    if !audio.is_connected() {
        return Ok(());
    }
    let Some(root) = application
        .project_path()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
    else {
        return Ok(());
    };
    let snapshots = audio.persist_vst3_states(&root)?;
    if snapshots.is_empty() {
        return Ok(());
    }
    let project = application.snapshot().project.project;
    for (track_id, state) in snapshots {
        let Some(track) = project.tracks.iter().find(|track| track.id == track_id) else {
            continue;
        };
        let Some(InstrumentConfig::Vst3 { plugin, .. }) = track.instrument.clone() else {
            continue;
        };
        application
            .execute_project(ProjectCommand::SetTrackInstrument {
                track_id,
                instrument: Some(InstrumentConfig::Vst3 {
                    plugin,
                    state: Some(state),
                }),
            })
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn import_finished_recording(
    application: &mut ProjectApplication,
    recording: audio_runtime::FinishedInputRecording,
) -> Result<(), String> {
    if recording.frames == 0 {
        return Err(format!(
            "la grabación de '{}' no contiene muestras; el archivo se conserva en {}",
            recording.track_name,
            recording.path.display()
        ));
    }
    let source =
        estudio_daw_media_adapter::inspect_media_source(&recording.path).map_err(|error| {
            format!(
                "se capturó '{}', pero no se pudo indexar {}: {error}",
                recording.track_name,
                recording.path.display()
            )
        })?;
    application
        .execute_project(ProjectCommand::ImportAudio {
            track_id: recording.track_id.clone(),
            name: format!("Grabación {}", recording.track_name),
            source,
            start_tick: recording.start_tick,
            duration_samples: recording.frames,
            sample_rate: recording.sample_rate_hz,
            channels: 2,
            source_channel_selection: recording.channel_selection,
        })
        .map_err(|error| {
            format!(
                "se capturó el audio, pero no se pudo añadir la región ({}): {error}",
                recording.path.display()
            )
        })?;
    if recording.dropped_samples > 0 {
        return Err(format!(
            "grabación creada con {} muestras descartadas; archivo: {}",
            recording.dropped_samples,
            recording.path.display()
        ));
    }
    Ok(())
}

#[tauri::command]
fn set_track_mixer(
    track_id: String,
    active: bool,
    mute: bool,
    solo: bool,
    gain_db: f32,
    pan: f32,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    application
        .execute_project(ProjectCommand::SetTrackMixer {
            track_id,
            mixer: TrackMixerState {
                active,
                mute,
                solo,
                gain_db,
                pan,
            },
        })
        .map_err(|error| error.to_string())?;
    let project = application.snapshot().project.project;
    let mut audio = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?;
    let connected = audio.is_connected();
    if connected {
        let settings = load_audio_runtime_settings().map_err(|error| error.to_string())?;
        audio.refresh_project(&project, settings.active()).map_err(|error| {
            format!("el cambio de mezcla quedó guardado, pero no se pudo actualizar el audio: {error}")
        })?;
    }
    Ok(summarize(application, connected))
}

#[tauri::command]
fn set_track_identity(
    track_id: String,
    color: String,
    marker: String,
    annotation: String,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    application
        .execute_project(ProjectCommand::SetTrackIdentity {
            track_id,
            color,
            marker,
            annotation,
        })
        .map_err(|error| error.to_string())?;
    let connected = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .is_connected();
    Ok(summarize(application, connected))
}

#[tauri::command]
fn set_track_instrument(
    track_id: String,
    instrument: Option<InstrumentConfig>,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    if state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .is_connected()
    {
        return Err("detén el transporte antes de cambiar el instrumento de pista".into());
    }
    application
        .execute_project(ProjectCommand::SetTrackInstrument {
            track_id,
            instrument,
        })
        .map_err(|error| error.to_string())?;
    Ok(summarize(application, false))
}

#[tauri::command]
fn inspect_vst3_plugin(path: String) -> Result<vst3_host::discovery::DetailedPluginInfo, String> {
    if !path.to_ascii_lowercase().ends_with(".vst3") {
        return Err("selecciona un bundle VST3 (.vst3)".into());
    }
    vst3_host::probe_plugin_info_isolated(
        std::path::Path::new(&path),
        std::time::Duration::from_secs(30),
    )
    .map_err(|error| format!("no se pudo leer el plugin VST3: {error}"))
}

#[tauri::command]
fn set_vst3_editor(
    track_id: String,
    open: bool,
    state: State<'_, DesktopState>,
) -> Result<(), String> {
    state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .set_vst3_editor(&track_id, open)
}

#[tauri::command]
fn open_standalone_instrument(
    track_id: String,
    application_path: String,
    wine_prefix: Option<String>,
    state: State<'_, DesktopState>,
) -> Result<(), String> {
    let application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let project = application
        .as_ref()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?
        .snapshot()
        .project
        .project;
    let track = project
        .tracks
        .iter()
        .find(|track| track.id == track_id)
        .ok_or_else(|| "la pista seleccionada ya no existe".to_owned())?;
    if !matches!(&track.kind, TrackKind::Midi) {
        return Err("Analog Lab standalone requiere una pista MIDI".into());
    }
    drop(application);
    state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .open_standalone_application(&track_id, &application_path, wine_prefix.as_deref())
}

#[tauri::command]
fn set_track_output(
    track_id: String,
    output_track_id: Option<String>,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    application
        .execute_project(ProjectCommand::SetTrackOutput {
            track_id,
            output_track_id,
        })
        .map_err(|error| error.to_string())?;
    let project = application.snapshot().project.project;
    let mut audio = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?;
    let connected = audio.is_connected();
    if connected {
        let settings = load_audio_runtime_settings().map_err(|error| error.to_string())?;
        audio
            .refresh_project(&project, settings.active())
            .map_err(|error| {
                format!("el ruteo quedó guardado, pero no se pudo actualizar el audio: {error}")
            })?;
    }
    Ok(summarize(application, connected))
}

#[tauri::command]
fn set_track_input_route(
    track_id: String,
    input_route: Option<TrackInputRoute>,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    application
        .execute_project(ProjectCommand::SetTrackInputRoute {
            track_id,
            input_route,
        })
        .map_err(|error| error.to_string())?;
    let connected = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .is_connected();
    Ok(summarize(application, connected))
}

#[tauri::command]
fn set_track_record_arm(
    track_id: String,
    armed: bool,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    application
        .execute_project(ProjectCommand::SetTrackRecordArm { track_id, armed })
        .map_err(|error| error.to_string())?;
    let connected = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .is_connected();
    Ok(summarize(application, connected))
}

#[tauri::command]
fn set_tracks_group(
    track_ids: Vec<String>,
    group_name: Option<String>,
    state: State<'_, DesktopState>,
) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    application
        .execute_project(ProjectCommand::SetTracksGroup {
            track_ids,
            group_name,
        })
        .map_err(|error| error.to_string())?;
    let connected = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .is_connected();
    Ok(summarize(application, connected))
}

#[tauri::command]
fn history_action(action: String, state: State<'_, DesktopState>) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    match action.as_str() {
        "undo" => application.undo(),
        "redo" => application.redo(),
        _ => return Err(format!("acción de historial desconocida: {action}")),
    }
    .map_err(|e| e.to_string())?;
    let project = application.snapshot().project.project;
    let mut audio = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?;
    let connected = audio.is_connected();
    if connected {
        let settings = load_audio_runtime_settings().map_err(|error| error.to_string())?;
        audio
            .refresh_project(&project, settings.active())
            .map_err(|error| {
                format!("el historial se actualizó, pero no se pudo refrescar el audio: {error}")
            })?;
    }
    Ok(summarize(application, connected))
}

#[tauri::command]
fn audio_runtime_settings(state: State<'_, DesktopState>) -> Result<AudioRuntimeView, String> {
    let settings = load_audio_runtime_settings().map_err(|error| error.to_string())?;
    let mut view = AudioRuntimeView::from_settings(settings, 48_000);
    view.engine_connected = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .is_connected();
    Ok(view)
}

#[tauri::command]
fn audio_output_devices() -> Result<Vec<AudioOutputDeviceSummary>, String> {
    audio_devices()
        .map_err(|error| format!("no se pudieron consultar las salidas de audio: {error}"))
        .map(|devices| {
            devices
                .into_iter()
                .filter(|device| device.media_class.to_ascii_lowercase().contains("sink"))
                .map(|device| AudioOutputDeviceSummary {
                    key: format!("pipewire:{}", device.name),
                    name: device.name,
                    description: device.description,
                })
                .collect()
        })
}

#[tauri::command]
fn audio_input_devices() -> Result<Vec<AudioOutputDeviceSummary>, String> {
    audio_devices()
        .map_err(|error| format!("no se pudieron consultar las entradas de audio: {error}"))
        .map(|devices| {
            devices
                .into_iter()
                .filter(|device| {
                    let class = device.media_class.to_ascii_lowercase();
                    class.contains("source") && !class.contains("monitor")
                })
                .map(|device| AudioOutputDeviceSummary {
                    key: format!("pipewire:{}", device.name),
                    name: device.name,
                    description: device.description,
                })
                .collect()
        })
}

#[tauri::command]
fn audio_return_devices() -> Result<Vec<AudioOutputDeviceSummary>, String> {
    audio_devices()
        .map_err(|error| format!("no se pudieron consultar los retornos de audio: {error}"))
        .map(|devices| {
            devices
                .into_iter()
                .filter(|device| {
                    let class = device.media_class.to_ascii_lowercase();
                    class.contains("stream/output/audio")
                        && !class.contains("monitor")
                        && !device.name.to_ascii_lowercase().contains(".monitor")
                })
                .map(|device| AudioOutputDeviceSummary {
                    key: format!("pipewire:{}", device.name),
                    name: device.name,
                    description: device.description,
                })
                .collect()
        })
}

#[tauri::command]
fn midi_output_devices() -> Result<Vec<MidiOutputDeviceSummary>, String> {
    let output = std::process::Command::new("pw-link")
        .args(["-i", "-v"])
        .output()
        .map_err(|error| {
            format!("no se pudieron consultar los puertos MIDI de PipeWire: {error}")
        })?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    Ok(parse_pipewire_midi_inputs(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn parse_pipewire_midi_inputs(output: &str) -> Vec<MidiOutputDeviceSummary> {
    let mut ports = Vec::new();
    let (mut current_port, mut is_midi) = (None::<String>, false);
    let flush = |port: Option<String>, is_midi: bool, ports: &mut Vec<MidiOutputDeviceSummary>| {
        if is_midi {
            if let Some(port) = port {
                ports.push(MidiOutputDeviceSummary {
                    key: port.clone(),
                    label: port,
                });
            }
        }
    };
    for line in output.lines() {
        if !line.chars().next().is_some_and(char::is_whitespace) {
            flush(current_port.take(), is_midi, &mut ports);
            current_port = Some(line.trim().to_owned());
            let port = line.to_ascii_lowercase();
            is_midi =
                port.contains("midi") || port.contains("events-in") || port.contains("event-in");
        } else {
            let metadata = line.to_ascii_lowercase();
            is_midi |= metadata.contains("alsa:seq")
                || metadata.contains("midi")
                || metadata.contains("events-in")
                || metadata.contains("event-in");
        }
    }
    flush(current_port, is_midi, &mut ports);
    ports
}

#[tauri::command]
fn save_audio_settings(
    settings: AudioRuntimeSettings,
    state: State<'_, DesktopState>,
) -> Result<AudioRuntimeView, String> {
    save_audio_runtime_settings(&settings).map_err(|error| error.to_string())?;
    let mut view = AudioRuntimeView::from_settings(settings, 48_000);
    view.engine_connected = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .is_connected();
    Ok(view)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(DesktopState {
            application: Mutex::new(None),
            audio: Mutex::new(AudioRuntimeHost::default()),
        })
        .invoke_handler(tauri::generate_handler![
            new_project,
            add_track,
            move_track,
            duplicate_track,
            remove_track,
            import_audio,
            edit_audio_region,
            quantize_midi_clip,
            move_midi_clip,
            duplicate_midi_clip,
            split_midi_clip,
            add_midi_note,
            update_midi_note,
            remove_midi_note,
            audio_waveform,
            audio_preview,
            audio_preview_file,
            inspect_audio_file,
            demo_midi_project,
            open_project,
            project_snapshot,
            transport_position,
            track_meters,
            set_loop_range,
            add_scene,
            rename_scene,
            remove_scene,
            move_scene,
            set_clip_slot,
            set_session_launch_quantization,
            set_session_launch_mode,
            launch_session_slot,
            launch_session_scene,
            stop_session_track,
            session_launches,
            save_project,
            save_project_as,
            set_transport,
            set_track_mixer,
            set_track_identity,
            set_track_instrument,
            inspect_vst3_plugin,
            set_vst3_editor,
            open_standalone_instrument,
            set_track_output,
            set_track_input_route,
            set_track_record_arm,
            set_tracks_group,
            history_action,
            audio_runtime_settings,
            audio_output_devices,
            audio_input_devices,
            audio_return_devices,
            midi_output_devices,
            save_audio_settings
        ])
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar el shell de Estudio DAW");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_project_starts_with_one_empty_midi_track_and_enabled_session_state() {
        let application = ProjectApplication::new(new_project_model());
        let snapshot = summarize(&application, false);

        assert!(snapshot.project_id.starts_with("proyecto-"));
        assert_eq!(snapshot.tempo_bpm, 120.0);
        assert_eq!(snapshot.track_count, 1);
        assert_eq!(snapshot.midi_clip_count, 0);
        assert!(snapshot.midi_clips.is_empty());
        assert_eq!(snapshot.tracks[0].kind, "midi");
        assert_eq!(snapshot.tracks[0].name, "MIDI 1");
    }

    #[test]
    fn demo_project_contains_playable_midi_notes_and_clip_summary() {
        let mut application = ProjectApplication::new(new_project_model());
        application
            .execute_project(ProjectCommand::AttachMidiTake {
                take: demo_midi_take(),
                name: "Melodía de prueba".into(),
            })
            .unwrap();
        let snapshot = summarize(&application, false);

        assert_eq!(snapshot.midi_clip_count, 1);
        assert_eq!(snapshot.midi_clips.len(), 1);
        assert_eq!(snapshot.midi_clips[0].note_count, 7);
        assert_eq!(snapshot.midi_clips[0].notes.len(), 7);
        assert_eq!(snapshot.midi_clips[0].ppq, demo_midi_take().ppq);
        assert_eq!(snapshot.midi_clips[0].notes[0].key, 60);
        assert_eq!(snapshot.midi_clips[0].notes[0].duration_beats, 0.75);
        assert!((snapshot.midi_clips[0].duration_beats - 6.75).abs() < f64::EPSILON);
        assert_eq!(snapshot.tracks[0].note_count, 7);
        let take = demo_midi_take();
        assert_eq!(take.events[2].tick, 480);
        assert_eq!(take.events[2].micros_since_start, 500_000);
    }

    #[test]
    fn clip_summary_converts_each_clip_to_beats_using_its_own_ppq() {
        let mut project = new_project_model();
        let mut take = demo_midi_take();
        take.events.clear();
        project.midi_clips = vec![
            MidiClip {
                id: "clip-480".into(),
                name: "480 PPQ".into(),
                track_id: "midi-1".into(),
                start_tick: 480,
                duration_ticks: 960,
                take: take.clone(),
            },
            MidiClip {
                id: "clip-960".into(),
                name: "960 PPQ".into(),
                track_id: "midi-1".into(),
                start_tick: 1920,
                duration_ticks: 1920,
                take: MidiTake { ppq: 960, ..take },
            },
        ];
        let snapshot = summarize(&ProjectApplication::new(project), false);

        assert_eq!(snapshot.midi_clips[0].ppq, 480);
        assert_eq!(snapshot.midi_clips[1].ppq, 960);
        assert_eq!(snapshot.midi_clips[0].start_beats, 1.0);
        assert_eq!(snapshot.midi_clips[0].duration_beats, 2.0);
        assert_eq!(snapshot.midi_clips[1].start_beats, 2.0);
        assert_eq!(snapshot.midi_clips[1].duration_beats, 2.0);
    }

    #[test]
    fn ui_snapshot_serializes_summary_without_audio_payload() {
        let project: Project = serde_json::from_str(
            r#"{
                "schema_version":"1",
                "project_id":"ui-contract-test",
                "transport":{"tempo_bpm":96.0,"time_signature":{"numerator":4,"denominator":4}},
                "tracks":[],
                "midi_clips":[],
                "audio_clips":[],
                "import_provenance":{"format":"internal","format_version":"1","source_file":"","warnings":[]}
            }"#,
        )
        .expect("el fixture del contrato UI debe ser un proyecto válido");
        let application = ProjectApplication::new(project);
        let value = serde_json::to_value(summarize(&application, false))
            .expect("el snapshot compacto debe serializarse");

        assert_eq!(value["projectId"], "ui-contract-test");
        assert_eq!(value["schemaVersion"], "ui-snapshot.v1");
        assert_eq!(value["tempoBpm"], 96.0);
        assert_eq!(value["trackCount"], 0);
        assert_eq!(value["audioEngineConnected"], false);
        assert!(value.get("samples").is_none());
        assert!(value.get("pcm").is_none());
        assert!(value.get("gpu_buffers").is_none());
    }

    #[test]
    fn ui_track_snapshot_exposes_standalone_instrument_assignment() {
        let mut project = new_project_model();
        project.tracks[0].instrument = Some(InstrumentConfig::Standalone {
            application_path: "/wine/Analog Lab V.exe".into(),
            wine_prefix: Some("/wine-prefix".into()),
            midi_output: Some(estudio_daw_project_model::ExternalMidiPort {
                device_key: "AnalogLab:events-in".into(),
                port_name: "AnalogLab:events-in".into(),
                channel: None,
            }),
            audio_input: None,
        });
        let application = ProjectApplication::new(project);
        let snapshot = serde_json::to_value(summarize(&application, false)).unwrap();
        assert_eq!(snapshot["tracks"][0]["instrument"]["backend"], "standalone");
        assert_eq!(
            snapshot["tracks"][0]["instrument"]["application_path"],
            "/wine/Analog Lab V.exe"
        );
    }

    #[test]
    fn pipewire_midi_port_discovery_skips_audio_and_keeps_jack_midi_inputs() {
        let ports = parse_pipewire_midi_inputs(
            "alsa_output:playback_FL\n  alsa:acp:device:playback_0\nAnalogLab:events-in\n  jack:AnalogLab:events-in\nMidi-Bridge:Arturia KeyLab: MID (playback)\n  alsa:seq:default:client_28:playback_0\n",
        );
        let labels: Vec<_> = ports.iter().map(|port| port.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "AnalogLab:events-in",
                "Midi-Bridge:Arturia KeyLab: MID (playback)"
            ]
        );
    }

    #[test]
    fn ui_track_snapshot_exposes_audio_channels_and_master_destination() {
        let mut application = ProjectApplication::new(new_project_model());
        let track = Track::new(
            "audio-ui-test",
            "Audio 1",
            TrackKind::Audio,
            TrackRole::Audio,
        )
        .unwrap();
        application
            .execute_project(ProjectCommand::AddTrack { track, index: None })
            .unwrap();

        let value = serde_json::to_value(summarize(&application, false)).unwrap();
        let tracks = value["tracks"].as_array().unwrap();
        let audio = tracks
            .iter()
            .find(|track| track["id"] == "audio-ui-test")
            .unwrap();
        let master = tracks
            .iter()
            .find(|track| track["role"] == "master")
            .unwrap();
        assert_eq!(audio["inputChannels"], 2);
        assert_eq!(audio["outputChannels"], 2);
        assert_eq!(audio["outputTrackId"], master["id"]);
    }

    #[test]
    fn visualization_contracts_are_versioned_and_use_opaque_artifact_ids() {
        let reference = VisualizationArtifactRefV1 {
            schema_version: "visualization-artifact-ref.v1",
            artifact_id: "viz_opaque_01HXYZ".to_owned(),
            kind: "waveform-minmax-pyramid",
            project_revision: 7,
            source_digest: "sha256:abc123".to_owned(),
            sample_rate_hz: 48_000,
            channel_count: 2,
            frame_count: 96_000,
            level_count: 8,
        };
        let value = serde_json::to_value(reference).expect("serialización de referencia");

        assert_eq!(value["schemaVersion"], "visualization-artifact-ref.v1");
        assert_eq!(value["artifactId"], "viz_opaque_01HXYZ");
        assert!(value.get("path").is_none());
        assert!(value.get("gpuBuffer").is_none());
        assert!(value.get("samples").is_none());

        let chunk = WaveformChunkV1 {
            schema_version: "waveform-chunk.v1",
            artifact_id: "viz_opaque_01HXYZ".to_owned(),
            level: 3,
            first_bin: 128,
            bins: vec![WaveformBinV1 {
                min: -0.8,
                max: 0.9,
            }],
        };
        let chunk = serde_json::to_value(chunk).expect("serialización de waveform");
        assert_eq!(chunk["bins"].as_array().map(Vec::len), Some(1));
        assert_eq!(chunk["firstBin"], 128);

        let meter = MeterFrameV1 {
            schema_version: "meter-frame.v1",
            sequence: 12,
            at_sample: 48_000,
            sample_rate_hz: 48_000,
            window_frames: 256,
            meters: vec![MeterReadingV1 {
                channel_id: "master".to_owned(),
                peak_dbfs: -1.5,
                rms_dbfs: -12.0,
                clipping: false,
            }],
        };
        let meter = serde_json::to_value(meter).expect("serialización de medidores");
        assert_eq!(meter["schemaVersion"], "meter-frame.v1");
        assert_eq!(meter["meters"][0]["channelId"], "master");
        assert!(meter.get("samples").is_none());

        let tile = SpectrogramTileRefV1 {
            schema_version: "spectrogram-tile-ref.v1",
            artifact_id: "spec_opaque_01HXYZ".to_owned(),
            tile_x: 4,
            tile_y: 2,
            width: 256,
            height: 256,
            encoding: "image/webp",
        };
        let tile = serde_json::to_value(tile).expect("serialización de tile");
        assert_eq!(tile["encoding"], "image/webp");
        assert!(tile.get("path").is_none());
    }
}
