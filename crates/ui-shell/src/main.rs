//! Adaptador Tauri de escritorio.
//!
//! Esta capa traduce llamadas de la UI a la API de aplicación. No mueve audio
//! ni buffers de GPU por IPC y no accede al motor RT desde los comandos de UI.

use estudio_daw_application::{
    CommandAuthor, DomainCommand, ProjectApplication, SessionCommand, TransportState,
};
use estudio_daw_project_model::{Project, TrackKind};
use serde::Serialize;
use std::{path::PathBuf, sync::Mutex};
use tauri::State;

struct DesktopState {
    // ProjectApplication conserva su runtime/historial; el lock sólo se toma
    // desde comandos de UI, nunca desde el callback de audio.
    application: Mutex<Option<ProjectApplication>>,
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

fn summarize(application: &ProjectApplication) -> UiSnapshot {
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
        // El prototipo sólo actualiza el estado de sesión; aún no conecta
        // TransportSnapshot con el RenderPlan ni el stream de audio.
        audio_engine_connected: false,
    }
}

#[tauri::command]
fn open_project(path: String, state: State<'_, DesktopState>) -> Result<UiSnapshot, String> {
    // Abrir fuera del lock evita bloquear otras llamadas durante lectura/parsing.
    let application = ProjectApplication::open(PathBuf::from(path)).map_err(|e| e.to_string())?;
    let snapshot = summarize(&application);
    *state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())? = Some(application);
    Ok(snapshot)
}

#[tauri::command]
fn project_snapshot(state: State<'_, DesktopState>) -> Result<UiSnapshot, String> {
    let application = state
        .application
        .lock()
        .map_err(|_| "el estado de la aplicación quedó bloqueado".to_owned())?;
    application
        .as_ref()
        .map(summarize)
        .ok_or_else(|| "primero abre un proyecto".to_owned())
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
    Ok(summarize(application))
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
    Ok(summarize(application))
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
    application
        .execute(DomainCommand::Session(session_command), CommandAuthor::User)
        .map_err(|e| e.to_string())?;
    Ok(summarize(application))
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
    Ok(summarize(application))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(DesktopState {
            application: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            open_project,
            project_snapshot,
            save_project,
            save_project_as,
            set_transport,
            history_action
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
        let value = serde_json::to_value(summarize(&application))
            .expect("el snapshot compacto debe serializarse");

        assert_eq!(value["projectId"], "ui-contract-test");
        assert_eq!(value["tempoBpm"], 96.0);
        assert_eq!(value["trackCount"], 0);
        assert_eq!(value["audioEngineConnected"], false);
        assert!(value.get("samples").is_none());
        assert!(value.get("pcm").is_none());
        assert!(value.get("gpu_buffers").is_none());
    }
}
