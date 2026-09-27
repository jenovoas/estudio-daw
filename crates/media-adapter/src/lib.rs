//! Operaciones de archivo y procesos auxiliares para fuentes y proxies de audio.
//!
//! Este adaptador depende del modelo portable para sus contratos serializables;
//! el modelo no depende de este módulo.

use estudio_daw_project_model::{MediaSource, ProxyAsset, ProxyPolicy};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};
use thiserror::Error;

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

/// Metadatos del primer flujo de audio decodificable del archivo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioFileMetadata {
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub duration_samples: u64,
}

/// Consulta metadatos técnicos sin cargar el PCM al proceso de interfaz.
pub fn inspect_audio_metadata(path: impl AsRef<Path>) -> Result<AudioFileMetadata, ProxyJobError> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=sample_rate,channels,duration:format=duration",
            "-of",
            "json",
        ])
        .arg(path.as_ref())
        .output()?;
    if !output.status.success() {
        return Err(ProxyJobError::InvalidAudioOutput(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    parse_audio_metadata_json(&output.stdout)
}

fn parse_audio_metadata_json(bytes: &[u8]) -> Result<AudioFileMetadata, ProxyJobError> {
    let document: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|error| ProxyJobError::InvalidAudioOutput(error.to_string()))?;
    let stream = document
        .get("streams")
        .and_then(serde_json::Value::as_array)
        .and_then(|streams| streams.first())
        .ok_or_else(|| {
            ProxyJobError::InvalidAudioOutput("ffprobe no encontró un flujo de audio".into())
        })?;
    let sample_rate_hz = stream
        .get("sample_rate")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .parse::<u32>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| {
            ProxyJobError::InvalidAudioOutput("frecuencia de muestreo inválida".into())
        })?;
    let channels = stream
        .get("channels")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u16::try_from(value).ok())
        .filter(|value| (1..=32).contains(value))
        .ok_or_else(|| ProxyJobError::InvalidAudioOutput("cantidad de canales inválida".into()))?;
    let duration_seconds = stream
        .get("duration")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| value.parse::<f64>().ok())
        .or_else(|| {
            document
                .get("format")
                .and_then(|format| format.get("duration"))
                .and_then(serde_json::Value::as_str)
                .and_then(|value| value.parse::<f64>().ok())
        })
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or_else(|| ProxyJobError::InvalidAudioOutput("duración de audio inválida".into()))?;
    Ok(AudioFileMetadata {
        sample_rate_hz,
        channels,
        duration_samples: (duration_seconds * f64::from(sample_rate_hz)).round() as u64,
    })
}

