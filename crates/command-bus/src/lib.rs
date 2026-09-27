//! Bus de comandos de dominio para UI, CLI, scripting, MIDI y agentes.
//!
//! Esta crate pertenece al Portable Domain: coordina proyecto y sesión sin
//! conocer PipeWire, ALSA, ffmpeg, GPU ni widgets. Cada comando está versionado,
//! atribuido y puede declarar precondiciones antes de mutar el estado.

use estudio_daw_midi_types::MidiTake;
use estudio_daw_project_model::{
    add_audio_clip, add_audio_clip_for_source, append_media_source, attach_media_source,
    attach_midi_take, quantize_midi_clip, set_audio_clip_fades, set_audio_clip_gain,
    trim_audio_clip, AudioClip, ClipReference, ClipSlot, MediaSource, Project, ProjectEvent,
    ProjectHistory, ProjectSnapshot, ProxyAsset, Scene, Track, TrackKind, TrackMixerState,
    TrackRole,
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
    DuplicateTrack {
        track_id: String,
        new_track_id: String,
        name: String,
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
    SetTrackActive {
        track_id: String,
        active: bool,
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
    ImportAudio {
        track_id: String,
        name: String,
        source: MediaSource,
        start_tick: u64,
        duration_samples: u64,
        sample_rate: u32,
        channels: u16,
        source_channel_selection: Vec<u16>,
    },
    TrimAudioClip {
        clip_id: String,
        source_start_samples: u64,
        duration_samples: u64,
        start_tick: Option<u64>,
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
    MoveAudioClip {
        clip_id: String,
        start_tick: u64,
    },
    RemoveAudioClip {
        clip_id: String,
    },
    AddScene {
        scene: Scene,
        index: Option<usize>,
    },
    RenameScene {
        scene_id: String,
        name: String,
    },
    MoveScene {
        scene_id: String,
        index: usize,
    },
    RemoveScene {
        scene_id: String,
    },
    SetClipSlot {
        slot: ClipSlot,
    },
    RemoveClipSlot {
        slot_id: String,
    },
    AttachMediaSource {
        track_id: String,
        source: MediaSource,
    },
    SetAudioSourceProxy {
        source_id: String,
        proxy: Option<ProxyAsset>,
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
            ProjectCommand::AddTrack { mut track, index } => self
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
                    let mut master_to_add = None;
                    if track.role == TrackRole::Audio {
                        if track.output_track_id.is_none() {
                            let master = project
                                .tracks
                                .iter()
                                .find(|item| item.role == TrackRole::Master)
                                .cloned();
                            let master = match master {
                                Some(master) => master,
                                None => {
                                    let base_id = format!("master-{}", track.id);
                                    let mut id = base_id.clone();
                                    let mut suffix = 2;
                                    while project.tracks.iter().any(|item| item.id == id)
                                        || id == track.id
                                    {
                                        id = format!("{base_id}-{suffix}");
                                        suffix += 1;
                                    }
                                    let master = Track::new(
                                        id,
                                        "Master",
                                        TrackKind::Audio,
                                        TrackRole::Master,
                                    )
                                    .map_err(|error| error.to_string())?;
                                    master_to_add = Some(master.clone());
                                    master
                                }
                            };
                            track.output_track_id = Some(master.id);
                        }
                        project
                            .audio_playlists
                            .push(estudio_daw_project_model::AudioPlaylist {
                                id: format!("playlist-{}", track.id),
                                track_id: track.id.clone(),
                                region_ids: Vec::new(),
                            });
                    }
                    project.tracks.insert(insert_at, track);
                    if let Some(master) = master_to_add {
                        project.tracks.push(master);
                    }
                    project
                        .validate_persisted_contracts()
                        .map_err(|error| error.to_string())
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::DuplicateTrack {
                track_id,
                new_track_id,
                name,
                index,
            } => {
                self.project_history
                    .transact("duplicate track", |project| -> Result<(), String> {
                        let original = project
                            .tracks
                            .iter()
                            .find(|track| track.id == track_id)
                            .cloned()
                            .ok_or_else(|| format!("unknown track: {track_id}"))?;
                        if new_track_id.trim().is_empty()
                            || project.tracks.iter().any(|track| track.id == new_track_id)
                        {
                            return Err(format!("duplicate or empty track id: {new_track_id}"));
                        }
                        if name.trim().is_empty() {
                            return Err(String::from("track name must not be empty"));
                        }
                        if original.role == TrackRole::Master {
                            return Err(String::from("the master track cannot be duplicated"));
                        }

                        let mut duplicated = original.clone();
                        duplicated.id = new_track_id.clone();
                        duplicated.name = name;
                        duplicated.media_source = None;

                        let mut source_ids = std::collections::HashMap::new();
                        let mut sources = Vec::new();
                        for source in project
                            .audio_sources
                            .iter()
                            .filter(|source| source.owner_track_id == track_id)
                        {
                            let mut copy = source.clone();
                            copy.id = format!("{}-copy-{new_track_id}", source.id);
                            copy.owner_track_id = new_track_id.clone();
                            if project.audio_sources.iter().any(|item| item.id == copy.id) {
                                return Err(format!("duplicate audio source id: {}", copy.id));
                            }
                            source_ids.insert(source.id.clone(), copy.id.clone());
                            sources.push(copy);
                        }
                        project.audio_sources.extend(sources);

                        let mut midi_clip_ids = std::collections::HashMap::new();
                        let mut midi_copies = Vec::new();
                        for clip in project
                            .midi_clips
                            .iter()
                            .filter(|clip| clip.track_id == track_id)
                        {
                            let mut copy = clip.clone();
                            copy.id = format!("{}-copy-{new_track_id}", clip.id);
                            if project.midi_clips.iter().any(|item| item.id == copy.id) {
                                return Err(format!("duplicate MIDI clip id: {}", copy.id));
                            }
                            midi_clip_ids.insert(clip.id.clone(), copy.id.clone());
                            copy.track_id = new_track_id.clone();
                            midi_copies.push(copy);
                        }
                        project.midi_clips.extend(midi_copies);

                        let mut audio_clip_ids = std::collections::HashMap::new();
                        let mut audio_copies: Vec<AudioClip> = Vec::new();
                        for clip in project
                            .audio_clips
                            .iter()
                            .filter(|clip| clip.track_id == track_id)
                        {
                            let mut copy = clip.clone();
                            copy.id = format!("{}-copy-{new_track_id}", clip.id);
                            if project.audio_clips.iter().any(|item| item.id == copy.id) {
                                return Err(format!("duplicate audio clip id: {}", copy.id));
                            }
                            audio_clip_ids.insert(clip.id.clone(), copy.id.clone());
                            copy.track_id = new_track_id.clone();
                            copy.source_id = match copy.source_id.as_ref() {
                                Some(id) => Some(source_ids.get(id).cloned().ok_or_else(|| {
                                    String::from("audio source was not duplicated")
                                })?),
                                None => None,
                            };
                            audio_copies.push(copy);
                        }
                        project.audio_clips.extend(audio_copies);

                        if let Some(playlist) = project
                            .audio_playlists
                            .iter()
                            .find(|item| item.track_id == track_id)
                            .cloned()
                        {
                            let mut copy = playlist;
                            copy.id = format!("playlist-{new_track_id}");
                            copy.track_id = new_track_id.clone();
                            copy.region_ids = copy
                                .region_ids
                                .iter()
                                .filter_map(|id| audio_clip_ids.get(id).cloned())
                                .collect();
                            project.audio_playlists.push(copy);
                        } else if original.role == TrackRole::Audio {
                            project.audio_playlists.push(
                                estudio_daw_project_model::AudioPlaylist {
                                    id: format!("playlist-{new_track_id}"),
                                    track_id: new_track_id.clone(),
                                    region_ids: Vec::new(),
                                },
                            );
                        }

                        let mut slot_copies = Vec::new();
                        for slot in project
                            .clip_slots
                            .iter()
                            .filter(|slot| slot.track_id == track_id)
                        {
                            let mut copy = slot.clone();
                            copy.id = format!("{}-copy-{new_track_id}", slot.id);
                            copy.track_id = new_track_id.clone();
                            copy.clip = match copy.clip {
                                Some(ClipReference::Midi(id)) => Some(ClipReference::Midi(
                                    midi_clip_ids.get(&id).cloned().ok_or_else(|| {
                                        String::from("slot MIDI clip was not duplicated")
                                    })?,
                                )),
                                Some(ClipReference::Audio(id)) => Some(ClipReference::Audio(
                                    audio_clip_ids.get(&id).cloned().ok_or_else(|| {
                                        String::from("slot audio clip was not duplicated")
                                    })?,
                                )),
                                None => None,
                            };
                            slot_copies.push(copy);
                        }
                        project.clip_slots.extend(slot_copies);

                        let insert_at = index
                            .unwrap_or(project.tracks.len())
                            .min(project.tracks.len());
                        project.tracks.insert(insert_at, duplicated);
                        project
                            .validate_persisted_contracts()
                            .map_err(|error| error.to_string())
                    })
                    .map_err(|error| CommandError::Project(error.to_string()))?
            }
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
                    project
                        .validate_persisted_contracts()
                        .map_err(|error| error.to_string())
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::SetTrackActive { track_id, active } => self
                .project_history
                .transact("set track active", |project| -> Result<(), String> {
                    let track = project
                        .tracks
                        .iter_mut()
                        .find(|track| track.id == track_id)
                        .ok_or_else(|| format!("unknown track: {track_id}"))?;
                    track.mixer.active = active;
                    project
                        .validate_persisted_contracts()
                        .map_err(|error| error.to_string())
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
                    project
                        .audio_sources
                        .retain(|source| source.owner_track_id != track_id);
                    project
                        .audio_playlists
                        .retain(|playlist| playlist.track_id != track_id);
                    project.clip_slots.retain(|slot| slot.track_id != track_id);
                    project
                        .validate_persisted_contracts()
                        .map_err(|error| error.to_string())
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
            ProjectCommand::ImportAudio {
                track_id,
                name,
                source,
                start_tick,
                duration_samples,
                sample_rate,
                channels,
                source_channel_selection,
            } => self
                .project_history
                .transact("import audio", |project| {
                    if project.audio_sources.iter().any(|item| {
                        item.owner_track_id == track_id
                            && item.media.original_hash == source.original_hash
                    }) {
                        return Err(String::from("esta fuente ya está importada en la pista"));
                    }
                    let source_id = append_media_source(project, &track_id, source)
                        .map_err(|error| error.to_string())?;
                    let clip_id = add_audio_clip_for_source(
                        project,
                        &track_id,
                        &source_id,
                        name,
                        start_tick,
                        0,
                        duration_samples,
                        sample_rate,
                        channels,
                    )
                    .map_err(|error| error.to_string())?;
                    let clip = project
                        .audio_clips
                        .iter_mut()
                        .find(|clip| clip.id == clip_id)
                        .ok_or_else(|| String::from("la región importada no quedó disponible"))?;
                    if source_channel_selection
                        .iter()
                        .any(|channel| *channel >= channels)
                        || source_channel_selection.len() > 2
                    {
                        return Err(String::from(
                            "la selección de canales de la fuente no es válida",
                        ));
                    }
                    clip.source_channel_selection = source_channel_selection;
                    Ok(())
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::TrimAudioClip {
                clip_id,
                source_start_samples,
                duration_samples,
                start_tick,
            } => self
                .project_history
                .transact("trim audio clip", |project| -> Result<(), String> {
                    trim_audio_clip(project, &clip_id, source_start_samples, duration_samples)
                        .map_err(|error| error.to_string())?;
                    if let Some(start_tick) = start_tick {
                        let clip = project
                            .audio_clips
                            .iter_mut()
                            .find(|clip| clip.id == clip_id)
                            .ok_or_else(|| format!("unknown audio clip: {clip_id}"))?;
                        clip.start_tick = start_tick;
                    }
                    project
                        .validate_persisted_contracts()
                        .map_err(|error| error.to_string())
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
            ProjectCommand::MoveAudioClip {
                clip_id,
                start_tick,
            } => self
                .project_history
                .transact("move audio clip", |project| -> Result<(), String> {
                    let clip = project
                        .audio_clips
                        .iter_mut()
                        .find(|clip| clip.id == clip_id)
                        .ok_or_else(|| format!("unknown audio clip: {clip_id}"))?;
                    clip.start_tick = start_tick;
                    project
                        .validate_persisted_contracts()
                        .map_err(|error| error.to_string())
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::RemoveAudioClip { clip_id } => self
                .project_history
                .transact("remove audio clip", |project| -> Result<(), String> {
                    let index = project
                        .audio_clips
                        .iter()
                        .position(|clip| clip.id == clip_id)
                        .ok_or_else(|| format!("unknown audio clip: {clip_id}"))?;
                    let clip = project.audio_clips.remove(index);
                    for playlist in &mut project.audio_playlists {
                        if playlist.track_id == clip.track_id {
                            playlist
                                .region_ids
                                .retain(|region_id| region_id != &clip_id);
                        }
                    }
                    project
                        .validate_persisted_contracts()
                        .map_err(|error| error.to_string())
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::AddScene { scene, index } => self
                .project_history
                .transact("add scene", |project| -> Result<(), String> {
                    if scene.id.trim().is_empty() || scene.name.trim().is_empty() {
                        return Err(String::from("scene id and name must not be empty"));
                    }
                    if project.scenes.iter().any(|item| item.id == scene.id) {
                        return Err(format!("scene id already exists: {}", scene.id));
                    }
                    let insert_at = index
                        .unwrap_or(project.scenes.len())
                        .min(project.scenes.len());
                    project.scenes.insert(insert_at, scene);
                    project
                        .validate_persisted_contracts()
                        .map_err(|error| error.to_string())
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::RenameScene { scene_id, name } => self
                .project_history
                .transact("rename scene", |project| -> Result<(), String> {
                    if name.trim().is_empty() {
                        return Err(String::from("scene name must not be empty"));
                    }
                    let scene = project
                        .scenes
                        .iter_mut()
                        .find(|scene| scene.id == scene_id)
                        .ok_or_else(|| format!("unknown scene: {scene_id}"))?;
                    scene.name = name;
                    project
                        .validate_persisted_contracts()
                        .map_err(|error| error.to_string())
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::MoveScene { scene_id, index } => self
                .project_history
                .transact("move scene", |project| -> Result<(), String> {
                    let from = project
                        .scenes
                        .iter()
                        .position(|scene| scene.id == scene_id)
                        .ok_or_else(|| format!("unknown scene: {scene_id}"))?;
                    let scene = project.scenes.remove(from);
                    project
                        .scenes
                        .insert(index.min(project.scenes.len()), scene);
                    project
                        .validate_persisted_contracts()
                        .map_err(|error| error.to_string())
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::RemoveScene { scene_id } => self
                .project_history
                .transact("remove scene", |project| -> Result<(), String> {
                    let before = project.scenes.len();
                    project.scenes.retain(|scene| scene.id != scene_id);
                    if project.scenes.len() == before {
                        return Err(format!("unknown scene: {scene_id}"));
                    }
                    project.clip_slots.retain(|slot| slot.scene_id != scene_id);
                    project
                        .validate_persisted_contracts()
                        .map_err(|error| error.to_string())
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::SetClipSlot { slot } => self
                .project_history
                .transact("set clip slot", |project| -> Result<(), String> {
                    if let Some(existing) = project.clip_slots.iter().find(|existing| {
                        existing.scene_id == slot.scene_id
                            && existing.track_id == slot.track_id
                            && existing.id != slot.id
                    }) {
                        return Err(format!(
                            "scene/track slot already has another identity: {}",
                            existing.id
                        ));
                    }
                    if let Some(existing) = project
                        .clip_slots
                        .iter_mut()
                        .find(|existing| existing.id == slot.id)
                    {
                        *existing = slot;
                    } else {
                        project.clip_slots.push(slot);
                    }
                    project
                        .validate_persisted_contracts()
                        .map_err(|error| error.to_string())
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::RemoveClipSlot { slot_id } => self
                .project_history
                .transact("remove clip slot", |project| -> Result<(), String> {
                    let before = project.clip_slots.len();
                    project.clip_slots.retain(|slot| slot.id != slot_id);
                    if project.clip_slots.len() == before {
                        return Err(format!("unknown clip slot: {slot_id}"));
                    }
                    project
                        .validate_persisted_contracts()
                        .map_err(|error| error.to_string())
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::AttachMediaSource { track_id, source } => self
                .project_history
                .transact("attach media source", |project| {
                    attach_media_source(project, &track_id, source)
                })
                .map_err(|error| CommandError::Project(error.to_string()))?,
            ProjectCommand::SetAudioSourceProxy { source_id, proxy } => self
                .project_history
                .transact("set audio source proxy", |project| -> Result<(), String> {
                    let source = project
                        .audio_sources
                        .iter_mut()
                        .find(|source| source.id == source_id)
                        .ok_or_else(|| format!("unknown audio source: {source_id}"))?;
                    if let Some(asset) = &proxy {
                        if asset.source_signature != source.media.original_signature
                            || asset.source_hash != source.media.original_hash
                        {
                            return Err(format!(
                                "proxy provenance does not match source: {source_id}"
                            ));
                        }
                    }
                    source.media.proxy = proxy;
                    project
                        .validate_persisted_contracts()
                        .map_err(|error| error.to_string())
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
    use estudio_daw_midi_types::{MidiSource, RecordedMidiEvent, RecordedMidiMessage};
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
                    output_track_id: None,
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
                    output_track_id: None,
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
            audio_playlists: vec![estudio_daw_project_model::AudioPlaylist {
                id: "playlist-track-audio".into(),
                track_id: "track-audio".into(),
                region_ids: vec!["clip-1".into()],
            }],
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
                source_channel_selection: Vec::new(),
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
        let restored = runtime.snapshot().project.project;
        assert_eq!(restored.audio_playlists.len(), 2);
        assert!(restored.audio_playlists.iter().any(|playlist| {
            playlist.track_id == "track-audio" && playlist.region_ids == ["clip-1"]
        }));
    }

    #[test]
    fn adding_audio_track_assigns_stereo_channels_and_default_master_output_reversibly() {
        let mut runtime = CommandRuntime::new(project());
        let mut track =
            Track::new("audio-new", "Audio 2", TrackKind::Audio, TrackRole::Audio).unwrap();
        track.channel_config.input_channels = Some(2);
        runtime
            .apply(envelope(
                "add-audio-track",
                DomainCommand::Project(ProjectCommand::AddTrack { track, index: None }),
            ))
            .unwrap();

        let project_after_add = runtime.snapshot().project.project;
        let audio = project_after_add
            .tracks
            .iter()
            .find(|track| track.id == "audio-new")
            .unwrap();
        let master = project_after_add
            .tracks
            .iter()
            .find(|track| track.role == TrackRole::Master)
            .unwrap();
        assert_eq!(audio.channel_config.input_channels, Some(2));
        assert_eq!(audio.channel_config.output_channels, 2);
        assert_eq!(audio.output_track_id.as_deref(), Some(master.id.as_str()));

        runtime
            .apply(envelope(
                "undo-add-audio-track",
                DomainCommand::Project(ProjectCommand::Undo),
            ))
            .unwrap();
        let undone = runtime.snapshot().project.project;
        assert!(!undone.tracks.iter().any(|track| track.id == "audio-new"));
        assert!(!undone
            .tracks
            .iter()
            .any(|track| track.role == TrackRole::Master));
    }

    #[test]
    fn audio_source_proxy_uses_reversible_command_and_checks_provenance() {
        let mut initial = project();
        initial
            .audio_sources
            .push(estudio_daw_project_model::AudioSource {
                id: "source-1".into(),
                owner_track_id: "track-audio".into(),
                media: MediaSource {
                    original_path: "audio.wav".into(),
                    original_signature: "size:10:mtime:1".into(),
                    original_hash: "sha256:source".into(),
                    proxy: None,
                },
                sample_rate_hz: Some(48_000),
                channels: Some(2),
            });
        let mut runtime = CommandRuntime::new(initial);
        let proxy = ProxyAsset {
            path: "cache/audio.opus".into(),
            source_signature: "size:10:mtime:1".into(),
            source_hash: "sha256:source".into(),
            profile: "preview".into(),
        };
        runtime
            .apply(envelope(
                "set-proxy",
                DomainCommand::Project(ProjectCommand::SetAudioSourceProxy {
                    source_id: "source-1".into(),
                    proxy: Some(proxy.clone()),
                }),
            ))
            .unwrap();
        assert_eq!(
            runtime.snapshot().project.project.audio_sources[0]
                .media
                .proxy,
            Some(proxy.clone())
        );
        runtime
            .apply(envelope(
                "undo-proxy",
                DomainCommand::Project(ProjectCommand::Undo),
            ))
            .unwrap();
        assert_eq!(
            runtime.snapshot().project.project.audio_sources[0]
                .media
                .proxy,
            None
        );

        let before = runtime.snapshot().project.project;
        let error = runtime
            .apply(envelope(
                "bad-proxy",
                DomainCommand::Project(ProjectCommand::SetAudioSourceProxy {
                    source_id: "source-1".into(),
                    proxy: Some(ProxyAsset {
                        source_signature: "otra-firma".into(),
                        ..proxy
                    }),
                }),
            ))
            .unwrap_err();
        assert!(matches!(error, CommandError::Project(_)));
        assert_eq!(runtime.snapshot().project.project, before);
    }

    #[test]
    fn duplicate_track_copies_owned_clips_and_slots_as_one_reversible_edit() {
        let mut initial = project();
        initial.scenes.push(Scene {
            id: "scene-1".into(),
            name: "Verso".into(),
        });
        initial.clip_slots.push(ClipSlot {
            id: "slot-1".into(),
            scene_id: "scene-1".into(),
            track_id: "track-midi".into(),
            clip: Some(ClipReference::Midi("midi-clip-1".into())),
        });
        let mut runtime = CommandRuntime::new(initial.clone());
        let outcome = runtime.apply(envelope(
            "duplicate-midi",
            DomainCommand::Project(ProjectCommand::DuplicateTrack {
                track_id: "track-midi".into(),
                new_track_id: "midi-copy".into(),
                name: "MIDI copia".into(),
                index: Some(1),
            }),
        ));
        assert!(outcome.is_ok(), "duplicar pista MIDI: {outcome:?}");
        let copied = runtime.snapshot().project.project;
        assert_eq!(copied.tracks[1].id, "midi-copy");
        let clip = copied
            .midi_clips
            .iter()
            .find(|clip| clip.track_id == "midi-copy")
            .unwrap();
        assert_ne!(clip.id, "midi-clip-1");
        assert_eq!(clip.take, initial.midi_clips[0].take);
        let slot = copied
            .clip_slots
            .iter()
            .find(|slot| slot.track_id == "midi-copy")
            .unwrap();
        assert_eq!(slot.clip, Some(ClipReference::Midi(clip.id.clone())));
        runtime
            .apply(envelope(
                "undo-duplicate",
                DomainCommand::Project(ProjectCommand::Undo),
            ))
            .unwrap();
        assert_eq!(runtime.snapshot().project.project, initial);
        runtime
            .apply(envelope(
                "redo-duplicate",
                DomainCommand::Project(ProjectCommand::Redo),
            ))
            .unwrap();
        assert!(runtime
            .snapshot()
            .project
            .project
            .tracks
            .iter()
            .any(|track| track.id == "midi-copy"));
    }

    #[test]
    fn scene_slot_track_state_and_audio_position_are_reversible_and_validated() {
        let mut runtime = CommandRuntime::new(project());
        runtime
            .apply(envelope(
                "add-scene",
                DomainCommand::Project(ProjectCommand::AddScene {
                    scene: Scene {
                        id: "scene-1".into(),
                        name: "Verso".into(),
                    },
                    index: None,
                }),
            ))
            .unwrap();
        runtime
            .apply(envelope(
                "set-slot",
                DomainCommand::Project(ProjectCommand::SetClipSlot {
                    slot: ClipSlot {
                        id: "slot-1".into(),
                        scene_id: "scene-1".into(),
                        track_id: "track-midi".into(),
                        clip: Some(ClipReference::Midi("midi-clip-1".into())),
                    },
                }),
            ))
            .unwrap();
        let original_clip = runtime.snapshot().project.project.audio_clips[0].clone();
        runtime
            .apply(envelope(
                "move-audio",
                DomainCommand::Project(ProjectCommand::MoveAudioClip {
                    clip_id: "clip-1".into(),
                    start_tick: 960,
                }),
            ))
            .unwrap();
        runtime
            .apply(envelope(
                "deactivate",
                DomainCommand::Project(ProjectCommand::SetTrackActive {
                    track_id: "track-audio".into(),
                    active: false,
                }),
            ))
            .unwrap();
        let changed = runtime.snapshot().project.project;
        assert_eq!(changed.audio_clips[0].start_tick, 960);
        assert_eq!(
            changed.audio_clips[0].source_start_samples,
            original_clip.source_start_samples
        );
        assert_eq!(
            changed.audio_clips[0].duration_samples,
            original_clip.duration_samples
        );
        assert!(
            !changed
                .tracks
                .iter()
                .find(|track| track.id == "track-audio")
                .unwrap()
                .mixer
                .active
        );
        assert!(runtime
            .apply(envelope(
                "wrong-slot-type",
                DomainCommand::Project(ProjectCommand::SetClipSlot {
                    slot: ClipSlot {
                        id: "slot-wrong".into(),
                        scene_id: "scene-1".into(),
                        track_id: "track-audio".into(),
                        clip: Some(ClipReference::Midi("midi-clip-1".into()))
                    },
                })
            ))
            .is_err());
        for id in ["undo-active", "undo-move", "undo-slot", "undo-scene"] {
            runtime
                .apply(envelope(id, DomainCommand::Project(ProjectCommand::Undo)))
                .unwrap();
        }
        let restored = runtime.snapshot().project.project;
        assert!(restored.scenes.is_empty());
        assert!(restored.clip_slots.is_empty());
        assert_eq!(restored.audio_clips[0], original_clip);
        assert!(
            restored
                .tracks
                .iter()
                .find(|track| track.id == "track-audio")
                .unwrap()
                .mixer
                .active
        );
    }

    #[test]
    fn audio_region_commands_add_trim_gain_and_fades_undo_without_changing_source() {
        let mut runtime = CommandRuntime::new(project());
        let source = MediaSource {
            original_path: "/music/original.wav".into(),
            original_signature: "size:96000;mtime:2".into(),
            original_hash: "sha256:original".into(),
            proxy: None,
        };
        runtime
            .apply(envelope(
                "attach-source",
                DomainCommand::Project(ProjectCommand::AttachMediaSource {
                    track_id: "track-audio".into(),
                    source: source.clone(),
                }),
            ))
            .unwrap();
        runtime
            .apply(envelope(
                "add-region",
                DomainCommand::Project(ProjectCommand::AddAudioClip {
                    track_id: "track-audio".into(),
                    name: "Puente".into(),
                    start_tick: 240,
                    source_start_samples: 0,
                    duration_samples: 48_000,
                    sample_rate: 48_000,
                    channels: 2,
                }),
            ))
            .unwrap();
        runtime
            .apply(envelope(
                "trim-region",
                DomainCommand::Project(ProjectCommand::TrimAudioClip {
                    clip_id: "audio-clip-2".into(),
                    source_start_samples: 4_800,
                    duration_samples: 24_000,
                    start_tick: None,
                }),
            ))
            .unwrap();
        runtime
            .apply(envelope(
                "gain-region",
                DomainCommand::Project(ProjectCommand::SetAudioClipGain {
                    clip_id: "audio-clip-2".into(),
                    gain_db: -6.0,
                }),
            ))
            .unwrap();
        runtime
            .apply(envelope(
                "fade-region",
                DomainCommand::Project(ProjectCommand::SetAudioClipFades {
                    clip_id: "audio-clip-2".into(),
                    fade_in_samples: 2_400,
                    fade_out_samples: 1_200,
                }),
            ))
            .unwrap();

        let edited = runtime.snapshot().project.project;
        let region = edited
            .audio_clips
            .iter()
            .find(|clip| clip.id == "audio-clip-2")
            .unwrap();
        assert_eq!(
            (
                region.start_tick,
                region.source_start_samples,
                region.duration_samples
            ),
            (240, 4_800, 24_000)
        );
        assert_eq!(
            (
                region.gain_db,
                region.fade_in_samples,
                region.fade_out_samples
            ),
            (-6.0, 2_400, 1_200)
        );
        assert_eq!(edited.audio_sources[0].media, source);
        for id in ["undo-fades", "undo-gain", "undo-trim", "undo-add"] {
            runtime
                .apply(envelope(id, DomainCommand::Project(ProjectCommand::Undo)))
                .unwrap();
        }
        let restored = runtime.snapshot().project.project;
        assert_eq!(restored.audio_clips.len(), 1);
        assert_eq!(restored.audio_playlists[0].region_ids, ["clip-1"]);
        assert_eq!(restored.audio_sources[0].media, source);
    }

    #[test]
    fn scenes_and_slots_can_be_edited_removed_and_restored_without_deleting_clips() {
        let mut initial = project();
        initial.scenes = vec![
            Scene {
                id: "scene-a".into(),
                name: "A".into(),
            },
            Scene {
                id: "scene-b".into(),
                name: "B".into(),
            },
        ];
        initial.clip_slots.push(ClipSlot {
            id: "slot-a".into(),
            scene_id: "scene-a".into(),
            track_id: "track-midi".into(),
            clip: Some(ClipReference::Midi("midi-clip-1".into())),
        });
        let mut runtime = CommandRuntime::new(initial);
        runtime
            .apply(envelope(
                "rename-scene",
                DomainCommand::Project(ProjectCommand::RenameScene {
                    scene_id: "scene-a".into(),
                    name: "Intro".into(),
                }),
            ))
            .unwrap();
        runtime
            .apply(envelope(
                "move-scene",
                DomainCommand::Project(ProjectCommand::MoveScene {
                    scene_id: "scene-a".into(),
                    index: 1,
                }),
            ))
            .unwrap();
        runtime
            .apply(envelope(
                "remove-slot",
                DomainCommand::Project(ProjectCommand::RemoveClipSlot {
                    slot_id: "slot-a".into(),
                }),
            ))
            .unwrap();
        runtime
            .apply(envelope(
                "remove-scene",
                DomainCommand::Project(ProjectCommand::RemoveScene {
                    scene_id: "scene-a".into(),
                }),
            ))
            .unwrap();
        let changed = runtime.snapshot().project.project;
        assert_eq!(
            changed.scenes,
            [Scene {
                id: "scene-b".into(),
                name: "B".into()
            }]
        );
        assert!(changed.clip_slots.is_empty());
        assert_eq!(changed.midi_clips.len(), 1);
        for id in [
            "undo-scene-delete",
            "undo-slot-delete",
            "undo-scene-move",
            "undo-scene-rename",
        ] {
            runtime
                .apply(envelope(id, DomainCommand::Project(ProjectCommand::Undo)))
                .unwrap();
        }
        let restored = runtime.snapshot().project.project;
        assert_eq!(restored.scenes[0].name, "A");
        assert_eq!(restored.scenes[0].id, "scene-a");
        assert_eq!(
            restored.clip_slots[0].clip,
            Some(ClipReference::Midi("midi-clip-1".into()))
        );
        assert_eq!(restored.midi_clips.len(), 1);
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

    #[test]
    fn import_audio_adds_source_and_region_atomically_and_undoes_both() {
        let mut runtime = CommandRuntime::new(project());
        let source = MediaSource {
            original_path: "/project/media/voice.wav".into(),
            original_signature: "size:96000;mtime:3".into(),
            original_hash: "sha256:voice".into(),
            proxy: None,
        };
        runtime
            .apply(envelope(
                "import-audio",
                DomainCommand::Project(ProjectCommand::ImportAudio {
                    track_id: "track-audio".into(),
                    name: "voice".into(),
                    source: source.clone(),
                    start_tick: 960,
                    duration_samples: 48_000,
                    sample_rate: 48_000,
                    channels: 2,
                    source_channel_selection: vec![0, 1],
                }),
            ))
            .unwrap();
        let imported = runtime.snapshot().project.project;
        assert_eq!(imported.audio_sources.len(), 1);
        assert_eq!(imported.audio_clips.len(), 2);
        assert_eq!(imported.audio_clips[1].name, "voice");
        assert_eq!(imported.audio_clips[1].start_tick, 960);
        assert_eq!(imported.audio_clips[1].source_channel_selection, vec![0, 1]);
        assert_eq!(
            imported.audio_clips[1].source_id.as_deref(),
            Some("source-track-audio-1")
        );
        assert_eq!(imported.audio_sources[0].media, source);

        runtime
            .apply(envelope(
                "undo-import-audio",
                DomainCommand::Project(ProjectCommand::Undo),
            ))
            .unwrap();
        let restored = runtime.snapshot().project.project;
        assert!(restored.audio_sources.is_empty());
        assert_eq!(restored.audio_clips.len(), 1);
    }
}
