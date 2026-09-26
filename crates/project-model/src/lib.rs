//! Modelo canónico mínimo y adaptador de intercambio DAWproject.
//!
//! DAWproject sólo es una frontera de intercambio. El modelo interno conserva
//! información adicional como escala, procedencia y estado de proxies.

use estudio_daw_midi_engine::MidiTake;
use quick_xml::{de::from_str, escape::escape};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
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
    #[serde(default)]
    pub proxy: Option<ProxyAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProxyAsset {
    pub path: PathBuf,
    pub source_signature: String,
    pub profile: String,
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
            original_path: path,
            proxy: None,
        })
    }

    /// Resuelve la representación sin copiar, re-encodear ni modificar medios.
    pub fn resolve(&self, policy: ProxyPolicy) -> Result<ResolvedMedia, MediaResolveError> {
        let proxy_is_valid = self.proxy.as_ref().is_some_and(|proxy| {
            proxy.source_signature == self.original_signature && proxy.path.is_file()
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

/// Firma barata para invalidar proxies cuando cambia el archivo fuente.
pub fn media_signature(path: impl AsRef<Path>) -> Result<String, std::io::Error> {
    let metadata = std::fs::metadata(path.as_ref())?;
    let modified = metadata
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    Ok(format!("{}:{}", metadata.len(), modified.as_nanos()))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub schema_version: String,
    pub project_id: String,
    pub transport: Transport,
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub midi_clips: Vec<MidiClip>,
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
        import_provenance: ImportProvenance {
            format: "dawproject".into(),
            format_version: source.version,
            source_file: "project.xml".into(),
            warnings: warnings.clone(),
        },
    };
    Ok(ImportResult { project, warnings })
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
