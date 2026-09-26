//! Diagnóstico fuera del hilo de audio.
//!
//! Este crate no participa en el callback RT. Lee el inventario publicado por
//! PipeWire y devuelve datos neutrales para la CLI y, más adelante, la UI.

use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DiagnosticsError {
    #[error("no se pudo ejecutar pw-dump: {0}")]
    Command(#[from] std::io::Error),
    #[error("pw-dump terminó con error: {0}")]
    PipeWire(String),
    #[error("respuesta JSON inválida de pw-dump: {0}")]
    Json(#[from] serde_json::Error),
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
    Ok(enumerate_pipewire()?
        .into_iter()
        .filter(|device| device.media_class.to_ascii_lowercase().contains("midi"))
        .collect())
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
}
