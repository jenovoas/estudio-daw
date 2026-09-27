//! API de aplicación compartida por la CLI y futuras interfaces.
//!
//! Esta capa mantiene vivo el runtime de comandos durante la sesión de edición,
//! entrega eventos/snapshots a la UI y concentra la carga y guardado del modelo.
//! No conoce dispositivos, widgets ni buffers de audio.

pub use estudio_daw_command_bus::{
    CommandAuthor, CommandEnvelope, CommandMetadata, DomainCommand, DomainEvent,
    DomainEventPayload, DomainSnapshot, ProjectCommand,
};
use estudio_daw_command_bus::{CommandDiagnostic, CommandRuntime, DomainCommandBus};
use estudio_daw_project_model::{load_project_json, Project, ProjectJsonError};
pub use estudio_daw_session::{
    SessionCommand, TransportClock, TransportSnapshot, TransportState, TICKS_PER_QUARTER,
};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use thiserror::Error;

static NEXT_COMMAND_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_TEMP_FILE_ID: AtomicU64 = AtomicU64::new(1);

const COMMAND_QUEUE_CAPACITY: usize = 64;

const AUDIO_SETTINGS_SCHEMA: &str = "audio-runtime-settings.v1";
const DEFAULT_SETTINGS_BACKEND: &str = "pipewire:default";

/// Perfil activo para el próximo stream de audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioProfile {
    LiveRecord,
    MultitrackPlayback,
}

/// Parámetros solicitados; no representan una latencia extremo a extremo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioProfileSettings {
    pub device_period_frames: u32,
    pub playback_safety_frames: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioRuntimeSettings {
    pub schema_version: String,
    pub backend_device_key: String,
    pub active_profile: AudioProfile,
    pub live_record: AudioProfileSettings,
    pub multitrack_playback: AudioProfileSettings,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioRuntimeView {
    pub settings: AudioRuntimeSettings,
    pub sample_rate_hz: u32,
    pub requested_period_frames: u32,
    pub requested_period_ms: f64,
    pub effective_period_frames: Option<u32>,
    pub playback_safety_frames: u32,
    pub playback_safety_ms: f64,
    pub effective_period_available: bool,
    pub engine_connected: bool,
    pub playback_path_isolation_supported: bool,
    pub apply_state: &'static str,
}

impl AudioRuntimeView {
    pub fn from_settings(settings: AudioRuntimeSettings, sample_rate_hz: u32) -> Self {
        let profile = settings.active();
        Self {
            settings,
            sample_rate_hz,
            requested_period_frames: profile.device_period_frames,
            requested_period_ms: frames_to_milliseconds(
                profile.device_period_frames,
                sample_rate_hz,
            )
            .unwrap_or_default(),
            effective_period_frames: None,
            playback_safety_frames: profile.playback_safety_frames,
            playback_safety_ms: frames_to_milliseconds(
                profile.playback_safety_frames,
                sample_rate_hz,
            )
            .unwrap_or_default(),
            effective_period_available: false,
            engine_connected: false,
            playback_path_isolation_supported: false,
            apply_state: "nextStream",
        }
    }
}

pub fn frames_to_milliseconds(frames: u32, sample_rate_hz: u32) -> Option<f64> {
    (sample_rate_hz > 0).then(|| f64::from(frames) * 1_000.0 / f64::from(sample_rate_hz))
}

impl Default for AudioRuntimeSettings {
    fn default() -> Self {
        Self {
            schema_version: AUDIO_SETTINGS_SCHEMA.to_owned(),
            backend_device_key: DEFAULT_SETTINGS_BACKEND.to_owned(),
            active_profile: AudioProfile::LiveRecord,
            live_record: AudioProfileSettings {
                device_period_frames: 256,
                playback_safety_frames: 512,
            },
            multitrack_playback: AudioProfileSettings {
                device_period_frames: 256,
                playback_safety_frames: 1_024,
            },
        }
    }
}

impl AudioRuntimeSettings {
    pub fn active(&self) -> AudioProfileSettings {
        match self.active_profile {
            AudioProfile::LiveRecord => self.live_record,
            AudioProfile::MultitrackPlayback => self.multitrack_playback,
        }
    }

    pub fn validate(&self) -> Result<(), AudioSettingsError> {
        if self.schema_version != AUDIO_SETTINGS_SCHEMA {
            return Err(AudioSettingsError::UnsupportedVersion(
                self.schema_version.clone(),
            ));
        }
        if self.backend_device_key.trim().is_empty()
            || !valid_profile(self.live_record)
            || !valid_profile(self.multitrack_playback)
        {
            return Err(AudioSettingsError::InvalidSettings);
        }
        Ok(())
    }
}

fn valid_profile(profile: AudioProfileSettings) -> bool {
    (16..=2_048).contains(&profile.device_period_frames) && profile.playback_safety_frames <= 2_048
}

#[derive(Debug, Error)]
pub enum AudioSettingsError {
    #[error("falló la operación de archivo de configuración: {0}")]
    Io(#[from] std::io::Error),
    #[error("la configuración de audio no es JSON válido: {0}")]
    Json(#[from] serde_json::Error),
    #[error("versión de configuración de audio no compatible: {0}")]
    UnsupportedVersion(String),
    #[error("la configuración de audio contiene valores inválidos")]
    InvalidSettings,
}

/// Ruta compartida por la UI y los adaptadores CLI para preferencias del usuario.
pub fn audio_runtime_settings_path() -> Result<PathBuf, AudioSettingsError> {
    let config_root = if let Some(path) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(path)
    } else {
        PathBuf::from(std::env::var_os("HOME").ok_or_else(|| {
            AudioSettingsError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "HOME/XDG_CONFIG_HOME no está disponible",
            ))
        })?)
        .join(".config")
    };
    Ok(config_root.join("estudio-daw").join("audio-runtime.json"))
}

