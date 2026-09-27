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
    TrackChannelConfig, TrackKind, TrackMixerState, TrackRole, Transport,
};
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
    color: String,
    active: bool,
    mute: bool,
    solo: bool,
    gain_db: f32,
    pan: f32,
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
    track_count: usize,
    midi_clip_count: usize,
    audio_clip_count: usize,
    beats_per_bar: f64,
    tracks: Vec<TrackSummary>,
    midi_clips: Vec<MidiClipSummary>,
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
            note_count: track.notes.len()
                + project
                    .midi_clips
                    .iter()
                    .filter(|clip| clip.track_id == track.id)
                    .flat_map(|clip| &clip.take.events)
                    .filter(|event| matches!(&event.message, RecordedMidiMessage::NoteOn { velocity, .. } if *velocity > 0))
                    .count(),
            color: track.color.clone(),
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
        beats_per_bar,
        tracks,
        midi_clips,
        can_undo,
        can_redo,
        audio_engine_connected,
    }
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
    let (track_kind, name, instrument) = match kind.as_str() {
        "audio" => (TrackKind::Audio, "Audio", None),
        "midi" => (
            TrackKind::Midi,
            "MIDI",
            Some(estudio_daw_project_model::InstrumentConfig::Sine),
        ),
        _ => return Err(format!("tipo de pista desconocido: {kind}")),
    };
    let index = application.snapshot().project.project.tracks.len();
    let track = estudio_daw_project_model::Track {
        id: format!("track-{}-{index}", unix_timestamp_millis()),
        name: format!("{name} {}", index + 1),
        kind: track_kind,
        role: if kind == "audio" {
            TrackRole::Audio
        } else {
            TrackRole::Instrument
        },
        channel_config: TrackChannelConfig {
            input_channels: if kind == "audio" { Some(2) } else { None },
            output_channels: 2,
        },
        color: "#58a6b8".into(),
        mixer: TrackMixerState::default(),
        notes: Vec::new(),
        audio_channels: if kind == "audio" { Some(2) } else { None },
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
        schema_version: "estudio-daw.project.v4".into(),
        project_id: format!("proyecto-{}", unix_timestamp_millis()),
        transport: Transport {
            tempo_bpm: 120.0,
            time_signature: TimeSignature {
                numerator: 4,
                denominator: 4,
            },
        },
        tracks: vec![Track {
            id: "midi-1".into(),
            name: "MIDI 1".into(),
            kind: TrackKind::Midi,
            role: TrackRole::Instrument,
            channel_config: TrackChannelConfig::default(),
            color: "#58a6b8".into(),
            mixer: TrackMixerState::default(),
            notes: Vec::new(),
            audio_channels: None,
            media_source: None,
            instrument: Some(InstrumentConfig::Sine),
        }],
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
            let active_profile = settings
                .as_ref()
                .map(|settings| settings.active())
                .ok_or_else(|| "faltan preferencias para iniciar audio".to_owned())?;
            if let Err(error) = audio.play(&project, active_profile) {
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
            new_project,
            add_track,
            demo_midi_project,
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
