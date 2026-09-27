//! Adaptador Tauri de escritorio.
//!
//! Esta capa traduce llamadas de la UI a la API de aplicación. No mueve audio
//! ni buffers de GPU por IPC y no accede al motor RT desde los comandos de UI.

mod audio_runtime;

use estudio_daw_application::{
    load_audio_runtime_settings, save_audio_runtime_settings, AudioRuntimeSettings,
    AudioRuntimeView, CommandAuthor, DomainCommand, ProjectApplication, SessionCommand,
    TransportState,
};
use estudio_daw_project_model::{Project, TrackKind};
use serde::Serialize;
use std::{path::PathBuf, sync::Mutex};
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
    note_count: usize,
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
    track_count: usize,
    midi_clip_count: usize,
    audio_clip_count: usize,
    tracks: Vec<TrackSummary>,
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
            note_count: track.notes.len(),
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
        track_count: project.tracks.len(),
        midi_clip_count: project.midi_clips.len(),
        audio_clip_count: project.audio_clips.len(),
        tracks,
        can_undo,
        can_redo,
        audio_engine_connected,
    }
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
fn set_transport(command: String, state: State<'_, DesktopState>) -> Result<UiSnapshot, String> {
    let mut application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    let application = application
        .as_mut()
        .ok_or_else(|| "primero abre un proyecto".to_owned())?;
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
    match session_command {
        SessionCommand::Play => {
            let project = application.snapshot().project.project;
            let settings = load_audio_runtime_settings().map_err(|error| error.to_string())?;
            audio.play(&project, settings.active())?;
        }
        SessionCommand::Pause => audio.pause(true),
        SessionCommand::Stop => audio.stop()?,
        _ => unreachable!("el adaptador sólo acepta play/pause/stop"),
    }
    application
        .execute(DomainCommand::Session(session_command), CommandAuthor::User)
        .map_err(|e| e.to_string())?;
    Ok(summarize(application, audio.is_connected()))
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
    let connected = state
        .audio
        .lock()
        .map_err(|_| "el estado del motor de audio quedó bloqueado".to_owned())?
        .is_connected();
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
            open_project,
            project_snapshot,
            save_project,
            save_project_as,
            set_transport,
            history_action,
            audio_runtime_settings,
            save_audio_settings
        ])
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar el shell de Estudio DAW");
}

#[cfg(test)]
mod tests {
    use super::*;

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
