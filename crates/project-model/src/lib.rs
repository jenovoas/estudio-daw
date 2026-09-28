//! Modelo canónico mínimo y adaptador de intercambio DAWproject.
//!
//! DAWproject sólo es una frontera de intercambio. El modelo interno conserva
//! información adicional como escala, procedencia y estado de proxies.

use estudio_daw_midi_types::{MidiTake, RecordedMidiEvent, RecordedMidiMessage};
use quick_xml::{de::from_str, escape::escape};
use serde::{Deserialize, Serialize};
use std::{
    io::{Cursor, Read, Write},
    path::PathBuf,
};
use thiserror::Error;
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("no se pudo leer el archivo: {0}")]
    Io(#[from] std::io::Error),
    #[error("no se pudo abrir el contenedor DAWproject: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("XML inválido: {0}")]
    Xml(#[from] quick_xml::DeError),
}

/// Error de lectura del formato persistido de `project.json`.
#[derive(Debug, Error)]
pub enum ProjectJsonError {
    #[error("JSON de proyecto inválido: {0}")]
    Json(#[from] serde_json::Error),
    #[error("la raíz del proyecto debe ser un objeto JSON")]
    InvalidRoot,
    #[error("`schema_version` debe ser una cadena")]
    InvalidVersionType,
    #[error("revisión del formato de proyecto no soportada: {0}")]
    UnsupportedVersion(String),
    #[error("pista de proyecto inválida: {0}")]
    InvalidTrack(String),
}

/// Política que decide qué representación se usa durante reproducción o
/// edición. El original nunca se reemplaza ni se modifica por el proxy.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProxyPolicy {
    Original,
    Proxy,
    Auto,
}

/// Fuente persistente de un medio y su representación ligera opcional.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaSource {
    pub original_path: PathBuf,
    /// Firma de procedencia calculada cuando se incorpora el archivo.
    /// La implementación inicial usa tamaño y fecha; el hash de contenido
    /// completo se añadirá al job de generación de proxies.
    pub original_signature: String,
    /// Hash de contenido usado por los jobs de proxy y validación fuerte.
    #[serde(default)]
    pub original_hash: String,
    #[serde(default)]
    pub proxy: Option<ProxyAsset>,
}

/// Media original de un proyecto. Su archivo no pertenece a ninguna región:
/// varias regiones de una playlist pueden referenciar la misma fuente.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioSource {
    pub id: String,
    pub owner_track_id: String,
    pub media: MediaSource,
    #[serde(default)]
    pub sample_rate_hz: Option<u32>,
    #[serde(default)]
    pub channels: Option<u16>,
}

/// Orden explícito de regiones de audio para una pista.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioPlaylist {
    pub id: String,
    pub track_id: String,
    pub region_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Scene {
    pub id: String,
    pub name: String,
}

/// Un slot referencia un clip persistido; no contiene una copia del MIDI/audio.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClipSlot {
    pub id: String,
    pub scene_id: String,
    pub track_id: String,
    #[serde(default)]
    pub clip: Option<ClipReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "clip_id", rename_all = "snake_case")]
pub enum ClipReference {
    Midi(String),
    Audio(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProxyAsset {
    pub path: PathBuf,
    pub source_signature: String,
    #[serde(default)]
    pub source_hash: String,
    pub profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub schema_version: String,
    pub project_id: String,
    pub transport: Transport,
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub audio_sources: Vec<AudioSource>,
    #[serde(default)]
    pub audio_playlists: Vec<AudioPlaylist>,
    #[serde(default)]
    pub scenes: Vec<Scene>,
    #[serde(default)]
    pub clip_slots: Vec<ClipSlot>,
    #[serde(default)]
    pub midi_clips: Vec<MidiClip>,
    #[serde(default)]
    pub audio_clips: Vec<AudioClip>,
    pub import_provenance: ImportProvenance,
}

impl Project {
    /// Verifica identidad y relaciones persistidas entre pistas, medios y clips.
    pub fn validate_persisted_contracts(&self) -> Result<(), TrackValidationError> {
        if self
            .transport
            .loop_range
            .is_some_and(|range| range.start_tick >= range.end_tick)
        {
            return Err(TrackValidationError::InvalidTransportLoopRange);
        }
        let mut ids = std::collections::HashSet::new();
        let mut master_count = 0;
        for track in &self.tracks {
            track.validate()?;
            if !ids.insert(track.id.as_str()) {
                return Err(TrackValidationError::DuplicateId(track.id.clone()));
            }
            master_count += usize::from(track.role == TrackRole::Master);
        }
        if master_count > 1 {
            return Err(TrackValidationError::MultipleMasterTracks);
        }

        let tracks: std::collections::HashMap<_, _> =
            self.tracks.iter().map(|t| (t.id.as_str(), t)).collect();
        for track in &self.tracks {
            if track.role == TrackRole::Master && track.output_track_id.is_some() {
                return Err(TrackValidationError::InvalidTrackOutput);
            }
            if let Some(output_id) = track.output_track_id.as_deref() {
                if !tracks
                    .get(output_id)
                    .is_some_and(|target| target.kind == TrackKind::Audio && target.id != track.id)
                {
                    return Err(TrackValidationError::InvalidTrackOutput);
                }
            }
        }
        for track in &self.tracks {
            let mut visited = std::collections::HashSet::new();
            let mut cursor = track;
            while let Some(output_id) = cursor.output_track_id.as_deref() {
                if !visited.insert(cursor.id.as_str()) {
                    return Err(TrackValidationError::InvalidTrackOutput);
                }
                cursor = tracks
                    .get(output_id)
                    .copied()
                    .ok_or(TrackValidationError::InvalidTrackOutput)?;
            }
            if !visited.insert(cursor.id.as_str()) {
                return Err(TrackValidationError::InvalidTrackOutput);
            }
        }
        let mut source_ids = std::collections::HashSet::new();
        for source in &self.audio_sources {
            if source.id.trim().is_empty() || !source_ids.insert(source.id.as_str()) {
                return Err(TrackValidationError::InvalidAudioSource);
            }
            if !tracks
                .get(source.owner_track_id.as_str())
                .is_some_and(|t| t.role == TrackRole::Audio)
                || source.sample_rate_hz == Some(0)
                || source.channels == Some(0)
                || source.media.original_path.as_os_str().is_empty()
                || source.media.original_signature.trim().is_empty()
            {
                return Err(TrackValidationError::InvalidAudioSource);
            }
        }
        let audio_clips: std::collections::HashMap<_, _> = self
            .audio_clips
            .iter()
            .map(|c| (c.id.as_str(), c))
            .collect();
        let mut playlist_tracks = std::collections::HashSet::new();
        let mut playlist_ids = std::collections::HashSet::new();
        let mut listed_regions = std::collections::HashSet::new();
        for playlist in &self.audio_playlists {
            if playlist.id.trim().is_empty()
                || !playlist_ids.insert(playlist.id.as_str())
                || !playlist_tracks.insert(playlist.track_id.as_str())
                || !tracks
                    .get(playlist.track_id.as_str())
                    .is_some_and(|t| t.role == TrackRole::Audio)
            {
                return Err(TrackValidationError::InvalidAudioPlaylist);
            }
            for region_id in &playlist.region_ids {
                if !listed_regions.insert(region_id.as_str())
                    || !audio_clips
                        .get(region_id.as_str())
                        .is_some_and(|clip| clip.track_id == playlist.track_id)
                {
                    return Err(TrackValidationError::InvalidAudioPlaylist);
                }
            }
        }
        for clip in &self.audio_clips {
            if clip.duration_samples == 0
                || clip.sample_rate == 0
                || !(1..=32).contains(&clip.channels)
                || !clip.gain_db.is_finite()
                || clip.source_channel_selection.len() > 2
                || clip
                    .source_channel_selection
                    .iter()
                    .any(|channel| *channel >= clip.channels)
                || clip
                    .source_channel_selection
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    != clip.source_channel_selection.len()
                || clip.fade_in_samples.saturating_add(clip.fade_out_samples)
                    > clip.duration_samples
            {
                return Err(TrackValidationError::InvalidAudioPlaylist);
            }
            if let Some(source_id) = clip.source_id.as_deref() {
                if !self
                    .audio_sources
                    .iter()
                    .any(|s| s.id == source_id && s.owner_track_id == clip.track_id)
                {
                    return Err(TrackValidationError::InvalidAudioSource);
                }
            }
        }
        if listed_regions.len() != audio_clips.len() || audio_clips.len() != self.audio_clips.len()
        {
            return Err(TrackValidationError::InvalidAudioPlaylist);
        }

        let scene_ids: std::collections::HashSet<_> =
            self.scenes.iter().map(|s| s.id.as_str()).collect();
        if scene_ids.len() != self.scenes.len()
            || self
                .scenes
                .iter()
                .any(|s| s.id.trim().is_empty() || s.name.trim().is_empty())
        {
            return Err(TrackValidationError::InvalidClipSlot);
        }
        let midi_clips: std::collections::HashMap<_, _> =
            self.midi_clips.iter().map(|c| (c.id.as_str(), c)).collect();
        let mut slot_ids = std::collections::HashSet::new();
        let mut slot_coordinates = std::collections::HashSet::new();
        for slot in &self.clip_slots {
            if slot.id.trim().is_empty()
                || !slot_ids.insert(slot.id.as_str())
                || !scene_ids.contains(slot.scene_id.as_str())
                || !tracks.contains_key(slot.track_id.as_str())
                || !slot_coordinates.insert((slot.scene_id.as_str(), slot.track_id.as_str()))
            {
                return Err(TrackValidationError::InvalidClipSlot);
            }
            let compatible = match slot.clip.as_ref() {
                None => true,
                Some(ClipReference::Midi(id)) => midi_clips
                    .get(id.as_str())
                    .is_some_and(|clip| clip.track_id == slot.track_id),
                Some(ClipReference::Audio(id)) => audio_clips
                    .get(id.as_str())
                    .is_some_and(|clip| clip.track_id == slot.track_id),
            };
            if !compatible {
                return Err(TrackValidationError::InvalidClipSlot);
            }
        }
        Ok(())
    }

    /// Compatibilidad para llamadores previos al contrato de medios/regiones.
    pub fn validate_track_contracts(&self) -> Result<(), TrackValidationError> {
        self.validate_persisted_contracts()
    }
}

/// Deserializa y migra un proyecto antes de exponerlo al resto de la aplicación.
/// Los esquemas anteriores se migran en memoria. Las versiones futuras se
/// rechazan antes de deserializar para evitar guardar y perder datos que esta
/// versión aún no entiende.
pub fn load_project_json(bytes: &[u8]) -> Result<Project, ProjectJsonError> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes)?;
    let object = value.as_object_mut().ok_or(ProjectJsonError::InvalidRoot)?;

    let version = match object.get("schema_version") {
        None => "estudio-daw.project.v0",
        Some(serde_json::Value::String(version)) => version,
        Some(_) => return Err(ProjectJsonError::InvalidVersionType),
    }
    .to_owned();

    match version.as_str() {
        "estudio-daw.project.v0" | "0" => {
            object.insert(
                "schema_version".into(),
                serde_json::Value::String("estudio-daw.project.v1".into()),
            );
            object
                .entry("midi_clips")
                .or_insert_with(|| serde_json::Value::Array(Vec::new()));
            object
                .entry("audio_clips")
                .or_insert_with(|| serde_json::Value::Array(Vec::new()));
        }
        "1" => {
            object.insert(
                "schema_version".into(),
                serde_json::Value::String("estudio-daw.project.v1".into()),
            );
        }
        "estudio-daw.project.v1"
        | "estudio-daw.project.v2"
        | "estudio-daw.project.v3"
        | "estudio-daw.project.v4"
        | "estudio-daw.project.v5" => {}
        unsupported => {
            return Err(ProjectJsonError::UnsupportedVersion(unsupported.into()));
        }
    }

