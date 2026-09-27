//! Bus de comandos de dominio para UI, CLI, scripting, MIDI y agentes.
//!
//! Esta crate pertenece al Portable Domain: coordina proyecto y sesión sin
//! conocer PipeWire, ALSA, ffmpeg, GPU ni widgets. Cada comando está versionado,
//! atribuido y puede declarar precondiciones antes de mutar el estado.

use estudio_daw_midi_engine::MidiTake;
use estudio_daw_project_model::{
    add_audio_clip, attach_media_source, attach_midi_take, quantize_midi_clip,
    set_audio_clip_fades, set_audio_clip_gain, trim_audio_clip, MediaSource, Project, ProjectEvent,
    ProjectHistory, ProjectSnapshot, Track, TrackMixerState, TrackRole,
};
use estudio_daw_session::{Session, SessionCommand, TransportSnapshot, TransportState};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use thiserror::Error;

pub const COMMAND_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommandAuthor {
    User,
    Midi,
    Script,
    Agent,
    Import,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandMetadata {
    pub id: String,
    pub version: u32,
    pub author: CommandAuthor,
    pub expected_project_revision: Option<u64>,
    pub expected_transport_state: Option<TransportState>,
}

impl CommandMetadata {
    pub fn user(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: COMMAND_SCHEMA_VERSION,
            author: CommandAuthor::User,
            expected_project_revision: None,
            expected_transport_state: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProjectCommand {
    AddTrack {
        track: Track,
        index: Option<usize>,
    },
    RenameTrack {
        track_id: String,
        name: String,
    },
    MoveTrack {
        track_id: String,
        index: usize,
    },
    RemoveTrack {
        track_id: String,
    },
    SetTrackMixer {
        track_id: String,
        mixer: TrackMixerState,
    },
    AddAudioClip {
        track_id: String,
        name: String,
        start_tick: u64,
        source_start_samples: u64,
        duration_samples: u64,
        sample_rate: u32,
        channels: u16,
    },
    TrimAudioClip {
        clip_id: String,
        source_start_samples: u64,
        duration_samples: u64,
    },
    SetAudioClipGain {
        clip_id: String,
        gain_db: f32,
    },
    SetAudioClipFades {
        clip_id: String,
        fade_in_samples: u64,
        fade_out_samples: u64,
    },
    AttachMediaSource {
        track_id: String,
        source: MediaSource,
    },
    AttachMidiTake {
        take: MidiTake,
        name: String,
    },
    QuantizeMidiClip {
        clip_id: String,
        grid_ticks: u64,
    },
    Undo,
    Redo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "scope", content = "command", rename_all = "snake_case")]
pub enum DomainCommand {
    Session(SessionCommand),
    Project(ProjectCommand),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandEnvelope {
    pub metadata: CommandMetadata,
    pub command: DomainCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainSnapshot {
    pub transport: TransportSnapshot,
    pub project: ProjectSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum DomainEventPayload {
    TransportChanged(TransportSnapshot),
    ProjectChanged(ProjectEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainEvent {
    pub command_id: String,
    pub author: CommandAuthor,
    pub payload: DomainEventPayload,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CommandError {
    #[error("el comando no tiene identificador")]
    EmptyId,
    #[error("versión de comando no soportada: {0}")]
    UnsupportedVersion(u32),
    #[error("conflicto de revisión: esperada={expected}, actual={actual}")]
    ProjectRevisionConflict { expected: u64, actual: u64 },
    #[error("conflicto de transporte: esperado={expected:?}, actual={actual:?}")]
    TransportStateConflict {
        expected: TransportState,
        actual: TransportState,
    },
    #[error("el comando de proyecto falló: {0}")]
    Project(String),
    #[error("no hay cambios para deshacer")]
    NothingToUndo,
    #[error("no hay cambios para rehacer")]
    NothingToRedo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandDiagnostic {
    pub command_id: String,
    pub code: &'static str,
    pub message: String,
}

impl CommandDiagnostic {
    fn from_error(command_id: String, error: CommandError) -> Self {
        let code = match error {
            CommandError::EmptyId => "empty_id",
            CommandError::UnsupportedVersion(_) => "unsupported_version",
            CommandError::ProjectRevisionConflict { .. } => "project_revision_conflict",
            CommandError::TransportStateConflict { .. } => "transport_state_conflict",
            CommandError::Project(_) => "project_command_failed",
            CommandError::NothingToUndo => "nothing_to_undo",
            CommandError::NothingToRedo => "nothing_to_redo",
        };
        Self {
            command_id,
            code,
            message: error.to_string(),
        }
    }
}

pub struct CommandRuntime {
    session: Session,
    project_history: ProjectHistory,
    events: Vec<DomainEvent>,
}

impl CommandRuntime {
    pub fn new(project: Project) -> Self {
        // El tempo vive en el modelo persistente; iniciar siempre a 120 BPM
        // haría que la primera sesión abierta ignorara la tonalidad temporal.
        let mut session = Session::default();
        session.apply(SessionCommand::SetTempo(project.transport.tempo_bpm));
        Self {
            session,
            project_history: ProjectHistory::new(project),
            events: Vec::new(),
        }
    }

    pub fn snapshot(&self) -> DomainSnapshot {
        DomainSnapshot {
            transport: self.session.snapshot(),
            project: self
                .project_history
                .snapshot()
                .expect("el runtime siempre contiene un proyecto"),
        }
    }

    /// Permite a la UI habilitar/deshabilitar Undo sin inspeccionar el historial.
    pub fn can_undo(&self) -> bool {
        self.project_history.can_undo()
    }

    /// Permite a la UI habilitar/deshabilitar Redo sin inspeccionar el historial.
    pub fn can_redo(&self) -> bool {
        self.project_history.can_redo()
    }

    pub fn drain_events(&mut self) -> Vec<DomainEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn apply(&mut self, envelope: CommandEnvelope) -> Result<(), CommandError> {
        self.validate(&envelope.metadata)?;
        let CommandEnvelope { metadata, command } = envelope;
        match command {
            DomainCommand::Session(command) => {
                self.session.apply(command);
                self.events.push(DomainEvent {
                    command_id: metadata.id,
                    author: metadata.author,
                    payload: DomainEventPayload::TransportChanged(self.session.snapshot()),
                });
            }
            DomainCommand::Project(command) => {
                self.apply_project_command(command)?;
                self.events.extend(
                    self.project_history
                        .drain_events()
                        .into_iter()
                        .map(|event| DomainEvent {
                            command_id: metadata.id.clone(),
                            author: metadata.author.clone(),
                            payload: DomainEventPayload::ProjectChanged(event),
                        }),
                );
            }
        }
        Ok(())
    }

    fn validate(&self, metadata: &CommandMetadata) -> Result<(), CommandError> {
        if metadata.id.trim().is_empty() {
            return Err(CommandError::EmptyId);
        }
        if metadata.version != COMMAND_SCHEMA_VERSION {
            return Err(CommandError::UnsupportedVersion(metadata.version));
        }
        let snapshot = self.snapshot();
        if let Some(expected) = metadata.expected_project_revision {
            if expected != snapshot.project.revision {
                return Err(CommandError::ProjectRevisionConflict {
                    expected,
                    actual: snapshot.project.revision,
                });
            }
        }
        if let Some(expected) = metadata.expected_transport_state {
            if expected != snapshot.transport.state {
                return Err(CommandError::TransportStateConflict {
                    expected,
                    actual: snapshot.transport.state,
                });
            }
        }
        Ok(())
    }

    fn apply_project_command(&mut self, command: ProjectCommand) -> Result<(), CommandError> {
        match command {
            ProjectCommand::AddTrack { track, index } => self
                .project_history
                .transact("add track", |project| -> Result<(), String> {
                    track.validate().map_err(|error| error.to_string())?;
                    if project.tracks.iter().any(|item| item.id == track.id) {
                        return Err(format!("track id already exists: {}", track.id));
                    }
                    if track.role == TrackRole::Master
                        && project
                            .tracks
                            .iter()
                            .any(|item| item.role == TrackRole::Master)
                    {
                        return Err(String::from("project already has a master track"));
                    }
                    let insert_at = index
                        .unwrap_or(project.tracks.len())
                        .min(project.tracks.len());
                    project.tracks.insert(insert_at, track);
                    Ok(())
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::RenameTrack { track_id, name } => self
                .project_history
                .transact("rename track", |project| -> Result<(), String> {
                    if name.trim().is_empty() {
                        return Err(String::from("track name must not be empty"));
                    }
                    let track = project
                        .tracks
                        .iter_mut()
                        .find(|item| item.id == track_id)
                        .ok_or_else(|| format!("unknown track: {track_id}"))?;
                    track.name = name;
                    Ok(())
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::MoveTrack { track_id, index } => self
                .project_history
                .transact("move track", |project| -> Result<(), String> {
                    let from = project
                        .tracks
                        .iter()
                        .position(|item| item.id == track_id)
                        .ok_or_else(|| format!("unknown track: {track_id}"))?;
                    let track = project.tracks.remove(from);
                    project
                        .tracks
                        .insert(index.min(project.tracks.len()), track);
                    Ok(())
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::RemoveTrack { track_id } => self
                .project_history
                .transact("remove track", |project| -> Result<(), String> {
                    let before = project.tracks.len();
                    project.tracks.retain(|item| item.id != track_id);
                    if project.tracks.len() == before {
                        return Err(format!("unknown track: {track_id}"));
                    }
                    // Clips are project-owned; deleting a track removes its regions, never source files.
                    project.midi_clips.retain(|clip| clip.track_id != track_id);
                    project.audio_clips.retain(|clip| clip.track_id != track_id);
                    Ok(())
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::SetTrackMixer { track_id, mixer } => self
                .project_history
                .transact("set track mixer", |project| -> Result<(), String> {
                    if !mixer.gain_db.is_finite() || !(-60.0..=12.0).contains(&mixer.gain_db) {
                        return Err(String::from("track gain must be between -60 and +12 dB"));
                    }
                    if !mixer.pan.is_finite() || !(-1.0..=1.0).contains(&mixer.pan) {
                        return Err(String::from("track pan must be between -1 and +1"));
                    }
                    let track = project
                        .tracks
                        .iter_mut()
                        .find(|item| item.id == track_id)
                        .ok_or_else(|| format!("unknown track: {track_id}"))?;
                    track.mixer = mixer;
                    Ok(())
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::AddAudioClip {
                track_id,
                name,
                start_tick,
                source_start_samples,
                duration_samples,
                sample_rate,
                channels,
            } => self
                .project_history
                .transact("add audio clip", |project| {
                    add_audio_clip(
                        project,
                        &track_id,
                        name,
                        start_tick,
                        source_start_samples,
                        duration_samples,
                        sample_rate,
                        channels,
                    )
                    .map(|_| ())
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::TrimAudioClip {
                clip_id,
                source_start_samples,
                duration_samples,
            } => self
                .project_history
                .transact("trim audio clip", |project| {
                    trim_audio_clip(project, &clip_id, source_start_samples, duration_samples)
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::SetAudioClipGain { clip_id, gain_db } => self
                .project_history
                .transact("set audio clip gain", |project| {
                    set_audio_clip_gain(project, &clip_id, gain_db)
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::SetAudioClipFades {
                clip_id,
                fade_in_samples,
                fade_out_samples,
            } => self
                .project_history
                .transact("set audio clip fades", |project| {
                    set_audio_clip_fades(project, &clip_id, fade_in_samples, fade_out_samples)
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::AttachMediaSource { track_id, source } => self
                .project_history
                .transact("attach media source", |project| {
                    attach_media_source(project, &track_id, source)
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::AttachMidiTake { take, name } => self
                .project_history
                .transact("attach MIDI take", |project| {
                    attach_midi_take(project, take, name).map(|_| ())
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::QuantizeMidiClip {
                clip_id,
                grid_ticks,
            } => self
                .project_history
                .transact("quantize MIDI clip", |project| {
                    quantize_midi_clip(project, &clip_id, grid_ticks).map(|_| ())
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::Undo => {
                self.project_history
                    .undo()
                    .ok_or(CommandError::NothingToUndo)?;
            }
            ProjectCommand::Redo => {
                self.project_history
                    .redo()
                    .ok_or(CommandError::NothingToRedo)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CommandBusError {
    #[error("el bus de comandos está lleno")]
    Full,
    #[error("el bus de comandos está cerrado")]
    Closed,
}

/// Cola de entrada para comandos del dominio completo.
///
/// Es deliberadamente distinta de `estudio_daw_session::CommandBus`: aquella
/// es la cola mínima y vigente del transporte live; ésta orquesta proyecto,
/// sesión, precondiciones, eventos y undo. Mantener los nombres distintos en
/// código consumidor evita fusionar prematuramente ambos contratos.
pub struct DomainCommandBus {
    sender: SyncSender<CommandEnvelope>,
    receiver: Receiver<CommandEnvelope>,
}

#[derive(Debug, Default)]
pub struct DrainReport {
    pub applied: usize,
    pub rejected: Vec<CommandDiagnostic>,
}

impl DomainCommandBus {
    pub fn bounded(capacity: usize) -> Self {
        let (sender, receiver) = mpsc::sync_channel(capacity.max(1));
        Self { sender, receiver }
    }

    pub fn dispatch(&self, command: CommandEnvelope) -> Result<(), CommandBusError> {
        self.sender.try_send(command).map_err(|error| match error {
            TrySendError::Full(_) => CommandBusError::Full,
            TrySendError::Disconnected(_) => CommandBusError::Closed,
        })
    }

    pub fn drain_into(&self, runtime: &mut CommandRuntime) -> DrainReport {
        let mut report = DrainReport::default();
        for envelope in self.receiver.try_iter() {
            let command_id = envelope.metadata.id.clone();
            match runtime.apply(envelope) {
                Ok(()) => report.applied += 1,
                Err(error) => report
                    .rejected
                    .push(CommandDiagnostic::from_error(command_id, error)),
            }
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use estudio_daw_midi_engine::{MidiSource, RecordedMidiEvent, RecordedMidiMessage};
    use estudio_daw_project_model::{
        AudioClip, ImportProvenance, InstrumentConfig, MidiClip, TimeSignature, Track,
        TrackChannelConfig, TrackKind, TrackMixerState, TrackRole, Transport,
    };

    fn project() -> Project {
        Project {
            schema_version: "test".into(),
            project_id: "command-bus-test".into(),
            transport: Transport {
                tempo_bpm: 120.0,
                time_signature: TimeSignature {
                    numerator: 4,
                    denominator: 4,
                },
            },
            tracks: vec![
                Track {
                    id: "track-midi".into(),
                    name: "MIDI".into(),
                    kind: TrackKind::Midi,
                    role: TrackRole::Instrument,
                    channel_config: TrackChannelConfig::default(),
                    color: "#58a6b8".into(),
                    mixer: TrackMixerState::default(),
                    notes: Vec::new(),
                    audio_channels: None,
                    media_source: None,
                    instrument: Some(InstrumentConfig::Sine),
                },
                Track {
                    id: "track-audio".into(),
                    name: "Audio".into(),
                    kind: TrackKind::Audio,
                    role: TrackRole::Audio,
                    channel_config: TrackChannelConfig {
                        input_channels: Some(2),
                        output_channels: 2,
                    },
                    color: "#58a6b8".into(),
                    mixer: TrackMixerState::default(),
                    notes: Vec::new(),
                    audio_channels: Some(2),
                    media_source: None,
                    instrument: None,
                },
            ],
            audio_sources: Vec::new(),
            audio_playlists: Vec::new(),
            scenes: Vec::new(),
            clip_slots: Vec::new(),
            midi_clips: vec![MidiClip {
                id: "midi-clip-1".into(),
                name: "Take".into(),
                track_id: "track-midi".into(),
                start_tick: 0,
                duration_ticks: 800,
                take: MidiTake {
                    ppq: 960,
                    tempo_bpm: 120,
                    duration_micros: 1_000_000,
                    events: vec![
                        RecordedMidiEvent {
                            tick: 500,
                            micros_since_start: 260_000,
                            source: MidiSource { client: 1, port: 0 },
                            message: RecordedMidiMessage::NoteOn {
                                channel: 0,
                                note: 64,
                                velocity: 90,
                            },
                        },
                        RecordedMidiEvent {
                            tick: 777,
                            micros_since_start: 400_000,
                            source: MidiSource { client: 1, port: 0 },
                            message: RecordedMidiMessage::ControlChange {
                                channel: 0,
                                controller: 64,
                                value: 127,
                            },
                        },
                    ],
                },
            }],
            audio_clips: vec![AudioClip {
                id: "clip-1".into(),
                name: "Audio".into(),
                track_id: "track-audio".into(),
                source_id: None,
                start_tick: 0,
                source_start_samples: 0,
                duration_samples: 48_000,
                sample_rate: 48_000,
                channels: 2,
                gain_db: 0.0,
                fade_in_samples: 0,
                fade_out_samples: 0,
            }],
            import_provenance: ImportProvenance {
                format: "internal".into(),
                format_version: "1".into(),
                source_file: String::new(),
                warnings: Vec::new(),
            },
        }
    }

    fn envelope(id: &str, command: DomainCommand) -> CommandEnvelope {
        CommandEnvelope {
            metadata: CommandMetadata::user(id),
            command,
        }
    }

    #[test]
    fn one_bus_coordinates_transport_project_and_undo() {
        let bus = DomainCommandBus::bounded(8);
        bus.dispatch(envelope(
            "play-1",
            DomainCommand::Session(SessionCommand::Play),
        ))
        .unwrap();
        bus.dispatch(envelope(
            "gain-1",
            DomainCommand::Project(ProjectCommand::SetAudioClipGain {
                clip_id: "clip-1".into(),
                gain_db: -6.0,
            }),
        ))
        .unwrap();
        bus.dispatch(envelope(
            "undo-1",
            DomainCommand::Project(ProjectCommand::Undo),
        ))
        .unwrap();
        let mut runtime = CommandRuntime::new(project());
        let report = bus.drain_into(&mut runtime);
        assert_eq!(report.applied, 3);
        assert!(report.rejected.is_empty());
        let snapshot = runtime.snapshot();
        assert_eq!(snapshot.transport.state, TransportState::Playing);
        assert_eq!(snapshot.project.project.audio_clips[0].gain_db, 0.0);
        assert_eq!(snapshot.project.revision, 2);
        assert_eq!(runtime.drain_events().len(), 3);
    }

    #[test]
    fn track_commands_are_validated_reversible_and_remove_only_project_regions() {
        let mut runtime = CommandRuntime::new(project());
        let mut track = runtime.snapshot().project.project.tracks[1].clone();
        track.id = "audio-new".into();
        track.name = "Audio 2".into();
        runtime
            .apply(envelope(
                "add-track",
                DomainCommand::Project(ProjectCommand::AddTrack {
                    track,
                    index: Some(0),
                }),
            ))
            .unwrap();
        runtime
            .apply(envelope(
                "mix-track",
                DomainCommand::Project(ProjectCommand::SetTrackMixer {
                    track_id: "audio-new".into(),
                    mixer: TrackMixerState {
                        mute: true,
                        pan: -0.25,
                        ..TrackMixerState::default()
                    },
                }),
            ))
            .unwrap();
        runtime
            .apply(envelope(
                "rename-track",
                DomainCommand::Project(ProjectCommand::RenameTrack {
                    track_id: "audio-new".into(),
                    name: "Percusión".into(),
                }),
            ))
            .unwrap();
        runtime
            .apply(envelope(
                "move-track",
                DomainCommand::Project(ProjectCommand::MoveTrack {
                    track_id: "audio-new".into(),
                    index: 1,
                }),
            ))
            .unwrap();

        let snapshot = runtime.snapshot().project.project;
        let track = snapshot
            .tracks
            .iter()
            .find(|track| track.id == "audio-new")
            .unwrap();
        assert_eq!(track.name, "Percusión");
        assert!(track.mixer.mute);
        assert_eq!(track.mixer.pan, -0.25);
        assert_eq!(snapshot.tracks[1].id, "audio-new");

        assert!(runtime
            .apply(envelope(
                "invalid-pan",
                DomainCommand::Project(ProjectCommand::SetTrackMixer {
                    track_id: "audio-new".into(),
                    mixer: TrackMixerState {
                        pan: 2.0,
                        ..TrackMixerState::default()
                    },
                }),
            ))
            .is_err());
        assert_eq!(
            runtime.snapshot().project.project.tracks[1].mixer.pan,
            -0.25
        );

        runtime
            .apply(envelope(
                "remove-track",
                DomainCommand::Project(ProjectCommand::RemoveTrack {
                    track_id: "track-audio".into(),
                }),
            ))
            .unwrap();
        assert!(runtime.snapshot().project.project.audio_clips.is_empty());
        runtime
            .apply(envelope(
                "undo-remove-track",
                DomainCommand::Project(ProjectCommand::Undo),
            ))
            .unwrap();
        assert_eq!(runtime.snapshot().project.project.audio_clips.len(), 1);
    }

    #[test]
    fn stale_precondition_is_rejected_without_mutating_state() {
        let bus = DomainCommandBus::bounded(2);
        let mut metadata = CommandMetadata::user("stale-gain");
        metadata.expected_project_revision = Some(9);
        bus.dispatch(CommandEnvelope {
            metadata,
            command: DomainCommand::Project(ProjectCommand::SetAudioClipGain {
                clip_id: "clip-1".into(),
                gain_db: -3.0,
            }),
        })
        .unwrap();
        let mut runtime = CommandRuntime::new(project());
        let report = bus.drain_into(&mut runtime);
        assert_eq!(report.applied, 0);
        assert_eq!(report.rejected[0].code, "project_revision_conflict");
        assert_eq!(runtime.snapshot().project.revision, 0);
        assert!(runtime.drain_events().is_empty());
    }

    #[test]
    fn command_envelope_has_a_stable_serializable_contract() {
        let original = envelope(
            "gain-json-1",
            DomainCommand::Project(ProjectCommand::SetAudioClipGain {
                clip_id: "clip-1".into(),
                gain_db: -4.5,
            }),
        );

        let json = serde_json::to_string(&original).unwrap();
        let restored: CommandEnvelope = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, original);
        assert!(json.contains("\"scope\":\"project\""));
        assert!(json.contains("\"type\":\"set_audio_clip_gain\""));
    }

    #[test]
    fn midi_quantization_is_a_reversible_domain_command() {
        let bus = DomainCommandBus::bounded(2);
        bus.dispatch(envelope(
            "quantize-1",
            DomainCommand::Project(ProjectCommand::QuantizeMidiClip {
                clip_id: "midi-clip-1".into(),
                grid_ticks: 120,
            }),
        ))
        .unwrap();
        let mut runtime = CommandRuntime::new(project());

        let report = bus.drain_into(&mut runtime);
        assert_eq!(report.applied, 1);
        assert_eq!(
            runtime.snapshot().project.project.midi_clips[0].take.events[0].tick,
            480
        );
        // La cuantización sólo mueve Note On/Off; los controladores se conservan.
        assert_eq!(
            runtime.snapshot().project.project.midi_clips[0].take.events[1].tick,
            777
        );

        runtime
            .apply(envelope(
                "undo-quantize-1",
                DomainCommand::Project(ProjectCommand::Undo),
            ))
            .unwrap();
        assert_eq!(
            runtime.snapshot().project.project.midi_clips[0].take.events[0].tick,
            500
        );
    }

    #[test]
    fn attach_midi_take_is_serializable_and_emits_an_attributed_event() {
        let mut runtime = CommandRuntime::new(project());
        let take = MidiTake {
            ppq: 960,
            tempo_bpm: 120,
            duration_micros: 500_000,
            events: Vec::new(),
        };
        let command = envelope(
            "attach-take-1",
            DomainCommand::Project(ProjectCommand::AttachMidiTake {
                take,
                name: "Grabación live".into(),
            }),
        );
        let encoded = serde_json::to_string(&command).unwrap();
        let decoded: CommandEnvelope = serde_json::from_str(&encoded).unwrap();

        runtime.apply(decoded).unwrap();
        let snapshot = runtime.snapshot();
        assert_eq!(snapshot.project.project.midi_clips.len(), 2);
        assert_eq!(
            snapshot.project.project.midi_clips[1].name,
            "Grabación live"
        );
        assert!(matches!(
            runtime.drain_events().as_slice(),
            [DomainEvent {
                command_id,
                author: CommandAuthor::User,
                payload: DomainEventPayload::ProjectChanged(_),
            }] if command_id == "attach-take-1"
        ));
    }

    #[test]
    fn media_source_attachment_is_reversible_and_rejects_midi_tracks() {
        let source = MediaSource {
            original_path: "/music/demo.wav".into(),
            original_signature: "size:128;mtime:1".into(),
            original_hash: "sha256:source".into(),
            proxy: None,
        };
        let mut runtime = CommandRuntime::new(project());
        runtime
            .apply(envelope(
                "attach-media-1",
                DomainCommand::Project(ProjectCommand::AttachMediaSource {
                    track_id: "track-audio".into(),
                    source: source.clone(),
                }),
            ))
            .unwrap();

        let attached = runtime.snapshot();
        assert_eq!(attached.project.project.audio_sources.len(), 1);
        assert_eq!(attached.project.project.audio_sources[0].media, source);
        runtime
            .apply(envelope(
                "undo-media-1",
                DomainCommand::Project(ProjectCommand::Undo),
            ))
            .unwrap();
        assert!(runtime.snapshot().project.project.audio_sources.is_empty());

        let bus = DomainCommandBus::bounded(1);
        bus.dispatch(envelope(
            "wrong-track-1",
            DomainCommand::Project(ProjectCommand::AttachMediaSource {
                track_id: "track-midi".into(),
                source,
            }),
        ))
        .unwrap();
        let report = bus.drain_into(&mut runtime);
        assert_eq!(report.applied, 0);
        assert_eq!(report.rejected[0].code, "project_command_failed");
        assert_eq!(
            runtime.snapshot().project.project.tracks[0].media_source,
            None
        );
    }
}
