//! Diagnóstico fuera del hilo de audio.
//!
//! Este crate no participa en el callback RT. Lee el inventario publicado por
//! PipeWire y devuelve datos neutrales para la CLI y, más adelante, la UI.

use alsa::seq::{Addr, ClientIter, EvCtrl, EvNote, EventType, PortCap, PortIter, PortType, Seq};
use std::{ffi::CString, fs, path::Path, process::Command};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DiagnosticsError {
    #[error("no se pudo ejecutar pw-dump: {0}")]
    Command(#[from] std::io::Error),
    #[error("pw-dump terminó con error: {0}")]
    PipeWire(String),
    #[error("respuesta JSON inválida de pw-dump: {0}")]
    Json(#[from] serde_json::Error),
    #[error("error ALSA: {0}")]
    Alsa(#[from] alsa::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    pub id: u64,
    pub media_class: String,
    pub name: String,
    pub description: String,
}

pub fn enumerate_pipewire() -> Result<Vec<DeviceInfo>, DiagnosticsError> {
    let output = Command::new("pw-dump").output()?;
    if !output.status.success() {
        return Err(DiagnosticsError::PipeWire(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    let objects: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)?;
    Ok(objects
        .into_iter()
        .filter_map(|object| {
            let object_type = object.get("type")?.as_str()?;
            if !matches!(
                object_type,
                "PipeWire:Interface:Node" | "PipeWire:Interface:Device"
            ) {
                return None;
            }
            let id = object.get("id")?.as_u64()?;
            let props = object.get("info")?.get("props")?;
            let media_class = props
                .get("media.class")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let name = props
                .get("node.name")
                .or_else(|| props.get("device.name"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let description = props
                .get("node.description")
                .or_else(|| props.get("device.description"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string();
            Some(DeviceInfo {
                id,
                media_class,
                name,
                description,
            })
        })
        .collect())
}

pub fn midi_devices() -> Result<Vec<DeviceInfo>, DiagnosticsError> {
    let mut devices = enumerate_pipewire().unwrap_or_default();
    devices.retain(|device| device.media_class.to_ascii_lowercase().contains("midi"));
    devices.extend(enumerate_alsa_midi()?);
    Ok(devices)
}

pub fn audio_devices() -> Result<Vec<DeviceInfo>, DiagnosticsError> {
    Ok(enumerate_pipewire()?
        .into_iter()
        .filter(|device| {
            let class = device.media_class.to_ascii_lowercase();
            class.contains("audio") || class.contains("source") || class.contains("sink")
        })
        .collect())
}

/// Descubre puertos MIDI USB que ALSA conoce aunque PipeWire aún no los haya
/// expuesto como nodos. Es útil para controladores como KeyLab.
pub fn enumerate_alsa_midi() -> Result<Vec<DeviceInfo>, DiagnosticsError> {
    let cards = fs::read_to_string("/proc/asound/cards")?;
    let mut descriptions = std::collections::HashMap::new();
    for line in cards.lines() {
        let Some((number, rest)) = line.trim_start().split_once(' ') else {
            continue;
        };
        let Ok(card) = number.parse::<u64>() else {
            continue;
        };
        let description = rest
            .split_once(':')
            .map(|(_, description)| description.trim().to_string())
            .unwrap_or_else(|| rest.trim().to_string());
        descriptions.insert(card, description);
    }

    let mut devices = Vec::new();
    let snd = Path::new("/dev/snd");
    if !snd.exists() {
        return Ok(devices);
    }
    for entry in fs::read_dir(snd)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(rest) = name.strip_prefix("midiC") else {
            continue;
        };
        let Some((card, device)) = rest.split_once('D') else {
            continue;
        };
        let (Ok(card), Ok(device)) = (card.parse::<u64>(), device.parse::<u64>()) else {
            continue;
        };
        devices.push(DeviceInfo {
            id: 10_000 + card * 100 + device,
            media_class: "ALSA/MIDI".into(),
            name: format!("hw:{card},{device}"),
            description: descriptions
                .get(&card)
                .cloned()
                .unwrap_or_else(|| "ALSA MIDI".into()),
        });
    }
    devices.sort_by_key(|device| device.id);
    Ok(devices)
}

/// Busca un puerto ALSA MIDI por nombre y devuelve su dirección sequencer.
pub fn find_alsa_midi_port(query: &str) -> Result<Option<(Addr, String)>, DiagnosticsError> {
    let seq = Seq::open(None, None, false)?;
    let query = query.to_ascii_lowercase();
    for client in ClientIter::new(&seq) {
        for port in PortIter::new(&seq, client.get_client()) {
            let name = port.get_name()?.to_string();
            let capabilities = port.get_capability();
            if !capabilities.contains(PortCap::READ | PortCap::SUBS_READ) {
                continue;
            }
            if name.to_ascii_lowercase().contains(&query)
                || client.get_name()?.to_ascii_lowercase().contains(&query)
            {
                return Ok(Some((
                    port.addr(),
                    format!("{}: {}", client.get_name()?, name),
                )));
            }
        }
    }
    Ok(None)
}

/// Conecta un puerto ALSA MIDI a un puerto local y bloquea mostrando eventos.
pub fn monitor_alsa_midi(query: &str) -> Result<(), DiagnosticsError> {
    let seq = Seq::open(None, None, false)?;
    let Some((source, label)) = find_alsa_midi_port(query)? else {
        return Err(DiagnosticsError::PipeWire(format!(
            "no se encontró puerto MIDI ALSA para '{query}'"
        )));
    };
    let client_name = CString::new("Estudio DAW MIDI Monitor").expect("literal sin NUL");
    seq.set_client_name(&client_name)?;
    let port_name = CString::new("input").expect("literal sin NUL");
    let local_port = seq.create_simple_port(
        &port_name,
        PortCap::WRITE | PortCap::SUBS_WRITE,
        PortType::MIDI_GENERIC | PortType::APPLICATION,
    )?;
    let subscription = alsa::seq::PortSubscribe::empty()?;
    subscription.set_sender(source);
    subscription.set_dest(Addr {
        client: seq.client_id()?,
        port: local_port,
    });
    seq.subscribe_port(&subscription)?;
    println!("Escuchando {label}. Presiona Ctrl+C para salir.");
    let mut input = seq.input();
    loop {
        let event = input.event_input()?;
        match event.get_type() {
            EventType::Noteon | EventType::Noteoff | EventType::Keypress => {
                if let Some(note) = event.get_data::<EvNote>() {
                    println!(
                        "{:?} ch={} note={} velocity={}",
                        event.get_type(),
                        note.channel + 1,
                        note.note,
                        note.velocity
                    );
                }
            }
            EventType::Controller
            | EventType::Pitchbend
            | EventType::Chanpress
            | EventType::Pgmchange => {
                if let Some(control) = event.get_data::<EvCtrl>() {
                    println!(
                        "{:?} ch={} param={} value={}",
                        event.get_type(),
                        control.channel + 1,
                        control.param,
                        control.value
                    );
                }
            }
            event_type => println!("{:?} source={:?}", event_type, event.get_source()),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_pipewire_node_fixture() {
        let raw = r#"[{"id":42,"type":"PipeWire:Interface:Node","info":{"props":{"media.class":"Audio/Source","node.name":"alsa_input.usb","node.description":"AudioBox USB 96"}}}]"#;
        let objects: Vec<serde_json::Value> = serde_json::from_str(raw).unwrap();
        let object = &objects[0];
        assert_eq!(object["id"].as_u64(), Some(42));
        assert_eq!(
            object["info"]["props"]["node.description"],
            "AudioBox USB 96"
        );
    }

    #[test]
    fn parses_alsa_midi_device_name() {
        let rest = "midiC3D0".strip_prefix("midiC").unwrap();
        let (card, device) = rest.split_once('D').unwrap();
        assert_eq!(card.parse::<u64>().unwrap(), 3);
        assert_eq!(device.parse::<u64>().unwrap(), 0);
    }
}