    // project.v2 añade el instrumento a cada pista MIDI. Los proyectos antiguos
    // mantienen las mismas notas y empiezan con el sinte de prueba, que no
    // depende de bibliotecas externas ni de SoundFonts instalados.
    if object.get("schema_version").and_then(|v| v.as_str()) != Some("estudio-daw.project.v2")
        && object.get("schema_version").and_then(|v| v.as_str()) != Some("estudio-daw.project.v3")
        && object.get("schema_version").and_then(|v| v.as_str()) != Some("estudio-daw.project.v4")
        && object.get("schema_version").and_then(|v| v.as_str()) != Some("estudio-daw.project.v5")
    {
        if let Some(tracks) = object.get_mut("tracks").and_then(|v| v.as_array_mut()) {
            for track in tracks {
                let Some(track) = track.as_object_mut() else {
                    continue;
                };
                if track.get("kind").and_then(|v| v.as_str()) == Some("midi") {
                    track
                        .entry("instrument")
                        .or_insert_with(|| serde_json::json!({ "backend": "sine" }));
                }
            }
        }
        object.insert(
            "schema_version".into(),
            serde_json::Value::String("estudio-daw.project.v2".into()),
        );
    }

    // project.v3 añade estado persistente de mixer y color a cada pista.
    // Los proyectos antiguos parten activos, sin mute/solo, en unidad y centrados.
    if object.get("schema_version").and_then(|v| v.as_str()) != Some("estudio-daw.project.v3")
        && object.get("schema_version").and_then(|v| v.as_str()) != Some("estudio-daw.project.v4")
        && object.get("schema_version").and_then(|v| v.as_str()) != Some("estudio-daw.project.v5")
    {
        if let Some(tracks) = object.get_mut("tracks").and_then(|v| v.as_array_mut()) {
            for track in tracks {
                let Some(track) = track.as_object_mut() else {
                    continue;
                };
                track
                    .entry("color")
                    .or_insert_with(|| serde_json::Value::String("#58a6b8".to_owned()));
                track.entry("mixer").or_insert_with(|| {
                    serde_json::json!({
                        "active": true,
                        "mute": false,
                        "solo": false,
                        "gain_db": 0.0,
                        "pan": 0.0
                    })
                });
            }
        }
        object.insert(
            "schema_version".into(),
            serde_json::Value::String("estudio-daw.project.v3".into()),
        );
    }

    // project.v4 separa la función de la pista de su medio y especifica la
    // topología de canales. En migración sólo se infiere lo que ya expresa el
    // modelo anterior; buses/returns/master requieren creación explícita.
    if let Some(tracks) = object.get_mut("tracks").and_then(|v| v.as_array_mut()) {
        for track in tracks {
            let Some(track) = track.as_object_mut() else {
                continue;
            };
            let is_audio = track.get("kind").and_then(|v| v.as_str()) == Some("audio");
            let has_instrument = track.get("instrument").is_some_and(|v| !v.is_null());
            let input_channels = if is_audio {
                track
                    .get("audio_channels")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            };
            let role = if is_audio {
                "audio"
            } else if has_instrument {
                "instrument"
            } else {
                "midi"
            };
            track
                .entry("role")
                .or_insert_with(|| serde_json::Value::String(role.into()));
            track.entry("channel_config").or_insert_with(|| {
                serde_json::json!({
                    "input_channels": input_channels,
                    "output_channels": 2
                })
            });
        }
    }
    if object.get("schema_version").and_then(|v| v.as_str()) != Some("estudio-daw.project.v4")
        && object.get("schema_version").and_then(|v| v.as_str()) != Some("estudio-daw.project.v5")
    {
        object.insert(
            "schema_version".into(),
            serde_json::Value::String("estudio-daw.project.v4".into()),
        );
    }

    // project.v5 normaliza la propiedad de medios y crea una playlist por
    // pista de audio. Los slots sólo guardan referencias a clips persistidos.
    let mut sources = object
        .get("audio_sources")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut playlists = object
        .get("audio_playlists")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    for track_value in object
        .get_mut("tracks")
        .and_then(|v| v.as_array_mut())
        .into_iter()
        .flatten()
    {
        let Some(track) = track_value.as_object_mut() else {
            continue;
        };
        if track.get("kind").and_then(|v| v.as_str()) != Some("audio") {
            continue;
        }
        let track_id = track
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_owned();
        let source_id = format!("source-{track_id}");
        if let Some(media) = track.remove("media_source").filter(|v| !v.is_null()) {
            if !sources
                .iter()
                .any(|v| v.get("id").and_then(|x| x.as_str()) == Some(&source_id))
            {
                sources.push(serde_json::json!({
                    "id": source_id,
                    "owner_track_id": track_id,
                    "media": media,
                    "sample_rate_hz": null,
                    "channels": null
                }));
            }
        }
        if !playlists
            .iter()
            .any(|v| v.get("track_id").and_then(|x| x.as_str()) == Some(&track_id))
        {
            playlists.push(serde_json::json!({"id": format!("playlist-{track_id}"), "track_id": track_id, "region_ids": []}));
        }
    }
    if let Some(clips) = object.get_mut("audio_clips").and_then(|v| v.as_array_mut()) {
        for clip in clips {
            let Some(clip_obj) = clip.as_object_mut() else {
                continue;
            };
            let track_id = clip_obj
                .get("track_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_owned();
            let source_id = format!("source-{track_id}");
            if sources
                .iter()
                .any(|v| v.get("id").and_then(|x| x.as_str()) == Some(&source_id))
            {
                clip_obj
                    .entry("source_id")
                    .or_insert_with(|| serde_json::Value::String(source_id.clone()));
                if let Some(source) = sources
                    .iter_mut()
                    .find(|v| v.get("id").and_then(|x| x.as_str()) == Some(&source_id))
                {
                    if source
                        .get("sample_rate_hz")
                        .map_or(true, serde_json::Value::is_null)
                    {
                        source["sample_rate_hz"] = clip_obj
                            .get("sample_rate")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null);
                    }
                    if source
                        .get("channels")
                        .map_or(true, serde_json::Value::is_null)
                    {
                        source["channels"] = clip_obj
                            .get("channels")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null);
                    }
                }
            }
            if let Some(playlist) = playlists
                .iter_mut()
                .find(|v| v.get("track_id").and_then(|x| x.as_str()) == Some(&track_id))
            {
                if !playlist.get("region_ids").is_some_and(|v| v.is_array()) {
                    playlist["region_ids"] = serde_json::Value::Array(Vec::new());
                }
                let Some(regions) = playlist
                    .get_mut("region_ids")
                    .and_then(|v| v.as_array_mut())
                else {
                    continue;
                };
                let id = clip_obj
                    .get("id")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                if !regions.contains(&id) {
                    regions.push(id);
                }
            }
        }
    }
    object.insert("audio_sources".into(), serde_json::Value::Array(sources));
    object.insert(
        "audio_playlists".into(),
        serde_json::Value::Array(playlists),
    );
    object
        .entry("scenes")
        .or_insert_with(|| serde_json::Value::Array(Vec::new()));
    object
        .entry("clip_slots")
        .or_insert_with(|| serde_json::Value::Array(Vec::new()));
    object.insert(
        "schema_version".into(),
        serde_json::Value::String("estudio-daw.project.v5".into()),
    );

    let project: Project = serde_json::from_value(value)?;
    project
        .validate_persisted_contracts()
        .map_err(|error| ProjectJsonError::InvalidTrack(error.to_string()))?;
    Ok(project)
}

