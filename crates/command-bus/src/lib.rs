//! Bus de comandos de dominio para UI, CLI, scripting, MIDI y agentes.
//!
//! Esta crate pertenece al Portable Domain: coordina proyecto y sesión sin
//! conocer PipeWire, ALSA, ffmpeg, GPU ni widgets. Cada comando está versionado,
//! atribuido y puede declarar precondiciones antes de mutar el estado.

use estudio_daw_project_model::{
    add_audio_clip, set_audio_clip_fades, set_audio_clip_gain, trim_audio_clip, Project,
    ProjectEvent, ProjectHistory, ProjectSnapshot,
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
        Self {
            session: Session::default(),
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
    use estudio_daw_project_model::{AudioClip, ImportProvenance, TimeSignature, Transport};

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
            tracks: Vec::new(),
            midi_clips: Vec::new(),
            audio_clips: vec![AudioClip {
                id: "clip-1".into(),
                name: "Audio".into(),
                track_id: "track-1".into(),
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
}
