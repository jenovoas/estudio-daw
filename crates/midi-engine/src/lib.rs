//! Modelo de eventos MIDI y grabación de takes.
//!
//! Este primer recorder vive fuera del callback de audio. La integración RT
//! definitiva recibirá eventos desde un ring bounded preasignado.

use alsa::seq::{PortCap, PortType, Seq};
use estudio_daw_runtime_diagnostics::{
    find_alsa_midi_port, normalize_alsa_event, NormalizedMidiEvent,
};
use serde::{Deserialize, Serialize};
use std::{
    ffi::CString,
    thread,
    time::{Duration, Instant},
};
use thiserror::Error;

pub const DEFAULT_PPQ: u32 = 480;

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

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RecorderError {
    #[error("el recorder no está activo")]
    NotRecording,
    #[error("el recorder ya está activo")]
    AlreadyRecording,
}

#[derive(Debug, Error)]
pub enum LiveRecordError {
    #[error("no se pudo encontrar el puerto MIDI: {0}")]
    Discovery(#[from] estudio_daw_runtime_diagnostics::DiagnosticsError),
    #[error("error ALSA durante la grabación: {0}")]
    Alsa(#[from] alsa::Error),
    #[error("error del recorder: {0}")]
    Recorder(#[from] RecorderError),
}

pub struct MidiRecorder {
    ppq: u32,
    tempo_bpm: u32,
    started_at: Option<Instant>,
    events: Vec<RecordedMidiEvent>,
}

impl MidiRecorder {
    pub fn new(tempo_bpm: u32, ppq: u32) -> Self {
        Self {
            ppq,
            tempo_bpm,
            started_at: None,
            events: Vec::new(),
        }
    }

    pub fn start(&mut self) -> Result<(), RecorderError> {
        if self.started_at.is_some() {
            return Err(RecorderError::AlreadyRecording);
        }
        self.events.clear();
        self.started_at = Some(Instant::now());
        Ok(())
    }

    pub fn record(&mut self, event: &NormalizedMidiEvent) -> Result<(), RecorderError> {
        let started_at = self.started_at.ok_or(RecorderError::NotRecording)?;
        self.record_at(event, started_at.elapsed())
    }

    pub fn record_at(
        &mut self,
        event: &NormalizedMidiEvent,
        elapsed: Duration,
    ) -> Result<(), RecorderError> {
        if self.started_at.is_none() {
            return Err(RecorderError::NotRecording);
        }
        let micros_since_start = elapsed.as_micros() as u64;
        let tick = ((elapsed.as_secs_f64() * f64::from(self.tempo_bpm) * f64::from(self.ppq))
            / 60.0)
            .round() as u64;
        let (source, message) = convert_event(event);
        self.events.push(RecordedMidiEvent {
            tick,
            micros_since_start,
            source,
            message,
        });
        Ok(())
    }

    pub fn stop(&mut self) -> Result<MidiTake, RecorderError> {
        let started_at = self.started_at.take().ok_or(RecorderError::NotRecording)?;
        Ok(MidiTake {
            ppq: self.ppq,
            tempo_bpm: self.tempo_bpm,
            duration_micros: started_at.elapsed().as_micros() as u64,
            events: std::mem::take(&mut self.events),
        })
    }

    pub fn is_recording(&self) -> bool {
        self.started_at.is_some()
    }
}

/// Graba una toma MIDI real desde un puerto ALSA durante `duration`.
///
/// Usa polling fuera del callback RT; la futura integración del motor usará un
/// ring bounded y el transporte musical compartido.
pub fn record_alsa_midi(
    query: &str,
    duration: Duration,
    tempo_bpm: u32,
) -> Result<MidiTake, LiveRecordError> {
    let seq = Seq::open(None, None, true)?;
    let Some((source, label)) = find_alsa_midi_port(query)? else {
        return Err(LiveRecordError::Discovery(
            estudio_daw_runtime_diagnostics::DiagnosticsError::PipeWire(format!(
                "no se encontró puerto MIDI para '{query}'"
            )),
        ));
    };
    let client_name = CString::new("Estudio DAW MIDI Recorder").expect("literal sin NUL");
    seq.set_client_name(&client_name)?;
    let port_name = CString::new("recording-input").expect("literal sin NUL");
    let local_port = seq.create_simple_port(
        &port_name,
        PortCap::WRITE | PortCap::SUBS_WRITE,
        PortType::MIDI_GENERIC | PortType::APPLICATION,
    )?;
    let subscription = alsa::seq::PortSubscribe::empty()?;
    subscription.set_sender(source);
    subscription.set_dest(alsa::seq::Addr {
        client: seq.client_id()?,
        port: local_port,
    });
    seq.subscribe_port(&subscription)?;

    println!(
        "Grabando MIDI desde {label} durante {:.1}s...",
        duration.as_secs_f64()
    );
    let mut recorder = MidiRecorder::new(tempo_bpm, DEFAULT_PPQ);
    recorder.start()?;
    let deadline = Instant::now() + duration;
    let mut input = seq.input();
    while Instant::now() < deadline {
        if input.event_input_pending(false)? > 0 {
            let event = input.event_input()?;
            let normalized = normalize_alsa_event(&event);
            recorder.record(&normalized)?;
        } else {
            thread::sleep(Duration::from_millis(1));
        }
    }
    let take = recorder.stop()?;
    println!("Toma finalizada: {} eventos.", take.events.len());
    Ok(take)
}

fn convert_event(event: &NormalizedMidiEvent) -> (MidiSource, RecordedMidiMessage) {
    let source = match event {
        NormalizedMidiEvent::NoteOn { source, .. }
        | NormalizedMidiEvent::NoteOff { source, .. }
        | NormalizedMidiEvent::KeyPressure { source, .. }
        | NormalizedMidiEvent::ControlChange { source, .. }
        | NormalizedMidiEvent::PitchBend { source, .. }
        | NormalizedMidiEvent::ChannelPressure { source, .. }
        | NormalizedMidiEvent::ProgramChange { source, .. }
        | NormalizedMidiEvent::SysEx { source, .. }
        | NormalizedMidiEvent::Other { source, .. } => MidiSource {
            client: source.client,
            port: source.port,
        },
    };
    let message = match event {
        NormalizedMidiEvent::NoteOn {
            channel,
            note,
            velocity,
            ..
        } => RecordedMidiMessage::NoteOn {
            channel: *channel,
            note: *note,
            velocity: *velocity,
        },
        NormalizedMidiEvent::NoteOff {
            channel,
            note,
            release_velocity,
            ..
        } => RecordedMidiMessage::NoteOff {
            channel: *channel,
            note: *note,
            release_velocity: *release_velocity,
        },
        NormalizedMidiEvent::KeyPressure {
            channel,
            note,
            pressure,
            ..
        } => RecordedMidiMessage::KeyPressure {
            channel: *channel,
            note: *note,
            pressure: *pressure,
        },
        NormalizedMidiEvent::ControlChange {
            channel,
            controller,
            value,
            ..
        } => RecordedMidiMessage::ControlChange {
            channel: *channel,
            controller: *controller,
            value: *value,
        },
        NormalizedMidiEvent::PitchBend { channel, value, .. } => RecordedMidiMessage::PitchBend {
            channel: *channel,
            value: *value,
        },
        NormalizedMidiEvent::ChannelPressure {
            channel, pressure, ..
        } => RecordedMidiMessage::ChannelPressure {
            channel: *channel,
            pressure: *pressure,
        },
        NormalizedMidiEvent::ProgramChange {
            channel, program, ..
        } => RecordedMidiMessage::ProgramChange {
            channel: *channel,
            program: *program,
        },
        NormalizedMidiEvent::SysEx { bytes, .. } => RecordedMidiMessage::SysEx {
            bytes: bytes.clone(),
        },
        NormalizedMidiEvent::Other { .. } => RecordedMidiMessage::Other,
    };
    (source, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alsa::seq::Addr;

    fn note_on() -> NormalizedMidiEvent {
        NormalizedMidiEvent::NoteOn {
            channel: 0,
            note: 60,
            velocity: 100,
            source: Addr {
                client: 28,
                port: 0,
            },
        }
    }

    #[test]
    fn records_events_as_musical_ticks() {
        let mut recorder = MidiRecorder::new(120, DEFAULT_PPQ);
        recorder.start().unwrap();
        recorder
            .record_at(&note_on(), Duration::from_millis(500))
            .unwrap();
        let take = recorder.stop().unwrap();
        assert_eq!(take.events[0].tick, 480);
        assert_eq!(take.events[0].micros_since_start, 500_000);
    }

    #[test]
    fn preserves_pressure_and_source() {
        let event = NormalizedMidiEvent::KeyPressure {
            channel: 9,
            note: 36,
            pressure: 82,
            source: Addr {
                client: 28,
                port: 0,
            },
        };
        let mut recorder = MidiRecorder::new(92, DEFAULT_PPQ);
        recorder.start().unwrap();
        recorder
            .record_at(&event, Duration::from_millis(100))
            .unwrap();
        let take = recorder.stop().unwrap();
        assert_eq!(
            take.events[0].source,
            MidiSource {
                client: 28,
                port: 0
            }
        );
        assert_eq!(
            take.events[0].message,
            RecordedMidiMessage::KeyPressure {
                channel: 9,
                note: 36,
                pressure: 82
            }
        );
    }

    #[test]
    fn rejects_events_outside_recording() {
        let mut recorder = MidiRecorder::new(120, DEFAULT_PPQ);
        assert_eq!(
            recorder.record_at(&note_on(), Duration::ZERO),
            Err(RecorderError::NotRecording)
        );
        assert_eq!(recorder.stop(), Err(RecorderError::NotRecording));
    }
}