/// Mutación completa y reversible del modelo de proyecto.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChangeSet {
    pub label: String,
    pub before: Project,
    pub after: Project,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectSnapshot {
    pub revision: u64,
    pub project: Project,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProjectEventKind {
    Committed { label: String },
    Undone { label: String },
    Redone { label: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectEvent {
    pub revision: u64,
    pub kind: ProjectEventKind,
}

/// Historial de cambios para UI, CLI y futuro scripting.
///
/// El historial sólo almacena modelo y referencias a archivos, nunca buffers
/// de audio. Por eso puede clonarse sin duplicar medios pesados.
#[derive(Debug, Clone, Default)]
pub struct ProjectHistory {
    current: Option<Project>,
    undo_stack: Vec<ChangeSet>,
    redo_stack: Vec<ChangeSet>,
    revision: u64,
    events: Vec<ProjectEvent>,
}

impl ProjectHistory {
    pub fn new(project: Project) -> Self {
        Self {
            current: Some(project),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            revision: 0,
            events: Vec::new(),
        }
    }

    pub fn project(&self) -> Option<&Project> {
        self.current.as_ref()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn snapshot(&self) -> Option<ProjectSnapshot> {
        self.current.as_ref().map(|project| ProjectSnapshot {
            revision: self.revision,
            project: project.clone(),
        })
    }

    /// Extrae eventos desde el último polling de la UI.
    pub fn drain_events(&mut self) -> Vec<ProjectEvent> {
        std::mem::take(&mut self.events)
    }

    /// Ejecuta una operación sobre una copia y la publica atómicamente.
    pub fn transact<F, E>(&mut self, label: impl Into<String>, operation: F) -> Result<(), E>
    where
        F: FnOnce(&mut Project) -> Result<(), E>,
    {
        let before = self
            .current
            .clone()
            .expect("el historial siempre tiene proyecto");
        let mut after = before.clone();
        operation(&mut after)?;
        self.undo_stack.push(ChangeSet {
            label: label.into(),
            before,
            after: after.clone(),
        });
        self.current = Some(after);
        self.redo_stack.clear();
        self.revision = self.revision.saturating_add(1);
        self.events.push(ProjectEvent {
            revision: self.revision,
            kind: ProjectEventKind::Committed {
                label: self
                    .undo_stack
                    .last()
                    .expect("change recién añadido")
                    .label
                    .clone(),
            },
        });
        Ok(())
    }

    pub fn undo(&mut self) -> Option<&Project> {
        let change = self.undo_stack.pop()?;
        let label = change.label.clone();
        self.current = Some(change.before.clone());
        self.redo_stack.push(change);
        self.revision = self.revision.saturating_add(1);
        self.events.push(ProjectEvent {
            revision: self.revision,
            kind: ProjectEventKind::Undone { label },
        });
        self.current.as_ref()
    }

    pub fn redo(&mut self) -> Option<&Project> {
        let change = self.redo_stack.pop()?;
        let label = change.label.clone();
        self.current = Some(change.after.clone());
        self.undo_stack.push(change);
        self.revision = self.revision.saturating_add(1);
        self.events.push(ProjectEvent {
            revision: self.revision,
            kind: ProjectEventKind::Redone { label },
        });
        self.current.as_ref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MidiClip {
    pub id: String,
    pub name: String,
    pub track_id: String,
    pub start_tick: u64,
    pub duration_ticks: u64,
    pub take: MidiTake,
}

/// Región de audio no destructiva dentro del arreglo.
///
/// La posición pertenece al timebase musical del proyecto (`start_tick`),
/// mientras que el recorte de la fuente se expresa en samples para conservar
/// precisión independiente del tempo. El sample rate se guarda por clip para
/// detectar conversiones necesarias antes del render.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioClip {
    pub id: String,
    pub name: String,
    pub track_id: String,
    /// Region-local source reference; None is retained only for legacy metadata
    /// regions that have not yet been relinked to a source record.
    #[serde(default)]
    pub source_id: Option<String>,
    pub start_tick: u64,
    pub source_start_samples: u64,
    pub duration_samples: u64,
    pub sample_rate: u32,
    pub channels: u16,
    /// Índices de canales de la fuente seleccionados para esta región. Vacío
    /// conserva la interpretación histórica de usar todos los canales.
    #[serde(default)]
    pub source_channel_selection: Vec<u16>,
    #[serde(default)]
    pub gain_db: f32,
    #[serde(default)]
    pub fade_in_samples: u64,
    #[serde(default)]
    pub fade_out_samples: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transport {
    pub tempo_bpm: f64,
    pub time_signature: TimeSignature,
    /// Rango musical de repetición, expresado a 960 ticks por negra.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_range: Option<TransportLoopRange>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransportLoopRange {
    pub start_tick: u64,
    pub end_tick: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimeSignature {
    pub numerator: u32,
    pub denominator: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub kind: TrackKind,
    /// Functional routing role is separate from the media/event format.
    #[serde(default)]
    pub role: TrackRole,
    /// Destino interno opcional del proyecto. La salida física pertenece al adaptador de plataforma.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_track_id: Option<String>,
    /// Entrada física elegida para esta pista. La clave es opaca al dominio;
    /// el adaptador de plataforma la resuelve contra dispositivos vigentes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_route: Option<TrackInputRoute>,
    /// Armado de grabación de la entrada física; no inicia captura por sí solo.
    #[serde(default)]
    pub record_armed: bool,
    #[serde(default)]
    pub channel_config: TrackChannelConfig,
    /// Color belongs to the project so Session and Arrangement remain visually linked.
    #[serde(default = "default_track_color")]
    pub color: String,
    /// Marca breve elegida por la persona para reconocer visualmente el instrumento.
    #[serde(default)]
    pub marker: String,
    /// Apunte libre de la persona sobre el papel o la toma de esta pista.
    #[serde(default)]
    pub annotation: String,
    /// Grupo organizativo persistente; no vincula el estado del mezclador.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_name: Option<String>,
    /// Mixer state is shared by all views; it is not presentation-only state.
    #[serde(default)]
    pub mixer: TrackMixerState,
    pub notes: Vec<Note>,
    pub audio_channels: Option<u32>,
    /// Procedencia original/proxy de una pista de audio. Las pistas MIDI no
    /// deben usar este campo; `None` conserva compatibilidad con project.json
    /// anteriores.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_source: Option<MediaSource>,
    /// Fuente de sonido asignada a la pista MIDI. `None` conserva proyectos y
    /// pistas sin instrumento hasta que el usuario elige uno explícitamente.
    #[serde(default)]
    pub instrument: Option<InstrumentConfig>,
}

/// Ruteo de una o dos entradas de una fuente física hacia una pista de audio.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrackInputRoute {
    pub device_key: String,
    pub channels: Vec<u16>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrackRole {
    Midi,
    Instrument,
    Audio,
    Bus,
    Return,
    Master,
}

impl Default for TrackRole {
    fn default() -> Self {
        Self::Midi
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct TrackChannelConfig {
    pub input_channels: Option<u32>,
    pub output_channels: u32,
}

impl Default for TrackChannelConfig {
    fn default() -> Self {
        Self {
            input_channels: None,
            output_channels: 2,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TrackValidationError {
    #[error("track id and name must not be empty")]
    EmptyIdentity,
    #[error("el color, la marca o la nota de pista no cumplen el formato admitido")]
    InvalidTrackIdentity,
    #[error("track id is duplicated: {0}")]
    DuplicateId(String),
    #[error("role is incompatible with the track media kind")]
    RoleKindMismatch,
    #[error("instrument role and instrument assignment must agree")]
    InstrumentRoleMismatch,
    #[error("MIDI tracks cannot own audio media, channel counts, or audio input channels")]
    AudioStateOnMidi,
    #[error("audio tracks cannot contain MIDI notes or an instrument")]
    MidiStateOnAudio,
    #[error("bus, return, and master tracks cannot own source media")]
    SourceOnInternalTrack,
    #[error("input and output channel counts must be greater than zero")]
    InvalidChannels,
    #[error("gain must be between -60 and +12 dB and pan between -1 and +1")]
    InvalidMixer,
    #[error("track group name must contain between 1 and 64 characters")]
    InvalidGroupName,
    #[error("a project can contain at most one master track")]
    MultipleMasterTracks,
    #[error("track output must reference a different existing audio track; the master has no project output")]
    InvalidTrackOutput,
    #[error("la entrada física requiere una pista de audio y uno o dos canales distintos en un dispositivo identificado")]
    InvalidInputRoute,
    #[error("el armado requiere una pista de audio con entrada física asignada")]
    InvalidRecordArm,
    #[error("audio source identity, owner, or format metadata is invalid")]
    InvalidAudioSource,
    #[error("audio playlist identity or region references are invalid")]
    InvalidAudioPlaylist,
    #[error("scene or clip-slot identity/reference is invalid")]
    InvalidClipSlot,
    #[error("el rango de bucle debe terminar después de comenzar")]
    InvalidTransportLoopRange,
}

impl Track {
    /// Creates an empty role-consistent track with conservative portable defaults.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        kind: TrackKind,
        role: TrackRole,
    ) -> Result<Self, TrackValidationError> {
        let instrument = (role == TrackRole::Instrument).then_some(InstrumentConfig::Sine);
        let audio_channels = (role == TrackRole::Audio).then_some(2);
        let track = Self {
            id: id.into(),
            name: name.into(),
            kind,
            role,
            output_track_id: None,
            input_route: None,
            record_armed: false,
            channel_config: TrackChannelConfig {
                input_channels: audio_channels,
                output_channels: 2,
            },
            color: default_track_color(),
            marker: String::new(),
            annotation: String::new(),
            group_name: None,
            mixer: TrackMixerState::default(),
            notes: Vec::new(),
            audio_channels,
            media_source: None,
            instrument,
        };
        track.validate()?;
        Ok(track)
    }

    pub fn validate(&self) -> Result<(), TrackValidationError> {
        if self.id.trim().is_empty() || self.name.trim().is_empty() {
            return Err(TrackValidationError::EmptyIdentity);
        }
        if self.color.len() != 7
            || !self.color.starts_with('#')
            || !self.color[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
            || self.marker.chars().count() > 4
            || self.annotation.chars().count() > 256
        {
            return Err(TrackValidationError::InvalidTrackIdentity);
        }
        let role_matches_kind = matches!(
            (&self.kind, self.role),
            (TrackKind::Midi, TrackRole::Midi | TrackRole::Instrument)
                | (
                    TrackKind::Audio,
                    TrackRole::Audio | TrackRole::Bus | TrackRole::Return | TrackRole::Master
                )
        );
        if !role_matches_kind {
            return Err(TrackValidationError::RoleKindMismatch);
        }
        if (self.role == TrackRole::Instrument) != self.instrument.is_some() {
            return Err(TrackValidationError::InstrumentRoleMismatch);
        }
        if self.channel_config.output_channels == 0 || self.channel_config.input_channels == Some(0)
        {
            return Err(TrackValidationError::InvalidChannels);
        }
        if let Some(route) = &self.input_route {
            let mut channels = std::collections::HashSet::new();
            if self.role != TrackRole::Audio
                || route.device_key.trim().is_empty()
                || route.channels.is_empty()
                || route.channels.len() > 2
                || route
                    .channels
                    .iter()
                    .any(|channel| *channel >= 2 || !channels.insert(*channel))
            {
                return Err(TrackValidationError::InvalidInputRoute);
            }
        }
        if self.record_armed && (self.role != TrackRole::Audio || self.input_route.is_none()) {
            return Err(TrackValidationError::InvalidRecordArm);
        }
        if !self.mixer.gain_db.is_finite()
            || !(-60.0..=12.0).contains(&self.mixer.gain_db)
            || !self.mixer.pan.is_finite()
            || !(-1.0..=1.0).contains(&self.mixer.pan)
        {
            return Err(TrackValidationError::InvalidMixer);
        }
        if self
            .group_name
            .as_ref()
            .is_some_and(|name| name.trim().is_empty() || name.trim().chars().count() > 64)
        {
            return Err(TrackValidationError::InvalidGroupName);
        }
        match &self.kind {
            TrackKind::Midi
                if self.media_source.is_some()
                    || self.audio_channels.is_some()
                    || self.channel_config.input_channels.is_some() =>
            {
                Err(TrackValidationError::AudioStateOnMidi)
            }
            TrackKind::Audio if self.instrument.is_some() || !self.notes.is_empty() => {
                Err(TrackValidationError::MidiStateOnAudio)
            }
            TrackKind::Audio if self.role != TrackRole::Audio && self.media_source.is_some() => {
                Err(TrackValidationError::SourceOnInternalTrack)
            }
            _ => Ok(()),
        }
    }
}

fn default_track_color() -> String {
    "#58a6b8".to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct TrackMixerState {
    pub active: bool,
    pub mute: bool,
    pub solo: bool,
    pub gain_db: f32,
    /// Normalized stereo pan in the range -1.0 (left) to 1.0 (right).
    pub pan: f32,
}

impl Default for TrackMixerState {
    fn default() -> Self {
        Self {
            active: true,
            mute: false,
            solo: false,
            gain_db: 0.0,
            pan: 0.0,
        }
    }
}

/// Referencia portable a un banco local: el proyecto guarda la ruta/URI, nunca
/// duplica el contenido potencialmente grande o sujeto a otra licencia.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SoundFontReference {
    pub path: String,
    #[serde(default)]
    pub sha256: Option<String>,
}

/// Configuración de instrumento persistible sin exponer detalles del backend
/// nativo ni introducir dependencias de plataforma en el modelo portable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "backend", rename_all = "snake_case")]
pub enum InstrumentConfig {
    Sine,
    FluidSynth {
        soundfont: SoundFontReference,
        bank: u16,
        program: u8,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TrackKind {
    Midi,
    Audio,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AttachTakeError {
    #[error("no existe una pista MIDI para adjuntar la toma")]
    NoMidiTrack,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MidiEditError {
    #[error("no existe el clip MIDI '{0}'")]
    ClipNotFound(String),
    #[error("la rejilla de cuantización debe ser mayor que cero")]
    InvalidGrid,
    #[error("el punto de división debe estar dentro del clip")]
    InvalidSplitPosition,
    #[error("el identificador del nuevo clip MIDI no es válido o ya existe")]
    DuplicateClipId,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AudioClipError {
    #[error("no existe la pista de audio '{0}'")]
    TrackNotFound(String),
    #[error("la pista '{0}' no es de audio")]
    NotAudioTrack(String),
    #[error("la pista '{0}' no tiene fuente de audio")]
    MissingSource(String),
    #[error("la duración del clip debe ser mayor que cero")]
    InvalidDuration,
    #[error("el sample rate debe ser mayor que cero")]
    InvalidSampleRate,
    #[error("el número de canales debe estar entre 1 y 32")]
    InvalidChannels,
    #[error("el formato del clip no coincide con los metadatos de su fuente")]
    SourceFormatMismatch,
    #[error("no existe el clip de audio '{0}'")]
    ClipNotFound(String),
    #[error("la ganancia debe ser un valor finito")]
    InvalidGain,
    #[error("el fade no puede superar la duración del clip")]
    InvalidFade,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MediaAttachError {
    #[error("no existe la pista '{0}'")]
    TrackNotFound(String),
    #[error("la pista '{0}' no es de audio")]
    NotAudioTrack(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Note {
    pub midi_key: u8,
    pub velocity: f32,
    pub time_beats: f64,
    pub duration_beats: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportProvenance {
    pub format: String,
    pub format_version: String,
    pub source_file: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportResult {
    pub project: Project,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportResult {
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct DawProject {
    #[serde(rename = "@version")]
    version: String,
    #[serde(rename = "Transport")]
    transport: Option<DawTransport>,
    #[serde(rename = "Structure")]
    structure: Option<DawStructure>,
    #[serde(rename = "Arrangement")]
    arrangement: Option<DawArrangement>,
}

#[derive(Debug, Deserialize)]
struct DawTransport {
    #[serde(rename = "Tempo")]
    tempo: Option<DawTempo>,
    #[serde(rename = "TimeSignature")]
    time_signature: Option<DawTimeSignature>,
}

#[derive(Debug, Deserialize)]
struct DawTempo {
    #[serde(rename = "@value")]
    value: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct DawTimeSignature {
    #[serde(rename = "@numerator")]
    numerator: u32,
    #[serde(rename = "@denominator")]
    denominator: u32,
}

#[derive(Debug, Deserialize)]
struct DawStructure {
    #[serde(rename = "Track", default)]
    tracks: Vec<DawTrack>,
}

#[derive(Debug, Deserialize)]
struct DawTrack {
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "@name", default)]
    name: String,
    #[serde(rename = "@contentType", default)]
    content_type: String,
    #[serde(rename = "Channel")]
    channel: Option<DawChannel>,
}

#[derive(Debug, Deserialize)]
struct DawChannel {
    #[serde(rename = "@audioChannels")]
    audio_channels: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct DawArrangement {
    #[serde(rename = "Lanes")]
    lanes: Option<DawLanes>,
}

#[derive(Debug, Deserialize)]
struct DawLanes {
    #[serde(rename = "Notes", default)]
    notes: Vec<DawNotes>,
}

#[derive(Debug, Deserialize)]
struct DawNotes {
    #[serde(rename = "@track")]
    track: String,
    #[serde(rename = "Note", default)]
    notes: Vec<DawNote>,
}

#[derive(Debug, Deserialize)]
struct DawNote {
    #[serde(rename = "@time")]
    time: f64,
    #[serde(rename = "@duration")]
    duration: f64,
    #[serde(rename = "@key")]
    key: u8,
    #[serde(rename = "@vel", default)]
    velocity: Option<f32>,
}

/// Importa los bytes de un contenedor `.dawproject` sin acceder al sistema de archivos.
pub fn import_dawproject(bytes: &[u8]) -> Result<ImportResult, ProjectError> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))?;
    let mut project_xml = String::new();
    archive
        .by_name("project.xml")?
        .read_to_string(&mut project_xml)?;
    import_project_xml(&project_xml)
}

/// Importa `project.xml`, útil para fixtures y pruebas del adaptador.
pub fn import_project_xml(xml: &str) -> Result<ImportResult, ProjectError> {
    let source: DawProject = from_str(xml)?;
    let tempo = source
        .transport
        .as_ref()
        .and_then(|t| t.tempo.as_ref())
        .and_then(|t| t.value)
        .unwrap_or(120.0);
    let signature = source
        .transport
        .as_ref()
        .and_then(|t| t.time_signature.as_ref())
        .map(|s| TimeSignature {
            numerator: s.numerator,
            denominator: s.denominator,
        })
        .unwrap_or(TimeSignature {
            numerator: 4,
            denominator: 4,
        });

    let daw_tracks = source.structure.map(|s| s.tracks).unwrap_or_default();
    let arrangement_notes = source
        .arrangement
        .and_then(|a| a.lanes)
        .map(|l| l.notes)
        .unwrap_or_default();
    let mut warnings = Vec::new();
    if arrangement_notes.is_empty() {
        warnings.push("El proyecto no contiene notas en Arrangement/Lanes.".into());
    }
    warnings
        .push("El estado de plugins, proxies y análisis no forma parte del XML importado.".into());

    let tracks = daw_tracks
        .into_iter()
        .map(|track| {
            let kind = if track.content_type.contains("audio") {
                TrackKind::Audio
            } else {
                TrackKind::Midi
            };
            let is_audio_track = matches!(&kind, TrackKind::Audio);
            let notes = arrangement_notes
                .iter()
                .filter(|group| group.track == track.id)
                .flat_map(|group| group.notes.iter())
                .map(|note| Note {
                    midi_key: note.key,
                    velocity: note.velocity.unwrap_or(1.0),
                    time_beats: note.time,
                    duration_beats: note.duration,
                })
                .collect();
            let instrument = matches!(&kind, TrackKind::Midi).then_some(InstrumentConfig::Sine);
            let role = if matches!(&kind, TrackKind::Audio) {
                TrackRole::Audio
            } else {
                TrackRole::Instrument
            };
            Track {
                id: track.id,
                name: track.name,
                kind,
                role,
                output_track_id: None,
                input_route: None,
                record_armed: false,
                channel_config: TrackChannelConfig {
                    input_channels: is_audio_track
                        .then(|| track.channel.as_ref().and_then(|c| c.audio_channels))
                        .flatten(),
                    output_channels: 2,
                },
                color: default_track_color(),
                marker: String::new(),
                annotation: String::new(),
                group_name: None,
                mixer: TrackMixerState::default(),
                notes,
                audio_channels: is_audio_track
                    .then(|| track.channel.and_then(|c| c.audio_channels))
                    .flatten(),
                media_source: None,
                instrument,
            }
        })
        .collect();

    let project = Project {
        schema_version: "estudio-daw.project.v5".into(),
        project_id: "imported-dawproject".into(),
        transport: Transport {
            tempo_bpm: tempo,
            time_signature: signature,
            loop_range: None,
        },
        tracks,
        audio_sources: Vec::new(),
        audio_playlists: Vec::new(),
        scenes: Vec::new(),
        clip_slots: Vec::new(),
        midi_clips: Vec::new(),
        audio_clips: Vec::new(),
        import_provenance: ImportProvenance {
            format: "dawproject".into(),
            format_version: source.version,
            source_file: "project.xml".into(),
            warnings: warnings.clone(),
        },
    };
    Ok(ImportResult { project, warnings })
}

/// Asocia una fuente de audio a una pista sin tocar el archivo original.
pub fn attach_media_source(
    project: &mut Project,
    track_id: &str,
    source: MediaSource,
) -> Result<(), MediaAttachError> {
    let track = project
        .tracks
        .iter()
        .find(|track| track.id == track_id)
        .ok_or_else(|| MediaAttachError::TrackNotFound(track_id.into()))?;
    if track.kind != TrackKind::Audio {
        return Err(MediaAttachError::NotAudioTrack(track_id.into()));
    }
    let source_id = format!("source-{track_id}");
    project.audio_sources.retain(|item| item.id != source_id);
    append_media_source_with_id(project, track_id, source_id, source).map(|_| ())
}

/// Añade una fuente adicional sin sustituir las fuentes existentes de la pista.
pub fn append_media_source(
    project: &mut Project,
    track_id: &str,
    source: MediaSource,
) -> Result<String, MediaAttachError> {
    let track = project
        .tracks
        .iter()
        .find(|track| track.id == track_id)
        .ok_or_else(|| MediaAttachError::TrackNotFound(track_id.into()))?;
    if track.kind != TrackKind::Audio {
        return Err(MediaAttachError::NotAudioTrack(track_id.into()));
    }
    let source_index = project
        .audio_sources
        .iter()
        .filter(|item| item.owner_track_id == track_id)
        .count()
        + 1;
    let source_id = format!("source-{track_id}-{source_index}");
    append_media_source_with_id(project, track_id, source_id, source)
}

fn append_media_source_with_id(
    project: &mut Project,
    track_id: &str,
    source_id: String,
    source: MediaSource,
) -> Result<String, MediaAttachError> {
    project.audio_sources.push(AudioSource {
        id: source_id.clone(),
        owner_track_id: track_id.to_owned(),
        media: source,
        sample_rate_hz: None,
        channels: None,
    });
    if !project
        .audio_playlists
        .iter()
        .any(|item| item.track_id == track_id)
    {
        project.audio_playlists.push(AudioPlaylist {
            id: format!("playlist-{track_id}"),
            track_id: track_id.to_owned(),
            region_ids: project
                .audio_clips
                .iter()
                .filter(|clip| clip.track_id == track_id)
                .map(|clip| clip.id.clone())
                .collect(),
        });
    }
    Ok(source_id)
}

/// Añade una región que referencia la fuente de su pista sin copiar ni cortar
/// el archivo. Las ediciones posteriores operarán sobre esta descripción.
pub fn add_audio_clip(
    project: &mut Project,
    track_id: &str,
    name: impl Into<String>,
    start_tick: u64,
    source_start_samples: u64,
    duration_samples: u64,
    sample_rate: u32,
    channels: u16,
) -> Result<String, AudioClipError> {
    let track = project
        .tracks
        .iter()
        .find(|track| track.id == track_id)
        .ok_or_else(|| AudioClipError::TrackNotFound(track_id.into()))?;
    if track.kind != TrackKind::Audio {
        return Err(AudioClipError::NotAudioTrack(track_id.into()));
    }
    let source_id = project
        .audio_sources
        .iter()
        .find(|source| source.owner_track_id == track_id)
        .map(|source| source.id.clone())
        .ok_or_else(|| AudioClipError::MissingSource(track_id.into()))?;
    add_audio_clip_for_source(
        project,
        track_id,
        &source_id,
        name,
        start_tick,
        source_start_samples,
        duration_samples,
        sample_rate,
        channels,
    )
}

/// Añade una región eligiendo explícitamente una fuente entre las de la pista.
pub fn add_audio_clip_for_source(
    project: &mut Project,
    track_id: &str,
    requested_source_id: &str,
    name: impl Into<String>,
    start_tick: u64,
    source_start_samples: u64,
    duration_samples: u64,
    sample_rate: u32,
    channels: u16,
) -> Result<String, AudioClipError> {
    let track = project
        .tracks
        .iter()
        .find(|track| track.id == track_id)
        .ok_or_else(|| AudioClipError::TrackNotFound(track_id.into()))?;
    if track.kind != TrackKind::Audio {
        return Err(AudioClipError::NotAudioTrack(track_id.into()));
    }
    let source_id = project
        .audio_sources
        .iter()
        .find(|source| source.id == requested_source_id && source.owner_track_id == track_id)
        .map(|source| source.id.clone())
        .ok_or_else(|| AudioClipError::MissingSource(track_id.into()))?;
    if duration_samples == 0 {
        return Err(AudioClipError::InvalidDuration);
    }
    if sample_rate == 0 {
        return Err(AudioClipError::InvalidSampleRate);
    }
    if !(1..=32).contains(&channels) {
        return Err(AudioClipError::InvalidChannels);
    }
    let source = project
        .audio_sources
        .iter_mut()
        .find(|source| source.id == source_id)
        .ok_or_else(|| AudioClipError::MissingSource(track_id.into()))?;
    if source
        .sample_rate_hz
        .is_some_and(|rate| rate != sample_rate)
        || source.channels.is_some_and(|count| count != channels)
    {
        return Err(AudioClipError::SourceFormatMismatch);
    }
    source.sample_rate_hz = Some(sample_rate);
    source.channels = Some(channels);
    let id = format!("audio-clip-{}", project.audio_clips.len() + 1);
    project.audio_clips.push(AudioClip {
        id: id.clone(),
        name: name.into(),
        track_id: track_id.into(),
        source_id: Some(source_id),
        start_tick,
        source_start_samples,
        duration_samples,
        sample_rate,
        channels,
        source_channel_selection: Vec::new(),
        gain_db: 0.0,
        fade_in_samples: 0,
        fade_out_samples: 0,
    });
    let playlist = project
        .audio_playlists
        .iter_mut()
        .find(|item| item.track_id == track_id);
    if let Some(playlist) = playlist {
        playlist.region_ids.push(id.clone());
    } else {
        project.audio_playlists.push(AudioPlaylist {
            id: format!("playlist-{track_id}"),
            track_id: track_id.into(),
            region_ids: vec![id.clone()],
        });
    }
    Ok(id)
}

/// Cambia el recorte de una región sin editar ni reescribir su fuente.
pub fn trim_audio_clip(
    project: &mut Project,
    clip_id: &str,
    source_start_samples: u64,
    duration_samples: u64,
) -> Result<(), AudioClipError> {
    if duration_samples == 0 {
        return Err(AudioClipError::InvalidDuration);
    }
    let clip = project
        .audio_clips
        .iter_mut()
        .find(|clip| clip.id == clip_id)
        .ok_or_else(|| AudioClipError::ClipNotFound(clip_id.into()))?;
    if clip.fade_in_samples + clip.fade_out_samples > duration_samples {
        return Err(AudioClipError::InvalidFade);
    }
    clip.source_start_samples = source_start_samples;
    clip.duration_samples = duration_samples;
    Ok(())
}

pub fn set_audio_clip_gain(
    project: &mut Project,
    clip_id: &str,
    gain_db: f32,
) -> Result<(), AudioClipError> {
    if !gain_db.is_finite() {
        return Err(AudioClipError::InvalidGain);
    }
    let clip = project
        .audio_clips
        .iter_mut()
        .find(|clip| clip.id == clip_id)
        .ok_or_else(|| AudioClipError::ClipNotFound(clip_id.into()))?;
    clip.gain_db = gain_db.clamp(-120.0, 24.0);
    Ok(())
}

pub fn set_audio_clip_fades(
    project: &mut Project,
    clip_id: &str,
    fade_in_samples: u64,
    fade_out_samples: u64,
) -> Result<(), AudioClipError> {
    let clip = project
        .audio_clips
        .iter_mut()
        .find(|clip| clip.id == clip_id)
        .ok_or_else(|| AudioClipError::ClipNotFound(clip_id.into()))?;
    if fade_in_samples.saturating_add(fade_out_samples) > clip.duration_samples {
        return Err(AudioClipError::InvalidFade);
    }
    clip.fade_in_samples = fade_in_samples;
    clip.fade_out_samples = fade_out_samples;
    Ok(())
}

/// Adjunta una toma MIDI a la primera pista MIDI disponible.
///
/// La operación conserva la toma completa y crea una entidad de clip para que
/// el motor pueda ubicarla en el arreglo sin modificar el archivo de captura.
pub fn attach_midi_take(
    project: &mut Project,
    take: MidiTake,
    name: impl Into<String>,
) -> Result<String, AttachTakeError> {
    let track_id = project
        .tracks
        .iter()
        .find(|track| track.kind == TrackKind::Midi)
        .map(|track| track.id.clone())
        .ok_or(AttachTakeError::NoMidiTrack)?;
    let duration_ticks = take
        .events
        .iter()
        .map(|event| event.tick)
        .max()
        .unwrap_or(0);
    let id = format!("midi-clip-{}", project.midi_clips.len() + 1);
    project.midi_clips.push(MidiClip {
        id: id.clone(),
        name: name.into(),
        track_id,
        start_tick: 0,
        duration_ticks,
        take,
    });
    Ok(id)
}

/// Cuantiza únicamente Note On/Off y deja intactos CC, aftertouch, pitch bend
/// y SysEx para preservar la interpretación expresiva.
pub fn quantize_midi_clip(
    project: &mut Project,
    clip_id: &str,
    grid_ticks: u64,
) -> Result<usize, MidiEditError> {
    if grid_ticks == 0 {
        return Err(MidiEditError::InvalidGrid);
    }
    let clip = project
        .midi_clips
        .iter_mut()
        .find(|clip| clip.id == clip_id)
        .ok_or_else(|| MidiEditError::ClipNotFound(clip_id.into()))?;
    let mut changed = 0;
    for event in &mut clip.take.events {
        if matches!(
            event.message,
            RecordedMidiMessage::NoteOn { .. } | RecordedMidiMessage::NoteOff { .. }
        ) {
            let quantized = ((event.tick + grid_ticks / 2) / grid_ticks) * grid_ticks;
            if quantized != event.tick {
                event.tick = quantized;
                changed += 1;
            }
        }
    }
    Ok(changed)
}

/// Divide un clip en un tick relativo a su toma, manteniendo ambos fragmentos
/// en el mismo proyecto. Las notas que cruzan el corte se cierran y rearticulan
/// en el segundo fragmento; se copia allí el estado MIDI de canal recuperable.
pub fn split_midi_clip(
    project: &mut Project,
    clip_id: &str,
    split_tick: u64,
    new_clip_id: &str,
) -> Result<(), MidiEditError> {
    let index = project
        .midi_clips
        .iter()
        .position(|clip| clip.id == clip_id)
        .ok_or_else(|| MidiEditError::ClipNotFound(clip_id.into()))?;
    let mut source = project.midi_clips[index].clone();
    if split_tick == 0 || split_tick >= source.duration_ticks {
        return Err(MidiEditError::InvalidSplitPosition);
    }
    if new_clip_id.trim().is_empty() || project.midi_clips.iter().any(|clip| clip.id == new_clip_id)
    {
        return Err(MidiEditError::DuplicateClipId);
    }
    source.take.events.sort_by_key(|event| event.tick);

    let mut active_notes = Vec::<(u8, u8, u8, estudio_daw_midi_types::MidiSource)>::new();
    let mut channel_state = Vec::<RecordedMidiEvent>::new();
    let mut key_pressure_state = Vec::<RecordedMidiEvent>::new();
    let mut left_events = Vec::new();
    let mut right_events = Vec::new();
    let mut boundary_events = Vec::new();
    for event in source
        .take
        .events
        .iter()
        .filter(|event| event.tick < split_tick)
    {
        match &event.message {
            RecordedMidiMessage::NoteOn {
                channel,
                note,
                velocity,
            } if *velocity > 0 => {
                active_notes.push((*channel, *note, *velocity, event.source.clone()));
            }
            RecordedMidiMessage::NoteOff { channel, note, .. }
            | RecordedMidiMessage::NoteOn {
                channel,
                note,
                velocity: 0,
            } => {
                if let Some(index) =
                    active_notes
                        .iter()
                        .rposition(|(active_channel, active_note, _, _)| {
                            active_channel == channel && active_note == note
                        })
                {
                    active_notes.remove(index);
                }
            }
            _ => {}
        }

        let state_key = match &event.message {
            RecordedMidiMessage::ControlChange {
                channel,
                controller,
                ..
            } => Some((0_u8, *channel, *controller, 0_u8)),
            RecordedMidiMessage::PitchBend { channel, .. } => Some((1, *channel, 0, 0)),
            RecordedMidiMessage::ChannelPressure { channel, .. } => Some((2, *channel, 0, 0)),
            RecordedMidiMessage::ProgramChange { channel, .. } => Some((3, *channel, 0, 0)),
            RecordedMidiMessage::KeyPressure { channel, note, .. } => {
                Some((4, *channel, u32::from(*note), 0))
            }
            _ => None,
        };
        if let Some(key) = state_key {
            let state_events = if matches!(event.message, RecordedMidiMessage::KeyPressure { .. }) {
                &mut key_pressure_state
            } else {
                &mut channel_state
            };
            if let Some(previous) = state_events.iter_mut().find(|previous| {
                let previous_key = match &previous.message {
                    RecordedMidiMessage::ControlChange {
                        channel,
                        controller,
                        ..
                    } => Some((0_u8, *channel, *controller, 0_u8)),
                    RecordedMidiMessage::PitchBend { channel, .. } => Some((1, *channel, 0, 0)),
                    RecordedMidiMessage::ChannelPressure { channel, .. } => {
                        Some((2, *channel, 0, 0))
                    }
                    RecordedMidiMessage::ProgramChange { channel, .. } => Some((3, *channel, 0, 0)),
                    RecordedMidiMessage::KeyPressure { channel, note, .. } => {
                        Some((4, *channel, u32::from(*note), 0))
                    }
                    _ => None,
                };
                previous_key == Some(key)
            }) {
                *previous = event.clone();
            } else {
                state_events.push(event.clone());
            }
        }
        left_events.push(event.clone());
    }
    let notes_to_close_left = active_notes.clone();

    // NoteOff exactly at the cut closes the left side and is not replayed on
    // the right. Other events at that tick belong to the right fragment.
    for event in source
        .take
        .events
        .iter()
        .filter(|event| event.tick == split_tick)
    {
        let closes_prior_note = match &event.message {
            RecordedMidiMessage::NoteOff { channel, note, .. }
            | RecordedMidiMessage::NoteOn {
                channel,
                note,
                velocity: 0,
            } => {
                if let Some(index) =
                    active_notes
                        .iter()
                        .rposition(|(active_channel, active_note, _, _)| {
                            active_channel == channel && active_note == note
                        })
                {
                    active_notes.remove(index);
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        if !closes_prior_note {
            boundary_events.push(rebase_midi_event(
                event.clone(),
                split_tick,
                source.take.ppq,
                source.take.tempo_bpm,
            ));
        }
    }

    for (channel, note, _, source_port) in &notes_to_close_left {
        left_events.push(RecordedMidiEvent {
            tick: split_tick,
            micros_since_start: midi_ticks_to_micros(
                split_tick,
                source.take.ppq,
                source.take.tempo_bpm,
            ),
            source: source_port.clone(),
            message: RecordedMidiMessage::NoteOff {
                channel: *channel,
                note: *note,
                release_velocity: 0,
            },
        });
    }
    for event in channel_state {
        right_events.push(rebase_midi_event(
            event.clone(),
            event.tick,
            source.take.ppq,
            source.take.tempo_bpm,
        ));
    }
    for (channel, note, velocity, source_port) in &active_notes {
        right_events.push(RecordedMidiEvent {
            tick: 0,
            micros_since_start: 0,
            source: source_port.clone(),
            message: RecordedMidiMessage::NoteOn {
                channel: *channel,
                note: *note,
                velocity: *velocity,
            },
        });
    }
    right_events.extend(boundary_events);
    for event in key_pressure_state {
        right_events.push(rebase_midi_event(
            event.clone(),
            event.tick,
            source.take.ppq,
            source.take.tempo_bpm,
        ));
    }
    for event in source
        .take
        .events
        .iter()
        .filter(|event| event.tick > split_tick)
    {
        right_events.push(rebase_midi_event(
            event.clone(),
            split_tick,
            source.take.ppq,
            source.take.tempo_bpm,
        ));
    }
    left_events.sort_by_key(|event| event.tick);
    right_events.sort_by_key(|event| event.tick);

    let left_duration_micros =
        midi_ticks_to_micros(split_tick, source.take.ppq, source.take.tempo_bpm);
    let mut left = source.clone();
    left.duration_ticks = split_tick;
    left.take.events = left_events;
    left.take.duration_micros = left_duration_micros;

    let mut right = source;
    right.id = new_clip_id.into();
    right.name = format!("{} (parte 2)", right.name);
    right.start_tick = right.start_tick.saturating_add(split_tick);
    right.duration_ticks = right.duration_ticks.saturating_sub(split_tick);
    right.take.events = right_events;
    right.take.duration_micros = right
        .take
        .duration_micros
        .saturating_sub(left_duration_micros);

    project.midi_clips[index] = left;
    project.midi_clips.insert(index + 1, right);
    Ok(())
}

fn rebase_midi_event(
    mut event: RecordedMidiEvent,
    offset_tick: u64,
    ppq: u32,
    tempo_bpm: u32,
) -> RecordedMidiEvent {
    let micros_offset = midi_ticks_to_micros(offset_tick, ppq, tempo_bpm);
    event.tick = event.tick.saturating_sub(offset_tick);
    event.micros_since_start = event.micros_since_start.saturating_sub(micros_offset);
    event
}

fn midi_ticks_to_micros(ticks: u64, ppq: u32, tempo_bpm: u32) -> u64 {
    (u128::from(ticks) * 60_000_000 / u128::from(ppq.max(1)) / u128::from(tempo_bpm.max(1)))
        .min(u128::from(u64::MAX)) as u64
}

/// Exporta el modelo interno como un contenedor `.dawproject` mínimo.
///
/// Los artefactos de audio, proxies, plugins y análisis no se inventan durante
/// la exportación: se reportan como warnings hasta que exista su adaptador.
pub fn export_dawproject(project: &Project) -> Result<(Vec<u8>, ExportResult), ProjectError> {
    let mut warnings = project.import_provenance.warnings.clone();
    if project
        .tracks
        .iter()
        .any(|track| matches!(track.kind, TrackKind::Audio))
    {
        warnings.push("Las referencias de audio/proxy aún no se exportan al contenedor.".into());
    }
    if !project.midi_clips.is_empty() {
        warnings
            .push("Los clips MIDI internos aún no se exportan al contenedor DAWproject.".into());
    }
    warnings
        .push("El estado de plugins, análisis y operaciones del agente no se exporta aún.".into());

    let project_xml = project_to_xml(project);
    let metadata_xml = metadata_to_xml(project);
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    archive.start_file("project.xml", options)?;
    archive.write_all(project_xml.as_bytes())?;
    archive.start_file("metadata.xml", options)?;
    archive.write_all(metadata_xml.as_bytes())?;
    let bytes = archive.finish()?.into_inner();
    Ok((bytes, ExportResult { warnings }))
}

fn project_to_xml(project: &Project) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n");
    xml.push_str("<Project version=\"1.0\">\n");
    xml.push_str("  <Application name=\"Estudio DAW\" version=\"0.1.0\"/>\n");
    xml.push_str("  <Transport>\n");
    xml.push_str(&format!(
        "    <Tempo max=\"666.000000\" min=\"20.000000\" unit=\"bpm\" value=\"{:.6}\" id=\"tempo\" name=\"Tempo\"/>\n",
        project.transport.tempo_bpm
    ));
    xml.push_str(&format!(
        "    <TimeSignature numerator=\"{}\" denominator=\"{}\" id=\"timesig\" name=\"Time Signature\"/>\n",
        project.transport.time_signature.numerator,
        project.transport.time_signature.denominator
    ));
    xml.push_str("  </Transport>\n  <Structure>\n");
    for track in &project.tracks {
        let content_type = match track.kind {
            TrackKind::Midi => "notes",
            TrackKind::Audio => "audio",
        };
        xml.push_str(&format!(
            "    <Track contentType=\"{}\" id=\"{}\" name=\"{}\">\n",
            content_type,
            escape_attr(&track.id),
            escape_attr(&track.name)
        ));
        xml.push_str(&format!(
            "      <Channel audioChannels=\"{}\" id=\"channel-{}\" name=\"{}\" role=\"regular\">\n",
            track.audio_channels.unwrap_or(2),
            escape_attr(&track.id),
            escape_attr(&track.name)
        ));
        xml.push_str("        <Pan max=\"1.000000\" min=\"-1.000000\" unit=\"linear\" value=\"0.000000\" id=\"pan-");
        xml.push_str(&escape_attr(&track.id));
        xml.push_str("\" name=\"Pan\"/>\n");
        xml.push_str("        <Volume max=\"1.000000\" min=\"0.000000\" unit=\"linear\" value=\"0.800000\" id=\"volume-");
        xml.push_str(&escape_attr(&track.id));
        xml.push_str("\" name=\"Volume\"/>\n      </Channel>\n    </Track>\n");
    }
    xml.push_str("  </Structure>\n  <Arrangement id=\"arrangement\" name=\"Arrangement\">\n    <Lanes id=\"arrangement-lanes\" timeUnit=\"beats\">\n");
    for track in project
        .tracks
        .iter()
        .filter(|track| !track.notes.is_empty())
    {
        xml.push_str(&format!(
            "      <Notes id=\"notes-{}\" track=\"{}\" timeUnit=\"beats\">\n",
            escape_attr(&track.id),
            escape_attr(&track.id)
        ));
        for note in &track.notes {
            xml.push_str(&format!(
                "        <Note time=\"{:.6}\" duration=\"{:.6}\" channel=\"1\" key=\"{}\" vel=\"{:.6}\"/>\n",
                note.time_beats, note.duration_beats, note.midi_key, note.velocity
            ));
        }
        xml.push_str("      </Notes>\n");
    }
    xml.push_str("    </Lanes>\n  </Arrangement>\n</Project>\n");
    xml
}

fn metadata_to_xml(project: &Project) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<MetaData>\n  <Title>{}</Title>\n  <Artist>Estudio DAW</Artist>\n  <Comment>Exported project {}</Comment>\n</MetaData>\n",
        escape_attr(&project.project_id),
        escape_attr(&project.project_id)
    )
}

fn escape_attr(value: &str) -> String {
    escape(value).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use estudio_daw_midi_types::{MidiSource, RecordedMidiEvent, RecordedMidiMessage};

    fn import_fixture() -> Result<ImportResult, ProjectError> {
        import_dawproject(include_bytes!(
            "../../../tests/fixtures/dawproject/minimal.dawproject"
        ))
    }

    #[test]
    fn migrates_unversioned_project_and_preserves_legacy_content() {
        let legacy = br#"{
            "project_id":"legacy-song",
            "transport":{"tempo_bpm":92.0,"time_signature":{"numerator":4,"denominator":4}},
            "tracks":[{"id":"track-1","name":"Voice","kind":"audio","notes":[],"audio_channels":2}],
            "import_provenance":{"format":"estudio-daw","format_version":"legacy","source_file":"","warnings":[]}
        }"#;

        let project = load_project_json(legacy).unwrap();

        assert_eq!(project.schema_version, "estudio-daw.project.v5");
        assert_eq!(project.project_id, "legacy-song");
        assert_eq!(project.tracks[0].name, "Voice");
        assert!(project.tracks[0].media_source.is_none());
        assert!(project.tracks[0].instrument.is_none());
        assert_eq!(project.tracks[0].color, "#58a6b8");
        assert_eq!(project.tracks[0].mixer, TrackMixerState::default());
        assert_eq!(project.tracks[0].role, TrackRole::Audio);
        assert_eq!(project.tracks[0].channel_config.input_channels, Some(2));
        assert!(project.midi_clips.is_empty());
        assert!(project.audio_clips.is_empty());
    }

    #[test]
    fn migrates_v1_midi_tracks_to_explicit_sine_instrument() {
        let v1 = br#"{
            "schema_version":"estudio-daw.project.v1",
            "project_id":"v1-song",
            "transport":{"tempo_bpm":92.0,"time_signature":{"numerator":4,"denominator":4}},
            "tracks":[{"id":"track-midi","name":"Keys","kind":"midi","notes":[{"midi_key":60,"velocity":0.8,"time_beats":0.5,"duration_beats":1.0}]}],
            "import_provenance":{"format":"estudio-daw","format_version":"1","source_file":"","warnings":[]}
        }"#;

        let project = load_project_json(v1).unwrap();

        assert_eq!(project.schema_version, "estudio-daw.project.v5");
        assert_eq!(project.tracks[0].id, "track-midi");
        assert_eq!(project.tracks[0].instrument, Some(InstrumentConfig::Sine));
        assert_eq!(project.tracks[0].notes[0].midi_key, 60);
        assert_eq!(project.tracks[0].notes[0].time_beats, 0.5);
        assert_eq!(project.tracks[0].mixer, TrackMixerState::default());
        assert_eq!(project.tracks[0].role, TrackRole::Instrument);
        assert_eq!(project.tracks[0].color, default_track_color());
        assert!(project.tracks[0].marker.is_empty());
        assert!(project.tracks[0].annotation.is_empty());
    }

    #[test]
    fn migrates_v2_tracks_without_changing_identity_order_or_note_content() {
        let v2 = br#"{
            "schema_version":"estudio-daw.project.v2",
            "project_id":"v2-arrangement",
            "transport":{"tempo_bpm":111.0,"time_signature":{"numerator":4,"denominator":4}},
            "tracks":[
                {"id":"lead-17","name":"Lead","kind":"midi","notes":[{"midi_key":64,"velocity":0.75,"time_beats":1.0,"duration_beats":0.5}],"instrument":{"backend":"sine"}},
                {"id":"audio-23","name":"Audio","kind":"audio","notes":[],"audio_channels":2}
            ],
            "midi_clips":[],
            "audio_clips":[],
            "import_provenance":{"format":"estudio-daw","format_version":"2","source_file":"","warnings":[]}
        }"#;

        let project = load_project_json(v2).unwrap();
        assert_eq!(project.schema_version, "estudio-daw.project.v5");
        assert_eq!(
            project
                .tracks
                .iter()
                .map(|track| track.id.as_str())
                .collect::<Vec<_>>(),
            ["lead-17", "audio-23"]
        );
        assert_eq!(project.tracks[0].notes[0].midi_key, 64);
        assert_eq!(project.tracks[0].role, TrackRole::Instrument);
        assert_eq!(project.tracks[1].role, TrackRole::Audio);
        assert_eq!(project.tracks[1].channel_config.input_channels, Some(2));
        assert!(project.midi_clips.is_empty());
        assert!(project.audio_clips.is_empty());
    }

    #[test]
    fn migrates_v3_to_current_format_and_fills_role_specific_defaults() {
        let v3 = br##"{
            "schema_version":"estudio-daw.project.v3",
            "project_id":"v3-audio",
            "transport":{"tempo_bpm":100.0,"time_signature":{"numerator":4,"denominator":4}},
            "tracks":[{"id":"audio-1","name":"Audio","kind":"audio","notes":[],"audio_channels":2,"color":"#ca7850","mixer":{"active":false,"mute":true,"solo":false,"gain_db":-3.0,"pan":0.25}}],
            "midi_clips":[],
            "audio_clips":[],
            "import_provenance":{"format":"estudio-daw","format_version":"3","source_file":"","warnings":[]}
        }"##;

        let project = load_project_json(v3).unwrap();
        let track = &project.tracks[0];
        assert_eq!(project.schema_version, "estudio-daw.project.v5");
        assert_eq!(track.role, TrackRole::Audio);
        assert_eq!(track.channel_config.input_channels, Some(2));
        assert_eq!(track.channel_config.output_channels, 2);
        assert_eq!(track.color, "#ca7850");
        assert!(track.mixer.mute);
        assert_eq!(track.mixer.pan, 0.25);
    }

    #[test]
    fn migrates_v4_media_into_source_catalog_and_ordered_playlist() {
        let v4 = br##"{
            "schema_version":"estudio-daw.project.v4",
            "project_id":"v4-regions",
            "transport":{"tempo_bpm":120.0,"time_signature":{"numerator":4,"denominator":4}},
            "tracks":[{"id":"audio-1","name":"Guitar","kind":"audio","role":"audio","channel_config":{"input_channels":2,"output_channels":2},"color":"#58a6b8","mixer":{"active":true,"mute":false,"solo":false,"gain_db":0.0,"pan":0.0},"notes":[],"audio_channels":2,"media_source":{"original_path":"audio/guitar.wav","original_signature":"size:96","original_hash":"sha256:abc","proxy":null},"instrument":null}],
            "midi_clips":[],
            "audio_clips":[{"id":"region-1","name":"Verse","track_id":"audio-1","start_tick":960,"source_start_samples":2400,"duration_samples":24000,"sample_rate":48000,"channels":2,"gain_db":-2.0,"fade_in_samples":100,"fade_out_samples":200}],
            "import_provenance":{"format":"internal","format_version":"4","source_file":"","warnings":[]}
        }"##;
        let project = load_project_json(v4).unwrap();
        assert_eq!(project.schema_version, "estudio-daw.project.v5");
        assert!(project.tracks[0].media_source.is_none());
        assert_eq!(
            project.audio_sources[0].media.original_path,
            PathBuf::from("audio/guitar.wav")
        );
        assert_eq!(project.audio_sources[0].media.original_signature, "size:96");
        assert_eq!(project.audio_sources[0].media.original_hash, "sha256:abc");
        assert_eq!(project.audio_sources[0].sample_rate_hz, Some(48_000));
        assert_eq!(project.audio_sources[0].channels, Some(2));
        assert_eq!(
            project.audio_clips[0].source_id.as_deref(),
            Some("source-audio-1")
        );
        assert_eq!(project.audio_clips[0].start_tick, 960);
        assert_eq!(project.audio_clips[0].source_start_samples, 2400);
        assert_eq!(project.audio_clips[0].duration_samples, 24000);
        assert_eq!(project.audio_clips[0].gain_db, -2.0);
        assert_eq!(project.audio_clips[0].fade_in_samples, 100);
        assert_eq!(project.audio_playlists[0].region_ids, ["region-1"]);
        assert_eq!(project.validate_track_contracts(), Ok(()));
    }

    #[test]
    fn clip_slots_reference_existing_clip_without_copying_content() {
        let mut project = import_fixture().unwrap().project;
        let midi_track_id = project
            .tracks
            .iter()
            .find(|t| t.kind == TrackKind::Midi)
            .unwrap()
            .id
            .clone();
        project.midi_clips.push(MidiClip {
            id: "midi-shared".into(),
            name: "Phrase".into(),
            track_id: midi_track_id.clone(),
            start_tick: 0,
            duration_ticks: 960,
            take: MidiTake {
                ppq: 480,
                tempo_bpm: 120,
                duration_micros: 1_000_000,
                events: Vec::new(),
            },
        });
        project.scenes.push(Scene {
            id: "scene-a".into(),
            name: "Intro".into(),
        });
        project.clip_slots.push(ClipSlot {
            id: "slot-a".into(),
            scene_id: "scene-a".into(),
            track_id: midi_track_id,
            clip: Some(ClipReference::Midi("midi-shared".into())),
        });
        assert_eq!(project.validate_track_contracts(), Ok(()));
        let restored: Project =
            serde_json::from_slice(&serde_json::to_vec(&project).unwrap()).unwrap();
        assert_eq!(restored.midi_clips.len(), 1);
        assert_eq!(
            restored.clip_slots[0].clip,
            Some(ClipReference::Midi("midi-shared".into()))
        );
        restored.validate_track_contracts().unwrap();
    }

    #[test]
    fn rejects_clip_slot_referencing_another_tracks_clip() {
        let mut project = import_fixture().unwrap().project;
        let first = project
            .tracks
            .iter()
            .find(|t| t.kind == TrackKind::Midi)
            .unwrap()
            .id
            .clone();
        let second = "midi-second".to_owned();
        project
            .tracks
            .push(Track::new(&second, "Other", TrackKind::Midi, TrackRole::Instrument).unwrap());
        project.midi_clips.push(MidiClip {
            id: "midi-owned-by-first".into(),
            name: "Phrase".into(),
            track_id: first,
            start_tick: 0,
            duration_ticks: 960,
            take: MidiTake {
                ppq: 480,
                tempo_bpm: 120,
                duration_micros: 1_000_000,
                events: Vec::new(),
            },
        });
        project.scenes.push(Scene {
            id: "scene-a".into(),
            name: "Intro".into(),
        });
        project.clip_slots.push(ClipSlot {
            id: "slot-invalid".into(),
            scene_id: "scene-a".into(),
            track_id: second,
            clip: Some(ClipReference::Midi("midi-owned-by-first".into())),
        });
        assert_eq!(
            project.validate_persisted_contracts(),
            Err(TrackValidationError::InvalidClipSlot)
        );
    }

    #[test]
    fn all_track_roles_have_valid_serializable_construction_contracts() {
        let role_kinds = [
            (TrackKind::Midi, TrackRole::Midi),
            (TrackKind::Midi, TrackRole::Instrument),
            (TrackKind::Audio, TrackRole::Audio),
            (TrackKind::Audio, TrackRole::Bus),
            (TrackKind::Audio, TrackRole::Return),
            (TrackKind::Audio, TrackRole::Master),
        ];
        for (index, (kind, role)) in role_kinds.into_iter().enumerate() {
            let track = Track::new(format!("track-{index}"), "Track", kind, role).unwrap();
            track.validate().unwrap();
            assert_eq!(track.channel_config.output_channels, 2);
            if role == TrackRole::Instrument {
                assert_eq!(track.instrument, Some(InstrumentConfig::Sine));
            }
            if role == TrackRole::Audio {
                assert_eq!(track.channel_config.input_channels, Some(2));
                assert_eq!(track.audio_channels, Some(2));
            }
            if matches!(role, TrackRole::Bus | TrackRole::Return | TrackRole::Master) {
                assert_eq!(track.channel_config.input_channels, None);
                assert!(track.media_source.is_none());
            }
            let encoded = serde_json::to_vec(&track).unwrap();
            let decoded: Track = serde_json::from_slice(&encoded).unwrap();
            assert_eq!(decoded, track);
        }
        assert_eq!(
            Track::new("bad", "Bad", TrackKind::Midi, TrackRole::Audio),
            Err(TrackValidationError::RoleKindMismatch)
        );
    }

    #[test]
    fn project_track_validation_rejects_duplicate_ids_and_multiple_masters() {
        let mut project = load_project_json(br#"{
            "schema_version":"estudio-daw.project.v4",
            "project_id":"invalid-tracks",
            "transport":{"tempo_bpm":120.0,"time_signature":{"numerator":4,"denominator":4}},
            "tracks":[],
            "import_provenance":{"format":"estudio-daw","format_version":"4","source_file":"","warnings":[]}
        }"#).unwrap();
        let master_a =
            Track::new("master-a", "Master A", TrackKind::Audio, TrackRole::Master).unwrap();
        let mut master_b =
            Track::new("master-b", "Master B", TrackKind::Audio, TrackRole::Master).unwrap();
        project.tracks.extend([master_a.clone(), master_b.clone()]);
        assert_eq!(
            project.validate_track_contracts(),
            Err(TrackValidationError::MultipleMasterTracks)
        );
        master_b.id = master_a.id.clone();
        project.tracks = vec![master_a, master_b];
        assert_eq!(
            project.validate_track_contracts(),
            Err(TrackValidationError::DuplicateId("master-a".into()))
        );
    }

    #[test]
    fn current_schema_repairs_missing_role_and_channel_defaults_from_track_kind() {
        let partial = br#"{
            "schema_version":"estudio-daw.project.v4",
            "project_id":"partial-current-format",
            "transport":{"tempo_bpm":120.0,"time_signature":{"numerator":4,"denominator":4}},
            "tracks":[{"id":"audio","name":"Audio","kind":"audio","notes":[],"audio_channels":1}],
            "import_provenance":{"format":"estudio-daw","format_version":"4","source_file":"","warnings":[]}
        }"#;
        let project = load_project_json(partial).unwrap();
        assert_eq!(project.tracks[0].role, TrackRole::Audio);
        assert_eq!(project.tracks[0].channel_config.input_channels, Some(1));
        assert_eq!(project.tracks[0].channel_config.output_channels, 2);
    }

    #[test]
    fn serializes_soundfont_instrument_reference_without_embedding_asset() {
        let config = InstrumentConfig::FluidSynth {
            soundfont: SoundFontReference {
                path: "/usr/share/soundfonts/FluidR3_GM.sf2".into(),
                sha256: Some("sha256:abc123".into()),
            },
            bank: 0,
            program: 0,
        };

        let encoded = serde_json::to_vec(&config).unwrap();
        let decoded: InstrumentConfig = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded, config);
        assert!(!String::from_utf8(encoded).unwrap().contains("sample_data"));
    }

    #[test]
    fn project_save_reopen_preserves_soundfont_reference_and_preset() {
        let project_json = br#"{
            "schema_version":"estudio-daw.project.v2",
            "project_id":"soundfont-session",
            "transport":{"tempo_bpm":92.0,"time_signature":{"numerator":4,"denominator":4}},
            "tracks":[{
                "id":"keys","name":"Keys","kind":"midi","notes":[],
                "instrument":{"backend":"fluid_synth","soundfont":{"path":"/banks/keys.sf2","sha256":"sha256:deadbeef"},"bank":1,"program":10}
            }],
            "import_provenance":{"format":"estudio-daw","format_version":"2","source_file":"","warnings":[]}
        }"#;

        let opened = load_project_json(project_json).unwrap();
        let saved = serde_json::to_vec(&opened).unwrap();
        let reopened = load_project_json(&saved).unwrap();

        assert_eq!(reopened.tracks[0].instrument, opened.tracks[0].instrument);
        assert!(!String::from_utf8(saved).unwrap().contains("sample_data"));
    }

    #[test]
    fn refuses_to_load_a_project_from_a_future_schema() {
        let future = br#"{"schema_version":"estudio-daw.project.v99"}"#;
        assert!(matches!(
            load_project_json(future),
            Err(ProjectJsonError::UnsupportedVersion(version))
                if version == "estudio-daw.project.v99"
        ));
    }

    #[test]
    fn refuses_a_non_string_schema_version() {
        assert!(matches!(
            load_project_json(br#"{"schema_version":1}"#),
            Err(ProjectJsonError::InvalidVersionType)
        ));
    }

    #[test]
    fn imports_minimal_fixture_from_zip() {
        let result = import_fixture().unwrap();
        assert_eq!(result.project.transport.tempo_bpm, 92.0);
        assert_eq!(result.project.transport.time_signature.denominator, 4);
        assert_eq!(result.project.tracks.len(), 2);
        assert_eq!(result.project.tracks[0].notes.len(), 3);
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.contains("plugins")));
        assert!(result.project.midi_clips.is_empty());
    }

    #[test]
    fn keeps_audio_track_without_notes() {
        let result = import_fixture().unwrap();
        let audio = result
            .project
            .tracks
            .iter()
            .find(|track| track.kind == TrackKind::Audio)
            .unwrap();
        assert!(audio.notes.is_empty());
        assert_eq!(audio.audio_channels, Some(1));
    }

    #[test]
    fn attaches_media_source_only_to_audio_tracks_and_survives_json() {
        let mut project = import_fixture().unwrap().project;
        let audio_id = project
            .tracks
            .iter()
            .find(|track| track.kind == TrackKind::Audio)
            .unwrap()
            .id
            .clone();
        let original = std::env::temp_dir().join(format!(
            "estudio-daw-track-source-{}.wav",
            std::process::id()
        ));
        std::fs::write(&original, b"audio").unwrap();
        let source = MediaSource {
            original_path: original.clone(),
            original_signature: "test-signature".into(),
            original_hash: "sha256:test".into(),
            proxy: None,
        };
        attach_media_source(&mut project, &audio_id, source).unwrap();
        let clip_id = add_audio_clip(
            &mut project,
            &audio_id,
            "Guitar region",
            960,
            0,
            48_000,
            48_000,
            2,
        )
        .unwrap();
        assert_eq!(clip_id, "audio-clip-1");
        set_audio_clip_gain(&mut project, &clip_id, -3.0).unwrap();
        set_audio_clip_fades(&mut project, &clip_id, 2_400, 2_400).unwrap();
        trim_audio_clip(&mut project, &clip_id, 4_800, 24_000).unwrap();
        let json = serde_json::to_vec(&project).unwrap();
        let restored: Project = serde_json::from_slice(&json).unwrap();
        assert_eq!(restored.audio_sources.len(), 1);
        assert_eq!(restored.audio_sources[0].owner_track_id, audio_id);
        assert_eq!(restored.audio_sources[0].media.original_path, original);
        assert_eq!(
            restored.audio_clips[0].source_id.as_deref(),
            Some(restored.audio_sources[0].id.as_str())
        );
        assert_eq!(restored.audio_playlists[0].region_ids, [clip_id]);
        assert_eq!(restored.audio_clips[0].start_tick, 960);
        assert_eq!(restored.audio_clips[0].duration_samples, 24_000);
        assert_eq!(restored.audio_clips[0].source_start_samples, 4_800);
        assert_eq!(restored.audio_clips[0].gain_db, -3.0);
        assert_eq!(restored.audio_sources[0].sample_rate_hz, Some(48_000));
        assert_eq!(restored.audio_sources[0].channels, Some(2));
        assert_eq!(std::fs::read(&original).unwrap(), b"audio");
        std::fs::remove_file(original).unwrap();
    }

    #[test]
    fn project_history_publishes_transactions_and_supports_undo_redo() {
        let project = import_fixture().unwrap().project;
        let audio_id = project
            .tracks
            .iter()
            .find(|track| track.kind == TrackKind::Audio)
            .unwrap()
            .id
            .clone();
        let original =
            std::env::temp_dir().join(format!("estudio-daw-history-{}.wav", std::process::id()));
        std::fs::write(&original, b"audio").unwrap();
        let source = MediaSource {
            original_path: original.clone(),
            original_signature: "test-signature".into(),
            original_hash: "sha256:test".into(),
            proxy: None,
        };
        let mut history = ProjectHistory::new(project);

        history
            .transact("attach source", |project| {
                attach_media_source(project, &audio_id, source)
            })
            .unwrap();
        assert!(history.can_undo());
        assert_eq!(history.project().unwrap().audio_sources.len(), 1);

        history.undo();
        assert!(history.can_redo());
        assert!(history.project().unwrap().audio_sources.is_empty());
        history.redo();
        assert_eq!(history.project().unwrap().audio_sources.len(), 1);
        let events = history.drain_events();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].revision, 1);
        assert_eq!(events[1].revision, 2);
        assert_eq!(events[2].revision, 3);
        assert_eq!(history.snapshot().unwrap().revision, 3);
        assert!(history.drain_events().is_empty());
        std::fs::remove_file(original).unwrap();
    }

    #[test]
    fn exports_and_reimports_project_round_trip() {
        let source = import_fixture().unwrap();
        let (bytes, export) = export_dawproject(&source.project).unwrap();
        assert!(!export.warnings.is_empty());
        let round_trip = import_dawproject(&bytes).unwrap();
        assert_eq!(round_trip.project.transport, source.project.transport);
        assert_eq!(round_trip.project.tracks.len(), source.project.tracks.len());
        assert_eq!(
            round_trip.project.tracks[0].notes,
            source.project.tracks[0].notes
        );
    }

    #[test]
    fn attaches_midi_take_to_first_midi_track() {
        let mut project = import_fixture().unwrap().project;
        let take = MidiTake {
            ppq: 480,
            tempo_bpm: 92,
            duration_micros: 1_000_000,
            events: vec![RecordedMidiEvent {
                tick: 480,
                micros_since_start: 500_000,
                source: MidiSource { client: 3, port: 0 },
                message: RecordedMidiMessage::NoteOn {
                    channel: 1,
                    note: 64,
                    velocity: 100,
                },
            }],
        };
        let id = attach_midi_take(&mut project, take, "KeyLab take").unwrap();
        assert_eq!(id, "midi-clip-1");
        assert_eq!(project.midi_clips[0].track_id, "track-midi");
        assert_eq!(project.midi_clips[0].duration_ticks, 480);
        assert_eq!(project.midi_clips[0].name, "KeyLab take");
    }

    #[test]
    fn rejects_take_when_project_has_no_midi_track() {
        let mut project = Project {
            schema_version: "test".into(),
            project_id: "test".into(),
            transport: Transport {
                tempo_bpm: 120.0,
                time_signature: TimeSignature {
                    numerator: 4,
                    denominator: 4,
                },
                loop_range: None,
            },
            tracks: vec![],
            audio_sources: vec![],
            audio_playlists: vec![],
            scenes: vec![],
            clip_slots: vec![],
            midi_clips: vec![],
            audio_clips: vec![],
            import_provenance: ImportProvenance {
                format: "internal".into(),
                format_version: "1".into(),
                source_file: "".into(),
                warnings: vec![],
            },
        };
        let take = MidiTake {
            ppq: 480,
            tempo_bpm: 120,
            duration_micros: 0,
            events: vec![],
        };
        assert_eq!(
            attach_midi_take(&mut project, take, "empty"),
            Err(AttachTakeError::NoMidiTrack)
        );
    }

    #[test]
    fn quantizes_notes_but_preserves_controllers() {
        let mut project = import_fixture().unwrap().project;
        let take = MidiTake {
            ppq: 480,
            tempo_bpm: 92,
            duration_micros: 1_000_000,
            events: vec![
                RecordedMidiEvent {
                    tick: 113,
                    micros_since_start: 0,
                    source: MidiSource { client: 3, port: 0 },
                    message: RecordedMidiMessage::NoteOn {
                        channel: 1,
                        note: 64,
                        velocity: 100,
                    },
                },
                RecordedMidiEvent {
                    tick: 117,
                    micros_since_start: 0,
                    source: MidiSource { client: 3, port: 0 },
                    message: RecordedMidiMessage::ControlChange {
                        channel: 1,
                        controller: 1,
                        value: 32,
                    },
                },
            ],
        };
        attach_midi_take(&mut project, take, "take").unwrap();
        assert_eq!(quantize_midi_clip(&mut project, "midi-clip-1", 120), Ok(1));
        assert_eq!(project.midi_clips[0].take.events[0].tick, 120);
        assert_eq!(project.midi_clips[0].take.events[1].tick, 117);
    }

    #[test]
    fn splits_midi_clip_and_rearticulates_notes_with_channel_state() {
        let mut project = import_fixture().unwrap().project;
        let track_id = project
            .tracks
            .iter()
            .find(|track| track.kind == TrackKind::Midi)
            .unwrap()
            .id
            .clone();
        let source = MidiSource { client: 2, port: 1 };
        project.midi_clips.push(MidiClip {
            id: "split-source".into(),
            name: "Frase".into(),
            track_id,
            start_tick: 960,
            duration_ticks: 960,
            take: MidiTake {
                ppq: 960,
                tempo_bpm: 120,
                duration_micros: 500_000,
                events: vec![
                    RecordedMidiEvent {
                        tick: 120,
                        micros_since_start: 62_500,
                        source: source.clone(),
                        message: RecordedMidiMessage::NoteOn {
                            channel: 0,
                            note: 60,
                            velocity: 90,
                        },
                    },
                    RecordedMidiEvent {
                        tick: 240,
                        micros_since_start: 125_000,
                        source: source.clone(),
                        message: RecordedMidiMessage::ControlChange {
                            channel: 0,
                            controller: 64,
                            value: 127,
                        },
                    },
                    RecordedMidiEvent {
                        tick: 360,
                        micros_since_start: 187_500,
                        source: source.clone(),
                        message: RecordedMidiMessage::NoteOn {
                            channel: 0,
                            note: 62,
                            velocity: 75,
                        },
                    },
                    RecordedMidiEvent {
                        tick: 480,
                        micros_since_start: 250_000,
                        source: source.clone(),
                        message: RecordedMidiMessage::NoteOff {
                            channel: 0,
                            note: 62,
                            release_velocity: 8,
                        },
                    },
                    RecordedMidiEvent {
                        tick: 720,
                        micros_since_start: 375_000,
                        source: source.clone(),
                        message: RecordedMidiMessage::NoteOff {
                            channel: 0,
                            note: 60,
                            release_velocity: 12,
                        },
                    },
                ],
            },
        });

        split_midi_clip(&mut project, "split-source", 480, "split-source-right").unwrap();
        let left = &project.midi_clips[0];
        let right = &project.midi_clips[1];
        assert_eq!(left.id, "split-source");
        assert_eq!(left.start_tick, 960);
        assert_eq!(left.duration_ticks, 480);
        assert_eq!(right.id, "split-source-right");
        assert_eq!(right.start_tick, 1_440);
        assert_eq!(right.duration_ticks, 480);
        assert!(left.take.events.iter().any(|event| {
            event.tick == 480
                && matches!(event.message, RecordedMidiMessage::NoteOff { note: 60, .. })
        }));
        assert!(left.take.events.iter().any(|event| {
            event.tick == 480
                && matches!(event.message, RecordedMidiMessage::NoteOff { note: 62, .. })
        }));
        assert!(!right.take.events.iter().any(|event| {
            matches!(event.message, RecordedMidiMessage::NoteOn { note: 62, .. })
        }));
        assert!(right.take.events.iter().any(|event| {
            event.tick == 0
                && matches!(
                    event.message,
                    RecordedMidiMessage::NoteOn {
                        note: 60,
                        velocity: 90,
                        ..
                    }
                )
        }));
        assert!(right.take.events.iter().any(|event| {
            event.tick == 0
                && matches!(
                    event.message,
                    RecordedMidiMessage::ControlChange {
                        controller: 64,
                        value: 127,
                        ..
                    }
                )
        }));
        assert!(right.take.events.iter().any(|event| {
            event.tick == 240
                && matches!(
                    event.message,
                    RecordedMidiMessage::NoteOff {
                        note: 60,
                        release_velocity: 12,
                        ..
                    }
                )
        }));
    }
}
