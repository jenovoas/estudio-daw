//! Modelo canónico mínimo y adaptador de intercambio DAWproject.
//!
//! DAWproject sólo es una frontera de intercambio. El modelo interno conserva
//! información adicional como escala, procedencia y estado de proxies.

use estudio_daw_midi_engine::MidiTake;
use quick_xml::{de::from_str, escape::escape};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    process::Command,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProxyAsset {
    pub path: PathBuf,
    pub source_signature: String,
    #[serde(default)]
    pub source_hash: String,
    pub profile: String,
}

#[derive(Debug, Error)]
pub enum ProxyJobError {
    #[error("no se pudo leer la fuente del proxy: {0}")]
    Io(#[from] std::io::Error),
    #[error("la fuente cambió antes de generar el proxy")]
    SourceChanged,
    #[error("el proxy generado está vacío")]
    EmptyOutput,
    #[error("el transcoder ffmpeg terminó con error: {0}")]
    TranscoderFailed(String),
    #[error("el proxy no cumple el perfil de audio solicitado: {0}")]
    InvalidAudioOutput(String),
    #[error("ya existe una generación activa para este proxy")]
    AlreadyBuilding,
    #[error("no se pudo leer o escribir el manifiesto de proxies: {0}")]
    Manifest(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyCacheState {
    Missing,
    Building,
    Ready,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioProxyProfile {
    pub id: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub codec: String,
    pub bitrate_kbps: u32,
}

impl AudioProxyProfile {
    pub fn opus_preview() -> Self {
        Self {
            id: "audio-opus-preview-v1".into(),
            sample_rate: 48_000,
            channels: 2,
            codec: "libopus".into(),
            bitrate_kbps: 128,
        }
    }
}

/// Administrador de caché de proxies derivados.
#[derive(Debug, Clone)]
pub struct ProxyCacheManager {
    root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProxyCacheManifest {
    pub schema_version: String,
    #[serde(default)]
    pub entries: Vec<ProxyManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProxyManifestEntry {
    pub original_path: PathBuf,
    pub original_hash: String,
    pub proxy_path: PathBuf,
    pub source_signature: String,
    pub profile: String,
}

impl ProxyCacheManager {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.root.join("proxy-manifest.json")
    }

    pub fn load_manifest(&self) -> Result<ProxyCacheManifest, ProxyJobError> {
        let path = self.manifest_path();
        if !path.is_file() {
            return Ok(ProxyCacheManifest {
                schema_version: "estudio-daw.proxy-manifest.v1".into(),
                entries: Vec::new(),
            });
        }
        serde_json::from_slice(&fs::read(path)?)
            .map_err(|error| ProxyJobError::Manifest(error.to_string()))
    }

    fn save_manifest(&self, manifest: &ProxyCacheManifest) -> Result<(), ProxyJobError> {
        fs::create_dir_all(&self.root)?;
        let path = self.manifest_path();
        let temporary = path.with_file_name(".proxy-manifest.json.tmp");
        let bytes = serde_json::to_vec_pretty(manifest)
            .map_err(|error| ProxyJobError::Manifest(error.to_string()))?;
        fs::write(&temporary, bytes)?;
        if let Err(error) = fs::rename(&temporary, &path) {
            let _ = fs::remove_file(&temporary);
            return Err(error.into());
        }
        Ok(())
    }

    fn record_asset(&self, source: &MediaSource, asset: &ProxyAsset) -> Result<(), ProxyJobError> {
        let mut manifest = self.load_manifest()?;
        manifest.entries.retain(|entry| {
            !(entry.original_path == source.original_path && entry.profile == asset.profile)
        });
        manifest.entries.push(ProxyManifestEntry {
            original_path: source.original_path.clone(),
            original_hash: source.original_hash.clone(),
            proxy_path: asset.path.clone(),
            source_signature: asset.source_signature.clone(),
            profile: asset.profile.clone(),
        });
        self.save_manifest(&manifest)
    }

    /// Restaura desde el manifiesto el proxy asociado a una fuente y perfil.
    pub fn hydrate_source(
        &self,
        source: &mut MediaSource,
        profile: &AudioProxyProfile,
    ) -> Result<ProxyCacheState, ProxyJobError> {
        let manifest = self.load_manifest()?;
        if let Some(entry) = manifest.entries.iter().find(|entry| {
            entry.original_path == source.original_path
                && entry.original_hash == source.original_hash
                && entry.profile == profile.id
        }) {
            source.proxy = Some(ProxyAsset {
                path: entry.proxy_path.clone(),
                source_signature: entry.source_signature.clone(),
                source_hash: entry.original_hash.clone(),
                profile: entry.profile.clone(),
            });
        }
        let destination = self.destination_for(source, profile);
        Ok(source.proxy_cache_state(destination, &profile.id))
    }

    /// Devuelve una ubicación estable para un perfil y una fuente concretos.
    /// El hash evita colisiones entre archivos con el mismo nombre y permite
    /// conservar versiones antiguas hasta que una limpieza explícita las retire.
    pub fn destination_for(&self, source: &MediaSource, profile: &AudioProxyProfile) -> PathBuf {
        let stem = source
            .original_path
            .file_stem()
            .and_then(|value| value.to_str())
            .map(sanitize_filename)
            .unwrap_or_else(|| "media".into());
        let hash = source
            .original_hash
            .strip_prefix("sha256:")
            .unwrap_or(&source.original_hash);
        let short_hash = &hash[..hash.len().min(16)];
        self.root
            .join(format!("{stem}-{short_hash}-{}.ogg", profile.id))
    }

    /// Reutiliza una entrada lista o genera una nueva si falta/está obsoleta.
    pub fn ensure_audio_proxy(
        &self,
        source: &mut MediaSource,
        profile: &AudioProxyProfile,
    ) -> Result<ProxyCacheState, ProxyJobError> {
        let destination = self.destination_for(source, profile);
        match source.proxy_cache_state(&destination, &profile.id) {
            ProxyCacheState::Ready => return Ok(ProxyCacheState::Ready),
            ProxyCacheState::Building => return Err(ProxyJobError::AlreadyBuilding),
            ProxyCacheState::Missing | ProxyCacheState::Stale => {}
        }
        let asset = generate_audio_proxy_ffmpeg(source, &destination, profile)?;
        self.record_asset(source, &asset)?;
        source.proxy = Some(asset);
        Ok(ProxyCacheState::Ready)
    }
}

fn sanitize_filename(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "media".into()
    } else {
        sanitized
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaRepresentation {
    Original,
    Proxy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedMedia {
    pub path: PathBuf,
    pub representation: MediaRepresentation,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MediaResolveError {
    #[error("el archivo original no existe: {0}")]
    OriginalMissing(PathBuf),
    #[error("no hay un proxy válido disponible")]
    ProxyUnavailable,
}

impl MediaSource {
    pub fn from_original(path: impl Into<PathBuf>) -> Result<Self, std::io::Error> {
        let path = path.into();
        Ok(Self {
            original_signature: media_signature(&path)?,
            original_hash: content_hash(&path)?,
            original_path: path,
            proxy: None,
        })
    }

    pub fn proxy_cache_state(
        &self,
        destination: impl AsRef<Path>,
        profile: &str,
    ) -> ProxyCacheState {
        let destination = destination.as_ref();
        if building_marker(destination).is_file() {
            return ProxyCacheState::Building;
        }
        let Some(proxy) = self.proxy.as_ref() else {
            return ProxyCacheState::Missing;
        };
        if proxy.path == destination
            && proxy.profile == profile
            && proxy.source_signature == self.original_signature
            && (proxy.source_hash.is_empty() || proxy.source_hash == self.original_hash)
            && destination.is_file()
        {
            ProxyCacheState::Ready
        } else {
            ProxyCacheState::Stale
        }
    }

    /// Resuelve la representación sin copiar, re-encodear ni modificar medios.
    pub fn resolve(&self, policy: ProxyPolicy) -> Result<ResolvedMedia, MediaResolveError> {
        let proxy_is_valid = self.proxy.as_ref().is_some_and(|proxy| {
            proxy.source_signature == self.original_signature
                && (proxy.source_hash.is_empty() || proxy.source_hash == self.original_hash)
                && proxy.path.is_file()
        });
        match policy {
            ProxyPolicy::Original if self.original_path.is_file() => Ok(ResolvedMedia {
                path: self.original_path.clone(),
                representation: MediaRepresentation::Original,
            }),
            ProxyPolicy::Proxy if proxy_is_valid => Ok(ResolvedMedia {
                path: self
                    .proxy
                    .as_ref()
                    .expect("proxy_is_valid implica proxy")
                    .path
                    .clone(),
                representation: MediaRepresentation::Proxy,
            }),
            ProxyPolicy::Auto if proxy_is_valid => Ok(ResolvedMedia {
                path: self
                    .proxy
                    .as_ref()
                    .expect("proxy_is_valid implica proxy")
                    .path
                    .clone(),
                representation: MediaRepresentation::Proxy,
            }),
            ProxyPolicy::Auto if self.original_path.is_file() => Ok(ResolvedMedia {
                path: self.original_path.clone(),
                representation: MediaRepresentation::Original,
            }),
            ProxyPolicy::Original | ProxyPolicy::Auto => Err(MediaResolveError::OriginalMissing(
                self.original_path.clone(),
            )),
            ProxyPolicy::Proxy => Err(MediaResolveError::ProxyUnavailable),
        }
    }
}

/// Materializa un proxy de identidad con publicación atómica.
///
/// Este primer backend conserva los bytes originales para validar la frontera
/// de caché y procedencia. Los transcoders (PCM reducido, Opus, FLAC proxy o
/// thumbnails) se conectarán detrás de la misma operación sin cambiar la
/// semántica de publicación: escribir temporal, validar y renombrar.
pub fn generate_proxy(
    source: &MediaSource,
    destination: impl AsRef<Path>,
    profile: impl Into<String>,
) -> Result<ProxyAsset, ProxyJobError> {
    let destination = destination.as_ref();
    let current_hash = content_hash(&source.original_path)?;
    if !source.original_hash.is_empty() && current_hash != source.original_hash {
        return Err(ProxyJobError::SourceChanged);
    }
    if let Some(parent) = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let temp = destination.with_file_name(format!(
        ".{}.tmp-{}",
        destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("proxy"),
        std::process::id()
    ));
    fs::copy(&source.original_path, &temp)?;
    let metadata = fs::metadata(&temp)?;
    if metadata.len() == 0 {
        let _ = fs::remove_file(&temp);
        return Err(ProxyJobError::EmptyOutput);
    }
    let generated_hash = content_hash(&temp)?;
    if generated_hash != current_hash {
        let _ = fs::remove_file(&temp);
        return Err(ProxyJobError::SourceChanged);
    }
    fs::rename(&temp, destination)?;
    Ok(ProxyAsset {
        path: destination.to_path_buf(),
        source_signature: media_signature(&source.original_path)?,
        source_hash: current_hash,
        profile: profile.into(),
    })
}

/// Genera un proxy de audio ligero con ffmpeg fuera del hilo de audio.
///
/// La salida se escribe en un temporal, se valida con ffprobe y sólo entonces
/// se publica. El original se hashea antes y después de la transcodificación
/// para impedir que una fuente modificada produzca un proxy aparentemente
/// válido.
pub fn generate_audio_proxy_ffmpeg(
    source: &MediaSource,
    destination: impl AsRef<Path>,
    profile: &AudioProxyProfile,
) -> Result<ProxyAsset, ProxyJobError> {
    let destination = destination.as_ref();
    let source_hash = content_hash(&source.original_path)?;
    if !source.original_hash.is_empty() && source_hash != source.original_hash {
        return Err(ProxyJobError::SourceChanged);
    }
    if let Some(parent) = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let _marker = BuildingMarker::create(destination)?;
    let temp = destination.with_file_name(format!(
        ".{}.tmp-{}",
        destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("proxy"),
        std::process::id()
    ));
    let output = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y", "-i"])
        .arg(&source.original_path)
        .args(["-vn", "-ac"])
        .arg(profile.channels.to_string())
        .args(["-ar"])
        .arg(profile.sample_rate.to_string())
        .args(["-c:a"])
        .arg(&profile.codec)
        .args(["-b:a"])
        .arg(format!("{}k", profile.bitrate_kbps))
        // El temporal no tiene extensión visible; el perfil actual usa Ogg
        // como contenedor para Opus y por eso se declara explícitamente.
        .args(["-f", "ogg"])
        .arg(&temp)
        .output()?;
    if !output.status.success() {
        let _ = fs::remove_file(&temp);
        return Err(ProxyJobError::TranscoderFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    if let Err(error) = validate_audio_proxy(&temp, profile) {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }
    if content_hash(&source.original_path)? != source_hash {
        let _ = fs::remove_file(&temp);
        return Err(ProxyJobError::SourceChanged);
    }
    if let Err(error) = fs::rename(&temp, destination) {
        let _ = fs::remove_file(&temp);
        return Err(error.into());
    }
    Ok(ProxyAsset {
        path: destination.to_path_buf(),
        source_signature: media_signature(&source.original_path)?,
        source_hash,
        profile: profile.id.clone(),
    })
}

fn building_marker(destination: &Path) -> PathBuf {
    destination.with_file_name(format!(
        ".{}.building",
        destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("proxy")
    ))
}

struct BuildingMarker {
    path: PathBuf,
}

impl BuildingMarker {
    fn create(destination: &Path) -> Result<Self, ProxyJobError> {
        let path = building_marker(destination);
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    ProxyJobError::AlreadyBuilding
                } else {
                    ProxyJobError::Io(error)
                }
            })?;
        Ok(Self { path })
    }
}

impl Drop for BuildingMarker {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn validate_audio_proxy(path: &Path, profile: &AudioProxyProfile) -> Result<(), ProxyJobError> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=sample_rate,channels",
            "-of",
            "csv=p=0:s=,",
        ])
        .arg(path)
        .output()?;
    if !output.status.success() {
        return Err(ProxyJobError::InvalidAudioOutput(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    let values = String::from_utf8_lossy(&output.stdout);
    let Some((sample_rate, channels)) = values.trim().split_once(',') else {
        return Err(ProxyJobError::InvalidAudioOutput(
            "ffprobe no devolvió sample rate y canales".into(),
        ));
    };
    let valid_rate = sample_rate.parse::<u32>().ok() == Some(profile.sample_rate);
    let valid_channels = channels.parse::<u16>().ok() == Some(profile.channels);
    if !valid_rate || !valid_channels {
        return Err(ProxyJobError::InvalidAudioOutput(format!(
            "esperado {} Hz / {} canales, obtenido {} Hz / {} canales",
            profile.sample_rate, profile.channels, sample_rate, channels
        )));
    }
    Ok(())
}

/// Firma barata para invalidar proxies cuando cambia el archivo fuente.
pub fn media_signature(path: impl AsRef<Path>) -> Result<String, std::io::Error> {
    let metadata = std::fs::metadata(path.as_ref())?;
    let modified = metadata
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    Ok(format!("{}:{}", metadata.len(), modified.as_nanos()))
}

pub fn content_hash(path: impl AsRef<Path>) -> Result<String, std::io::Error> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub schema_version: String,
    pub project_id: String,
    pub transport: Transport,
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub midi_clips: Vec<MidiClip>,
    #[serde(default)]
    pub audio_clips: Vec<AudioClip>,
    pub import_provenance: ImportProvenance,
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
    pub start_tick: u64,
    pub source_start_samples: u64,
    pub duration_samples: u64,
    pub sample_rate: u32,
    pub channels: u16,
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
    pub notes: Vec<Note>,
    pub audio_channels: Option<u32>,
    /// Procedencia original/proxy de una pista de audio. Las pistas MIDI no
    /// deben usar este campo; `None` conserva compatibilidad con project.json
    /// anteriores.
    #[serde(default)]
    pub media_source: Option<MediaSource>,
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

#[derive(Debug, Error)]
pub enum ProjectMediaError {
    #[error("no existe la pista '{0}'")]
    TrackNotFound(String),
    #[error("la pista '{0}' no es de audio")]
    NotAudioTrack(String),
    #[error("la pista '{0}' no tiene una fuente de audio asociada")]
    MissingSource(String),
    #[error("falló la generación del proxy: {0}")]
    Proxy(#[from] ProxyJobError),
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

/// Importa un archivo `.dawproject` empaquetado.
pub fn import_dawproject(path: impl AsRef<Path>) -> Result<ImportResult, ProjectError> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
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
            Track {
                id: track.id,
                name: track.name,
                kind,
                notes,
                audio_channels: track.channel.and_then(|c| c.audio_channels),
                media_source: None,
            }
        })
        .collect();

    let project = Project {
        schema_version: "estudio-daw.project.v1".into(),
        project_id: "imported-dawproject".into(),
        transport: Transport {
            tempo_bpm: tempo,
            time_signature: signature,
        },
        tracks,
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
        .iter_mut()
        .find(|track| track.id == track_id)
        .ok_or_else(|| MediaAttachError::TrackNotFound(track_id.into()))?;
    if track.kind != TrackKind::Audio {
        return Err(MediaAttachError::NotAudioTrack(track_id.into()));
    }
    track.media_source = Some(source);
    Ok(())
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
    if track.media_source.is_none() {
        return Err(AudioClipError::MissingSource(track_id.into()));
    }
    if duration_samples == 0 {
        return Err(AudioClipError::InvalidDuration);
    }
    if sample_rate == 0 {
        return Err(AudioClipError::InvalidSampleRate);
    }
    if !(1..=32).contains(&channels) {
        return Err(AudioClipError::InvalidChannels);
    }
    let id = format!("audio-clip-{}", project.audio_clips.len() + 1);
    project.audio_clips.push(AudioClip {
        id: id.clone(),
        name: name.into(),
        track_id: track_id.into(),
        start_tick,
        source_start_samples,
        duration_samples,
        sample_rate,
        channels,
        gain_db: 0.0,
        fade_in_samples: 0,
        fade_out_samples: 0,
    });
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

/// Garantiza el proxy de una pista y actualiza el proyecto sólo tras una
/// publicación válida en la caché.
pub fn ensure_track_audio_proxy(
    project: &mut Project,
    track_id: &str,
    cache: &ProxyCacheManager,
    profile: &AudioProxyProfile,
) -> Result<ProxyCacheState, ProjectMediaError> {
    let track = project
        .tracks
        .iter_mut()
        .find(|track| track.id == track_id)
        .ok_or_else(|| ProjectMediaError::TrackNotFound(track_id.into()))?;
    if track.kind != TrackKind::Audio {
        return Err(ProjectMediaError::NotAudioTrack(track_id.into()));
    }
    let source = track
        .media_source
        .as_mut()
        .ok_or_else(|| ProjectMediaError::MissingSource(track_id.into()))?;
    Ok(cache.ensure_audio_proxy(source, profile)?)
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
            estudio_daw_midi_engine::RecordedMidiMessage::NoteOn { .. }
                | estudio_daw_midi_engine::RecordedMidiMessage::NoteOff { .. }
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

/// Exporta el modelo interno como un contenedor `.dawproject` mínimo.
///
/// Los artefactos de audio, proxies, plugins y análisis no se inventan durante
/// la exportación: se reportan como warnings hasta que exista su adaptador.
pub fn export_dawproject(
    project: &Project,
    path: impl AsRef<Path>,
) -> Result<ExportResult, ProjectError> {
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
    let file = File::create(path)?;
    let mut archive = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    archive.start_file("project.xml", options)?;
    archive.write_all(project_xml.as_bytes())?;
    archive.start_file("metadata.xml", options)?;
    archive.write_all(metadata_xml.as_bytes())?;
    archive.finish()?;
    Ok(ExportResult { warnings })
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
    use estudio_daw_midi_engine::{MidiSource, RecordedMidiEvent, RecordedMidiMessage};

    #[test]
    fn imports_minimal_fixture_from_zip() {
        let result =
            import_dawproject("../../tests/fixtures/dawproject/minimal.dawproject").unwrap();
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
    fn auto_policy_prefers_fresh_proxy_and_never_changes_original() {
        let root = std::env::temp_dir().join(format!("estudio-daw-proxy-{}", std::process::id()));
        let original = root.join("original.wav");
        let proxy = root.join("proxy.wav");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(&original, b"original").unwrap();
        std::fs::write(&proxy, b"proxy").unwrap();
        let mut source = MediaSource::from_original(&original).unwrap();
        source.proxy = Some(ProxyAsset {
            path: proxy.clone(),
            source_signature: source.original_signature.clone(),
            source_hash: source.original_hash.clone(),
            profile: "audio-preview-f32".into(),
        });

        let resolved = source.resolve(ProxyPolicy::Auto).unwrap();
        assert_eq!(resolved.path, proxy);
        assert_eq!(resolved.representation, MediaRepresentation::Proxy);
        assert_eq!(std::fs::read(&original).unwrap(), b"original");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stale_proxy_is_rejected_and_original_policy_is_explicit() {
        let root =
            std::env::temp_dir().join(format!("estudio-daw-stale-proxy-{}", std::process::id()));
        let original = root.join("original.wav");
        let proxy = root.join("proxy.wav");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(&original, b"original").unwrap();
        std::fs::write(&proxy, b"proxy").unwrap();
        let mut source = MediaSource::from_original(&original).unwrap();
        source.proxy = Some(ProxyAsset {
            path: proxy,
            source_signature: "stale-signature".into(),
            source_hash: "stale-hash".into(),
            profile: "audio-preview-f32".into(),
        });

        assert_eq!(
            source.resolve(ProxyPolicy::Proxy),
            Err(MediaResolveError::ProxyUnavailable)
        );
        assert_eq!(
            source
                .resolve(ProxyPolicy::Original)
                .unwrap()
                .representation,
            MediaRepresentation::Original
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn proxy_job_publishes_atomically_and_records_strong_hash() {
        let root = std::env::temp_dir().join(format!("estudio-daw-job-{}", std::process::id()));
        let original = root.join("original.wav");
        let proxy = root.join("cache").join("proxy.wav");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(&original, b"source-audio").unwrap();
        let mut source = MediaSource::from_original(&original).unwrap();
        assert_eq!(
            source.proxy_cache_state(&proxy, "identity-test"),
            ProxyCacheState::Missing
        );

        let asset = generate_proxy(&source, &proxy, "identity-test").unwrap();
        assert_eq!(asset.profile, "identity-test");
        assert_eq!(asset.source_hash, content_hash(&original).unwrap());
        assert_eq!(std::fs::read(&proxy).unwrap(), b"source-audio");
        source.proxy = Some(asset);
        assert_eq!(
            source.proxy_cache_state(&proxy, "identity-test"),
            ProxyCacheState::Ready
        );
        assert_eq!(source.resolve(ProxyPolicy::Auto).unwrap().path, proxy);
        assert!(!root.join("cache/.proxy.wav.tmp").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cache_state_distinguishes_building_and_stale_profiles() {
        let root =
            std::env::temp_dir().join(format!("estudio-daw-cache-state-{}", std::process::id()));
        let original = root.join("original.wav");
        let proxy = root.join("preview.ogg");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(&original, b"audio").unwrap();
        std::fs::write(&proxy, b"proxy").unwrap();
        let mut source = MediaSource::from_original(&original).unwrap();
        let marker = building_marker(&proxy);
        std::fs::write(&marker, b"building").unwrap();
        assert_eq!(
            source.proxy_cache_state(&proxy, "audio-opus-preview-v1"),
            ProxyCacheState::Building
        );
        std::fs::remove_file(marker).unwrap();
        source.proxy = Some(ProxyAsset {
            path: proxy.clone(),
            source_signature: source.original_signature.clone(),
            source_hash: source.original_hash.clone(),
            profile: "old-profile".into(),
        });
        assert_eq!(
            source.proxy_cache_state(&proxy, "audio-opus-preview-v1"),
            ProxyCacheState::Stale
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn proxy_cache_destination_is_stable_and_sanitized() {
        let root =
            std::env::temp_dir().join(format!("estudio-daw-cache-path-{}", std::process::id()));
        let original = root.join("Mi canción (master).wav");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(&original, b"audio").unwrap();
        let source = MediaSource::from_original(&original).unwrap();
        let manager = ProxyCacheManager::new(root.join("cache"));
        let profile = AudioProxyProfile::opus_preview();
        let first = manager.destination_for(&source, &profile);
        let second = manager.destination_for(&source, &profile);
        assert_eq!(first, second);
        assert!(first.to_string_lossy().contains("Mi_canci_n__master_"));
        assert!(first
            .to_string_lossy()
            .ends_with("audio-opus-preview-v1.ogg"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn proxy_manifest_hydrates_ready_entry_after_restart() {
        let root =
            std::env::temp_dir().join(format!("estudio-daw-manifest-{}", std::process::id()));
        let original = root.join("song.wav");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(&original, b"audio").unwrap();
        let manager = ProxyCacheManager::new(root.join("cache"));
        let profile = AudioProxyProfile::opus_preview();
        let source = MediaSource::from_original(&original).unwrap();
        let destination = manager.destination_for(&source, &profile);
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::write(&destination, b"proxy").unwrap();
        manager
            .save_manifest(&ProxyCacheManifest {
                schema_version: "estudio-daw.proxy-manifest.v1".into(),
                entries: vec![ProxyManifestEntry {
                    original_path: source.original_path.clone(),
                    original_hash: source.original_hash.clone(),
                    proxy_path: destination.clone(),
                    source_signature: source.original_signature.clone(),
                    profile: profile.id.clone(),
                }],
            })
            .unwrap();

        let mut restored = MediaSource::from_original(&original).unwrap();
        assert_eq!(
            manager.hydrate_source(&mut restored, &profile).unwrap(),
            ProxyCacheState::Ready
        );
        assert_eq!(
            restored.resolve(ProxyPolicy::Auto).unwrap().path,
            destination
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn keeps_audio_track_without_notes() {
        let result =
            import_dawproject("../../tests/fixtures/dawproject/minimal.dawproject").unwrap();
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
        let mut project = import_dawproject("../../tests/fixtures/dawproject/minimal.dawproject")
            .unwrap()
            .project;
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
        let source = MediaSource::from_original(&original).unwrap();
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
        assert_eq!(
            restored
                .tracks
                .iter()
                .find(|track| track.id == audio_id)
                .unwrap()
                .media_source
                .as_ref()
                .unwrap()
                .original_path,
            original
        );
        assert_eq!(restored.audio_clips[0].start_tick, 960);
        assert_eq!(restored.audio_clips[0].duration_samples, 24_000);
        assert_eq!(restored.audio_clips[0].source_start_samples, 4_800);
        assert_eq!(restored.audio_clips[0].gain_db, -3.0);
        std::fs::remove_file(original).unwrap();
    }

    #[test]
    fn exports_and_reimports_project_round_trip() {
        let source =
            import_dawproject("../../tests/fixtures/dawproject/minimal.dawproject").unwrap();
        let output = std::env::temp_dir().join(format!(
            "estudio-daw-roundtrip-{}.dawproject",
            std::process::id()
        ));
        let export = export_dawproject(&source.project, &output).unwrap();
        assert!(!export.warnings.is_empty());
        let round_trip = import_dawproject(&output).unwrap();
        assert_eq!(round_trip.project.transport, source.project.transport);
        assert_eq!(round_trip.project.tracks.len(), source.project.tracks.len());
        assert_eq!(
            round_trip.project.tracks[0].notes,
            source.project.tracks[0].notes
        );
        std::fs::remove_file(output).unwrap();
    }

    #[test]
    fn attaches_midi_take_to_first_midi_track() {
        let mut project = import_dawproject("../../tests/fixtures/dawproject/minimal.dawproject")
            .unwrap()
            .project;
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
            },
            tracks: vec![],
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
        let mut project = import_dawproject("../../tests/fixtures/dawproject/minimal.dawproject")
            .unwrap()
            .project;
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
}
