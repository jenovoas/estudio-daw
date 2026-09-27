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
    ImportProvenance, InstrumentConfig, MidiClip, Project, TimeSignature, Track,
    TrackChannelConfig, TrackKind, TrackMixerState, TrackRole, Transport, TransportLoopRange,
};
use estudio_daw_runtime_diagnostics::audio_devices;
use serde::Serialize;
use std::{collections::HashMap, path::PathBuf, sync::Mutex};
use tauri::State;

use audio_runtime::AudioRuntimeHost;

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
    color: String,
    input_channels: Option<u32>,
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
struct MidiClipSummary {
    id: String,
    name: String,
    track_id: String,
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
            color: track.color.clone(),
            input_channels: track.channel_config.input_channels,
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

fn summarize_midi_notes(clip: &MidiClip) -> Vec<MidiNoteSummary> {
    let ppq = f64::from(clip.take.ppq.max(1));
    let mut active_notes = Vec::<(u8, u8, u64, u8)>::new();
    let mut notes = Vec::new();
    for event in &clip.take.events {
        match &event.message {
            RecordedMidiMessage::NoteOn {
                channel,
                note,
                velocity,
            } if *velocity > 0 => {
                active_notes.push((*channel, *note, event.tick, (*velocity).clamp(1, 127) as u8))
            }
            RecordedMidiMessage::NoteOff { channel, note, .. }
            | RecordedMidiMessage::NoteOn { channel, note, .. } => {
                if let Some(index) =
                    active_notes
                        .iter()
                        .rposition(|(active_channel, active_note, _, _)| {
                            active_channel == channel && active_note == note
                        })
                {
                    let (_, key, start_tick, velocity) = active_notes.remove(index);
                    notes.push(MidiNoteSummary {
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
        channel_config: TrackChannelConfig {
            input_channels: if role == TrackRole::Audio {
                Some(2)
            } else {
                None
            },
            output_channels: 2,
        },
        color: "#58a6b8".into(),
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
            channel_config: TrackChannelConfig::default(),
            color: "#58a6b8".into(),
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
fn save_project(state: State<'_, DesktopState>) -> Result<UiSnapshot, String> {
    let application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_ref()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
    application.save().map_err(|e| e.to_string())?;
    let connected = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .is_connected();
    Ok(summarize(application, connected))
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
    application
        .save_as(PathBuf::from(path))
        .map_err(|e| e.to_string())?;
    let connected = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .is_connected();
    Ok(summarize(application, connected))
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
    let session_command = match command.as_str() {
        "play" => SessionCommand::Play,
        "pause" => SessionCommand::Pause,
        "stop" => SessionCommand::Stop,
        _ => return Err(format!("comando de transporte desconocido: {command}")),
    };
    let mut audio = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?;
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
        SessionCommand::Stop => audio.stop()?,
        _ => unreachable!("el adaptador sólo acepta play/pause/stop"),
    }
    Ok(summarize(application, audio.is_connected()))
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
            remove_track,
            import_audio,
            edit_audio_region,
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
            save_project,
            save_project_as,
            set_transport,
            set_track_mixer,
            set_track_output,
            set_tracks_group,
            history_action,
            audio_runtime_settings,
            audio_output_devices,
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
