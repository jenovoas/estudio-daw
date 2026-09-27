//! Tipos portables y serializables para eventos y tomas MIDI.
//!
//! Este módulo no conoce dispositivos ni protocolos del sistema. Los motores
//! de entrada/salida convierten sus eventos concretos a estas estructuras.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MidiSource {
    pub client: i32,
    pub port: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RecordedMidiMessage {
    NoteOn {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    NoteOff {
        channel: u8,
        note: u8,
        release_velocity: u8,
    },
    KeyPressure {
        channel: u8,
        note: u8,
        pressure: u8,
    },
    ControlChange {
        channel: u8,
        controller: u32,
        value: i32,
    },
    PitchBend {
        channel: u8,
        value: i32,
    },
    ChannelPressure {
        channel: u8,
        pressure: i32,
    },
    ProgramChange {
        channel: u8,
        program: i32,
    },
    SysEx {
        bytes: Vec<u8>,
    },
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecordedMidiEvent {
    pub tick: u64,
    pub micros_since_start: u64,
    pub source: MidiSource,
    pub message: RecordedMidiMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MidiTake {
    pub ppq: u32,
    pub tempo_bpm: u32,
    pub duration_micros: u64,
    pub events: Vec<RecordedMidiEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializa_y_recupera_una_toma_sin_depender_del_dispositivo() {
        let take = MidiTake {
            ppq: 960,
            tempo_bpm: 120,
            duration_micros: 500_000,
            events: vec![RecordedMidiEvent {
                tick: 480,
                micros_since_start: 250_000,
                source: MidiSource { client: 3, port: 1 },
                message: RecordedMidiMessage::NoteOn {
                    channel: 0,
                    note: 60,
                    velocity: 96,
                },
            }],
        };

        let encoded = serde_json::to_vec(&take).unwrap();
        let decoded: MidiTake = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, take);
    }
}