/// Crea un resumen min/max de tamaño fijo para mostrar la forma de onda.
/// Sólo decodifica una señal mono remuestreada y limita la lectura a diez minutos.
pub fn audio_waveform(
    path: impl AsRef<Path>,
    bins: usize,
) -> Result<Vec<(f32, f32)>, ProxyJobError> {
    let bins = bins.clamp(1, 4096);
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path.as_ref())
        .args([
            "-t", "600", "-vn", "-ac", "1", "-ar", "1000", "-f", "f32le", "pipe:1",
        ])
        .output()?;
    if !output.status.success() {
        return Err(ProxyJobError::TranscoderFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    let samples: Vec<f32> = output
        .stdout
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .filter(|sample| sample.is_finite())
        .collect();
    if samples.is_empty() {
        return Err(ProxyJobError::EmptyOutput);
    }
    Ok(reduce_samples_to_min_max(&samples, bins))
}

fn reduce_samples_to_min_max(samples: &[f32], bins: usize) -> Vec<(f32, f32)> {
    let count = bins.clamp(1, 4096).min(samples.len());
    (0..count)
        .map(|index| {
            let start = index * samples.len() / count;
            let end = ((index + 1) * samples.len() / count).max(start + 1);
            samples[start..end.min(samples.len())]
                .iter()
                .fold((1.0_f32, -1.0_f32), |(min, max), sample| {
                    (min.min(*sample), max.max(*sample))
                })
        })
        .collect()
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
        Ok(proxy_cache_state(source, destination, &profile.id))
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
        match proxy_cache_state(source, &destination, &profile.id) {
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

/// Inspecciona una ruta y crea los metadatos portables de su fuente.
pub fn inspect_media_source(path: impl Into<PathBuf>) -> Result<MediaSource, std::io::Error> {
    let path = path.into();
    Ok(MediaSource {
        original_signature: media_signature(&path)?,
        original_hash: content_hash(&path)?,
        original_path: path,
        proxy: None,
    })
}

pub fn proxy_cache_state(
    source: &MediaSource,
    destination: impl AsRef<Path>,
    profile: &str,
) -> ProxyCacheState {
    let destination = destination.as_ref();
    if building_marker(destination).is_file() {
        return ProxyCacheState::Building;
    }
    let Some(proxy) = source.proxy.as_ref() else {
        return ProxyCacheState::Missing;
    };
    if proxy.path == destination
        && proxy.profile == profile
        && proxy.source_signature == source.original_signature
        && (proxy.source_hash.is_empty() || proxy.source_hash == source.original_hash)
        && destination.is_file()
    {
        ProxyCacheState::Ready
    } else {
        ProxyCacheState::Stale
    }
}

/// Resuelve una representación utilizable sin modificar el origen.
pub fn resolve_media(
    source: &MediaSource,
    policy: ProxyPolicy,
) -> Result<ResolvedMedia, MediaResolveError> {
    let proxy_is_valid = source.proxy.as_ref().is_some_and(|proxy| {
        proxy.source_signature == source.original_signature
            && (proxy.source_hash.is_empty() || proxy.source_hash == source.original_hash)
            && proxy.path.is_file()
    });
    match policy {
        ProxyPolicy::Original if source.original_path.is_file() => Ok(ResolvedMedia {
            path: source.original_path.clone(),
            representation: MediaRepresentation::Original,
        }),
        ProxyPolicy::Proxy if proxy_is_valid => Ok(ResolvedMedia {
            path: source.proxy.as_ref().expect("proxy validado").path.clone(),
            representation: MediaRepresentation::Proxy,
        }),
        ProxyPolicy::Auto if proxy_is_valid => Ok(ResolvedMedia {
            path: source.proxy.as_ref().expect("proxy validado").path.clone(),
            representation: MediaRepresentation::Proxy,
        }),
        ProxyPolicy::Auto if source.original_path.is_file() => Ok(ResolvedMedia {
            path: source.original_path.clone(),
            representation: MediaRepresentation::Original,
        }),
        ProxyPolicy::Original | ProxyPolicy::Auto => Err(MediaResolveError::OriginalMissing(
            source.original_path.clone(),
        )),
        ProxyPolicy::Proxy => Err(MediaResolveError::ProxyUnavailable),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("estudio-daw-{name}-{}", std::process::id()))
    }

    #[test]
    fn reduce_samples_to_bounded_min_max_bins_preserves_extrema() {
        let bins = reduce_samples_to_min_max(&[-0.8, 0.2, -0.1, 0.9, 0.0], 2);
        assert_eq!(bins, vec![(-0.8, 0.2), (-0.1, 0.9)]);
        assert_eq!(reduce_samples_to_min_max(&[0.25], 512), vec![(0.25, 0.25)]);
    }

    #[test]
    fn metadata_parser_uses_container_duration_when_stream_duration_is_unavailable() {
        let metadata = parse_audio_metadata_json(
            br#"{"streams":[{"sample_rate":"44100","channels":2,"duration":"N/A"}],"format":{"duration":"2.0"}}"#,
        )
        .unwrap();
        assert_eq!(metadata.sample_rate_hz, 44_100);
        assert_eq!(metadata.channels, 2);
        assert_eq!(metadata.duration_samples, 88_200);
    }

    #[test]
    fn inspecciona_fuente_y_resuelve_proxy_fresco_sin_alterar_original() {
        let root = temporary_root("media-source");
        fs::create_dir_all(&root).unwrap();
        let original = root.join("original.wav");
        let proxy = root.join("proxy.wav");
        fs::write(&original, b"original").unwrap();
        fs::write(&proxy, b"proxy").unwrap();
        let mut source = inspect_media_source(&original).unwrap();
        source.proxy = Some(ProxyAsset {
            path: proxy.clone(),
            source_signature: source.original_signature.clone(),
            source_hash: source.original_hash.clone(),
            profile: "preview".into(),
        });

        let resolved = resolve_media(&source, ProxyPolicy::Auto).unwrap();
        assert_eq!(resolved.path, proxy);
        assert_eq!(resolved.representation, MediaRepresentation::Proxy);
        assert_eq!(fs::read(&original).unwrap(), b"original");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rechaza_proxy_obsoleto_y_permita_solicitar_el_original() {
        let root = temporary_root("stale-proxy");
        fs::create_dir_all(&root).unwrap();
        let original = root.join("original.wav");
        let proxy = root.join("proxy.wav");
        fs::write(&original, b"original").unwrap();
        fs::write(&proxy, b"proxy").unwrap();
        let mut source = inspect_media_source(&original).unwrap();
        source.proxy = Some(ProxyAsset {
            path: proxy,
            source_signature: "stale-signature".into(),
            source_hash: "stale-hash".into(),
            profile: "preview".into(),
        });

        assert_eq!(
            resolve_media(&source, ProxyPolicy::Proxy),
            Err(MediaResolveError::ProxyUnavailable)
        );
        assert_eq!(
            resolve_media(&source, ProxyPolicy::Original)
                .unwrap()
                .representation,
            MediaRepresentation::Original
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn publica_proxy_de_identidad_con_hash_fuerte_y_temporal_atomico() {
        let root = temporary_root("proxy-job");
        fs::create_dir_all(&root).unwrap();
        let original = root.join("source.wav");
        let proxy = root.join("cache").join("proxy.wav");
        fs::write(&original, b"source-audio").unwrap();
        let mut source = inspect_media_source(&original).unwrap();
        assert_eq!(
            proxy_cache_state(&source, &proxy, "identity"),
            ProxyCacheState::Missing
        );

        let asset = generate_proxy(&source, &proxy, "identity").unwrap();
        assert_eq!(asset.source_hash, content_hash(&original).unwrap());
        assert_eq!(fs::read(&proxy).unwrap(), b"source-audio");
        source.proxy = Some(asset);
        assert_eq!(
            proxy_cache_state(&source, &proxy, "identity"),
            ProxyCacheState::Ready
        );
        assert_eq!(
            resolve_media(&source, ProxyPolicy::Auto).unwrap().path,
            proxy
        );
        assert!(!root.join("cache/.proxy.wav.tmp").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cache_reporta_generacion_activa_y_perfil_obsoleto() {
        let root = temporary_root("cache-state");
        fs::create_dir_all(&root).unwrap();
        let original = root.join("original.wav");
        let proxy = root.join("preview.ogg");
        fs::write(&original, b"audio").unwrap();
        fs::write(&proxy, b"proxy").unwrap();
        let mut source = inspect_media_source(&original).unwrap();
        let marker = building_marker(&proxy);
        fs::write(&marker, b"building").unwrap();
        assert_eq!(
            proxy_cache_state(&source, &proxy, "preview"),
            ProxyCacheState::Building
        );
        fs::remove_file(marker).unwrap();
        source.proxy = Some(ProxyAsset {
            path: proxy.clone(),
            source_signature: source.original_signature.clone(),
            source_hash: source.original_hash.clone(),
            profile: "old-profile".into(),
        });
        assert_eq!(
            proxy_cache_state(&source, &proxy, "preview"),
            ProxyCacheState::Stale
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cache_guarda_ruta_estable_y_rehidrata_el_manifiesto() {
        let root = temporary_root("proxy-manifest");
        fs::create_dir_all(&root).unwrap();
        let original = root.join("Mi canción (master).wav");
        fs::write(&original, b"audio").unwrap();
        let manager = ProxyCacheManager::new(root.join("cache"));
        let profile = AudioProxyProfile::opus_preview();
        let source = inspect_media_source(&original).unwrap();
        let destination = manager.destination_for(&source, &profile);
        assert_eq!(destination, manager.destination_for(&source, &profile));
        assert!(destination
            .to_string_lossy()
            .contains("Mi_canci_n__master_"));
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::write(&destination, b"proxy").unwrap();
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

        let mut restored = inspect_media_source(&original).unwrap();
        assert_eq!(
            manager.hydrate_source(&mut restored, &profile).unwrap(),
            ProxyCacheState::Ready
        );
        assert_eq!(
            resolve_media(&restored, ProxyPolicy::Auto).unwrap().path,
            destination
        );
        fs::remove_dir_all(root).unwrap();
    }
}