pub fn load_audio_runtime_settings() -> Result<AudioRuntimeSettings, AudioSettingsError> {
    load_audio_runtime_settings_from(&audio_runtime_settings_path()?)
}

pub fn load_audio_runtime_settings_from(
    path: &Path,
) -> Result<AudioRuntimeSettings, AudioSettingsError> {
    if !path.exists() {
        return Ok(AudioRuntimeSettings::default());
    }
    let settings: AudioRuntimeSettings = serde_json::from_slice(&fs::read(path)?)?;
    settings.validate()?;
    Ok(settings)
}

pub fn save_audio_runtime_settings(
    settings: &AudioRuntimeSettings,
) -> Result<(), AudioSettingsError> {
    settings.validate()?;
    let path = audio_runtime_settings_path()?;
    save_audio_runtime_settings_to(&path, settings)
}

pub fn save_audio_runtime_settings_to(
    path: &Path,
    settings: &AudioRuntimeSettings,
) -> Result<(), AudioSettingsError> {
    settings.validate()?;
    fs::create_dir_all(path.parent().unwrap_or(Path::new(".")))?;
    atomic_write(path, &serde_json::to_vec_pretty(settings)?)?;
    Ok(())
}

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("falló la operación de archivos: {0}")]
    Io(#[from] std::io::Error),
    #[error("el proyecto JSON no es válido: {0}")]
    Json(#[from] serde_json::Error),
    #[error("no se pudo migrar el proyecto JSON: {0}")]
    ProjectJson(#[from] ProjectJsonError),
    #[error("no se pudo despachar el comando: {0}")]
    Queue(#[from] estudio_daw_command_bus::CommandBusError),
    #[error("comando rechazado ({code}) [{command_id}]: {message}")]
    Rejected {
        command_id: String,
        code: &'static str,
        message: String,
    },
    #[error("el bus aplicó {applied} comandos; se esperaba exactamente uno")]
    UnexpectedApplyCount { applied: usize },
    #[error("la aplicación aún no tiene una ruta de proyecto asociada")]
    NoProjectPath,
}

/// Sesión de aplicación de un proyecto.
///
/// Una instancia debe vivir mientras la sesión permanezca abierta: el runtime
/// conserva el historial undo/redo en memoria y publica snapshots versionados.
/// Abrir un archivo crea un historial nuevo a partir del estado guardado.
pub struct ProjectApplication {
    runtime: CommandRuntime,
    command_bus: DomainCommandBus,
    project_path: Option<PathBuf>,
}

impl ProjectApplication {
    /// Crea una sesión nueva a partir de un modelo ya cargado.
    pub fn new(project: Project) -> Self {
        Self {
            runtime: CommandRuntime::new(project),
            command_bus: DomainCommandBus::bounded(COMMAND_QUEUE_CAPACITY),
            project_path: None,
        }
    }

    /// Abre un `project.json` y establece el estado cargado como base del undo.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ApplicationError> {
        let path = path.as_ref();
        let bytes = fs::read(path)?;
        let project = load_project_json(&bytes)?;
        let mut application = Self::new(project);
        application.project_path = Some(path.to_path_buf());
        Ok(application)
    }

    /// Devuelve un snapshot estable para que UI/scripting lean el estado.
    pub fn snapshot(&self) -> DomainSnapshot {
        self.runtime.snapshot()
    }

    /// Estado de los controles Undo/Redo para vistas de edición.
    pub fn history_state(&self) -> (bool, bool) {
        (self.runtime.can_undo(), self.runtime.can_redo())
    }

    /// Ruta asociada al proyecto abierto, si la sesión se abrió desde archivo.
    pub fn project_path(&self) -> Option<&Path> {
        self.project_path.as_deref()
    }

    /// Despacha un envelope explícito y entrega los eventos causados por él.
    ///
    /// La validación y mutación ocurren fuera del callback RT; la UI recibe los
    /// eventos atribuibles sin tener que acceder a `ProjectHistory` internamente.
    pub fn dispatch(
        &mut self,
        envelope: CommandEnvelope,
    ) -> Result<Vec<DomainEvent>, ApplicationError> {
        self.command_bus.dispatch(envelope)?;
        let report = self.command_bus.drain_into(&mut self.runtime);

        if let Some(diagnostic) = report.rejected.into_iter().next() {
            return Err(rejected(diagnostic));
        }
        if report.applied != 1 {
            return Err(ApplicationError::UnexpectedApplyCount {
                applied: report.applied,
            });
        }

        // Cada llamada despacha un solo comando; drenar aquí evita mezclar
        // eventos entre acciones distintas de UI/CLI.
        Ok(self.runtime.drain_events())
    }

    /// Ejecuta una acción con id generado y revisión actual como precondición.
    pub fn execute(
        &mut self,
        command: DomainCommand,
        author: CommandAuthor,
    ) -> Result<Vec<DomainEvent>, ApplicationError> {
        let id = format!(
            "app-{}-{}",
            std::process::id(),
            NEXT_COMMAND_ID.fetch_add(1, Ordering::Relaxed)
        );
        let mut metadata = CommandMetadata::user(id);
        metadata.author = author;
        metadata.expected_project_revision = Some(self.snapshot().project.revision);
        self.dispatch(CommandEnvelope { metadata, command })
    }

    /// Atajo para ejecutar una mutación del proyecto como acción del usuario.
    pub fn execute_project(
        &mut self,
        command: ProjectCommand,
    ) -> Result<Vec<DomainEvent>, ApplicationError> {
        self.execute(DomainCommand::Project(command), CommandAuthor::User)
    }

    /// Deshace la última transacción del proyecto en esta sesión.
    pub fn undo(&mut self) -> Result<Vec<DomainEvent>, ApplicationError> {
        self.execute_project(ProjectCommand::Undo)
    }

    /// Reaplica la última transacción deshecha en esta sesión.
    pub fn redo(&mut self) -> Result<Vec<DomainEvent>, ApplicationError> {
        self.execute_project(ProjectCommand::Redo)
    }

    /// Guarda una copia consistente del modelo mediante reemplazo atómico.
    ///
    /// El temporal vive junto al destino para que `rename` no cruce sistemas de
    /// archivos; el archivo destino nunca queda parcialmente serializado.
    pub fn save_to(&self, path: impl AsRef<Path>) -> Result<(), ApplicationError> {
        let snapshot = self.snapshot();
        let mut project = snapshot.project.project;
        // El transporte live y el JSON tienen un único tempo compartido.
        project.transport.tempo_bpm = snapshot.transport.tempo_bpm;
        let bytes = serde_json::to_vec_pretty(&project)?;
        atomic_write(path.as_ref(), &bytes)?;
        Ok(())
    }

    /// Guarda sobre la ruta asociada al proyecto abierto.
    pub fn save(&self) -> Result<(), ApplicationError> {
        let path = self
            .project_path
            .as_deref()
            .ok_or(ApplicationError::NoProjectPath)?;
        self.save_to(path)
    }

    /// Guarda en otra ruta y la asocia a la sesión sólo si la escritura termina.
    pub fn save_as(&mut self, path: impl AsRef<Path>) -> Result<(), ApplicationError> {
        let path = path.as_ref().to_path_buf();
        self.save_to(&path)?;
        self.project_path = Some(path);
        Ok(())
    }
}

fn rejected(diagnostic: CommandDiagnostic) -> ApplicationError {
    ApplicationError::Rejected {
        command_id: diagnostic.command_id,
        code: diagnostic.code,
        message: diagnostic.message,
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    let file_name = path.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "la ruta de guardado debe incluir un nombre de archivo",
        )
    })?;
    let mut temporary_name = OsString::from(".");
    temporary_name.push(file_name);
    temporary_name.push(format!(
        ".tmp-{}-{}",
        std::process::id(),
        NEXT_TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let temporary_path: PathBuf = parent.join(temporary_name);

    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary_path, path)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use estudio_daw_project_model::{
        AudioClip, ImportProvenance, TimeSignature, Track, TrackKind, Transport,
    };
    use std::sync::atomic::AtomicU64;

    static NEXT_TEST_DIR: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn audio_runtime_settings_default_and_round_trip_are_versioned() {
        let directory = TestDirectory::new();
        let path = directory.0.join("settings/audio-runtime.json");
        let defaults = load_audio_runtime_settings_from(&path).unwrap();
        assert_eq!(defaults, AudioRuntimeSettings::default());
        defaults.validate().unwrap();

        save_audio_runtime_settings_to(&path, &defaults).unwrap();
        assert_eq!(load_audio_runtime_settings_from(&path).unwrap(), defaults);
        assert!(path.exists());
    }

    #[test]
    fn audio_runtime_settings_reject_unknown_versions_and_invalid_periods() {
        let directory = TestDirectory::new();
        let path = directory.0.join("audio-runtime.json");
        let mut settings = AudioRuntimeSettings::default();
        settings.schema_version = "audio-runtime-settings.v9".into();
        fs::write(&path, serde_json::to_vec(&settings).unwrap()).unwrap();
        assert!(matches!(
            load_audio_runtime_settings_from(&path),
            Err(AudioSettingsError::UnsupportedVersion(_))
        ));

        settings.schema_version = AUDIO_SETTINGS_SCHEMA.into();
        settings.live_record.device_period_frames = 0;
        assert!(matches!(
            save_audio_runtime_settings_to(&path, &settings),
            Err(AudioSettingsError::InvalidSettings)
        ));

        settings.live_record.device_period_frames = 2_049;
        assert!(matches!(
            save_audio_runtime_settings_to(&path, &settings),
            Err(AudioSettingsError::InvalidSettings)
        ));
    }

    #[test]
    fn audio_runtime_view_keeps_requested_period_distinct_from_unknown_effective_period() {
        let mut settings = AudioRuntimeSettings::default();
        settings.live_record.device_period_frames = 128;
        settings.live_record.playback_safety_frames = 512;
        let view = AudioRuntimeView::from_settings(settings, 48_000);
        assert_eq!(view.requested_period_frames, 128);
        assert!((view.requested_period_ms - 2.666_666_7).abs() < 0.001);
        assert_eq!(view.effective_period_frames, None);
        assert!(!view.playback_path_isolation_supported);
        assert_eq!(view.apply_state, "nextStream");
        assert!((view.playback_safety_ms - 10.666_666_7).abs() < 0.001);
        assert_eq!(frames_to_milliseconds(64, 0), None);
    }

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "estudio-daw-application-{}-{}",
                std::process::id(),
                NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).expect("crear directorio temporal único");
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn project() -> Project {
        Project {
            schema_version: "1".into(),
            project_id: "application-test".into(),
            transport: Transport {
                tempo_bpm: 92.0,
                time_signature: TimeSignature {
                    numerator: 4,
                    denominator: 4,
                },
                loop_range: None,
            },
            tracks: vec![Track {
                id: "track-audio".into(),
                name: "Audio".into(),
                kind: TrackKind::Audio,
                role: estudio_daw_project_model::TrackRole::Audio,
                output_track_id: None,
                input_route: None,
                record_armed: false,
                channel_config: estudio_daw_project_model::TrackChannelConfig {
                    input_channels: Some(2),
                    output_channels: 2,
                },
                color: "#58a6b8".into(),
                group_name: None,
                mixer: estudio_daw_project_model::TrackMixerState::default(),
                notes: Vec::new(),
                audio_channels: Some(2),
                media_source: None,
                instrument: None,
            }],
            audio_sources: Vec::new(),
            audio_playlists: Vec::new(),
            scenes: Vec::new(),
            clip_slots: Vec::new(),
            midi_clips: Vec::new(),
            audio_clips: vec![AudioClip {
                id: "clip-audio-1".into(),
                name: "Región".into(),
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

    #[test]
    fn open_execute_undo_redo_and_atomic_save_round_trip() {
        let directory = TestDirectory::new();
        let project_path = directory.0.join("project.json");
        fs::write(
            &project_path,
            serde_json::to_vec_pretty(&project()).unwrap(),
        )
        .unwrap();

        let mut application = ProjectApplication::open(&project_path).unwrap();
        assert_eq!(application.project_path(), Some(project_path.as_path()));
        assert_eq!(application.snapshot().transport.tempo_bpm, 92.0);
        assert_eq!(application.history_state(), (false, false));
        let committed = application
            .execute_project(ProjectCommand::SetAudioClipGain {
                clip_id: "clip-audio-1".into(),
                gain_db: -6.0,
            })
            .unwrap();
        assert_eq!(committed.len(), 1);
        assert_eq!(committed[0].author, CommandAuthor::User);
        assert_eq!(application.history_state(), (true, false));
        assert_eq!(
            application.snapshot().project.project.audio_clips[0].gain_db,
            -6.0
        );

        let undone = application.undo().unwrap();
        assert_eq!(undone.len(), 1);
        assert_eq!(application.history_state(), (false, true));
        assert_eq!(
            application.snapshot().project.project.audio_clips[0].gain_db,
            0.0
        );
        application.redo().unwrap();
        assert_eq!(
            application.snapshot().project.project.audio_clips[0].gain_db,
            -6.0
        );
        application
            .execute(
                DomainCommand::Session(SessionCommand::SetTempo(128.0)),
                CommandAuthor::User,
            )
            .unwrap();
        assert_eq!(application.snapshot().transport.tempo_bpm, 128.0);

        application.save().unwrap();
        let reopened = ProjectApplication::open(&project_path).unwrap();
        assert_eq!(
            reopened.snapshot().project.project.audio_clips[0].gain_db,
            -6.0
        );
        assert_eq!(reopened.snapshot().transport.tempo_bpm, 128.0);
        assert_eq!(
            reopened.snapshot().project.project.transport.tempo_bpm,
            128.0
        );

        let copy_path = directory.0.join("project-copy.json");
        application.save_as(&copy_path).unwrap();
        assert_eq!(application.project_path(), Some(copy_path.as_path()));
        let copied = ProjectApplication::open(&copy_path).unwrap();
        assert_eq!(
            copied.snapshot().project.project.audio_clips[0].gain_db,
            -6.0
        );
        assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 2);
    }

    #[test]
    fn opening_a_legacy_project_migrates_it_before_application_use() {
        let directory = TestDirectory::new();
        let project_path = directory.0.join("legacy.json");
        fs::write(
            &project_path,
            br#"{"project_id":"legacy","transport":{"tempo_bpm":90.0,"time_signature":{"numerator":4,"denominator":4}},"tracks":[],"import_provenance":{"format":"internal","format_version":"0","source_file":"","warnings":[]}}"#,
        )
        .unwrap();

        let application = ProjectApplication::open(&project_path).unwrap();
        let project = application.snapshot().project.project;
        assert_eq!(project.schema_version, "estudio-daw.project.v5");
        assert!(project.midi_clips.is_empty());
        assert!(project.audio_clips.is_empty());
    }

    #[test]
    fn rejected_command_does_not_change_the_application_snapshot() {
        let mut application = ProjectApplication::new(project());
        let before = application.snapshot();
        let error = application
            .execute_project(ProjectCommand::SetAudioClipGain {
                clip_id: "missing".into(),
                gain_db: -3.0,
            })
            .unwrap_err();

        assert!(matches!(error, ApplicationError::Rejected { .. }));
        assert_eq!(application.snapshot(), before);
        assert_eq!(application.history_state(), (false, false));
    }
}
