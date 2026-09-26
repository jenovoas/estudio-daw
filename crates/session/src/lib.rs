//! Estado de sesión y bus de comandos de Estudio DAW.
//!
//! Este crate no abre dispositivos ni procesa audio. Su responsabilidad es
//! ofrecer una frontera pequeña y testeable entre MIDI/UI/IA y el transporte
//! que más adelante consumirá el motor de audio.

use serde::{Deserialize, Serialize};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransportState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct TransportSnapshot {
    pub state: TransportState,
    pub tempo_bpm: f64,
    pub position_ticks: u64,
    pub loop_enabled: bool,
    pub recording: bool,
    pub scene_index: u32,
    pub master_volume: f32,
}

impl Default for TransportSnapshot {
    fn default() -> Self {
        Self {
            state: TransportState::Stopped,
            tempo_bpm: 120.0,
            position_ticks: 0,
            loop_enabled: false,
            recording: false,
            scene_index: 0,
            master_volume: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SessionCommand {
    Play,
    Pause,
    TogglePlay,
    Stop,
    ToggleRecord,
    ToggleLoop,
    NextScene,
    PreviousScene,
    SetMasterVolume(f32),
    SetTempo(f64),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CommandBusError {
    #[error("el bus de comandos está lleno")]
    Full,
    #[error("el bus de comandos está cerrado")]
    Closed,
}

/// Bus bounded para que una ráfaga de MIDI nunca cree una cola infinita.
///
/// La escritura no bloquea: el productor recibe `Full` y puede decidir si
/// descarta, coalescea o registra el diagnóstico. El motor de audio consumirá
/// la cola fuera de su callback hasta migrar este contrato a un ring RT.
pub struct CommandBus {
    sender: SyncSender<SessionCommand>,
    receiver: Receiver<SessionCommand>,
}

impl CommandBus {
    pub fn bounded(capacity: usize) -> Self {
        let (sender, receiver) = mpsc::sync_channel(capacity.max(1));
        Self { sender, receiver }
    }

    pub fn dispatch(&self, command: SessionCommand) -> Result<(), CommandBusError> {
        self.sender.try_send(command).map_err(|error| match error {
            TrySendError::Full(_) => CommandBusError::Full,
            TrySendError::Disconnected(_) => CommandBusError::Closed,
        })
    }

    pub fn drain_into(&self, session: &mut Session) -> usize {
        let mut applied = 0;
        for command in self.receiver.try_iter() {
            session.apply(command);
            applied += 1;
        }
        applied
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Session {
    snapshot: TransportSnapshot,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            snapshot: TransportSnapshot::default(),
        }
    }
}

impl Session {
    pub fn snapshot(&self) -> TransportSnapshot {
        self.snapshot
    }

    pub fn apply(&mut self, command: SessionCommand) {
        match command {
            SessionCommand::Play => self.snapshot.state = TransportState::Playing,
            SessionCommand::Pause => {
                if self.snapshot.state == TransportState::Playing {
                    self.snapshot.state = TransportState::Paused;
                }
            }
            SessionCommand::TogglePlay => {
                self.snapshot.state = match self.snapshot.state {
                    TransportState::Playing => TransportState::Paused,
                    TransportState::Paused | TransportState::Stopped => TransportState::Playing,
                };
            }
            SessionCommand::Stop => {
                self.snapshot.state = TransportState::Stopped;
                self.snapshot.position_ticks = 0;
                self.snapshot.recording = false;
            }
            SessionCommand::ToggleRecord => {
                self.snapshot.recording = !self.snapshot.recording;
                if self.snapshot.recording && self.snapshot.state == TransportState::Stopped {
                    self.snapshot.state = TransportState::Playing;
                }
            }
            SessionCommand::ToggleLoop => self.snapshot.loop_enabled = !self.snapshot.loop_enabled,
            SessionCommand::NextScene => {
                self.snapshot.scene_index = self.snapshot.scene_index.saturating_add(1)
            }
            SessionCommand::PreviousScene => {
                self.snapshot.scene_index = self.snapshot.scene_index.saturating_sub(1)
            }
            SessionCommand::SetMasterVolume(value) => {
                self.snapshot.master_volume = value.clamp(0.0, 1.0)
            }
            SessionCommand::SetTempo(value) if value.is_finite() && value > 0.0 => {
                self.snapshot.tempo_bpm = value.clamp(20.0, 999.0)
            }
            SessionCommand::SetTempo(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_bus_applies_commands_in_order() {
        let bus = CommandBus::bounded(4);
        bus.dispatch(SessionCommand::Play).unwrap();
        bus.dispatch(SessionCommand::ToggleLoop).unwrap();
        bus.dispatch(SessionCommand::SetMasterVolume(0.5)).unwrap();
        let mut session = Session::default();
        assert_eq!(bus.drain_into(&mut session), 3);
        let snapshot = session.snapshot();
        assert_eq!(snapshot.state, TransportState::Playing);
        assert!(snapshot.loop_enabled);
        assert_eq!(snapshot.master_volume, 0.5);
    }

    #[test]
    fn full_bus_never_blocks_the_producer() {
        let bus = CommandBus::bounded(1);
        bus.dispatch(SessionCommand::Play).unwrap();
        assert_eq!(
            bus.dispatch(SessionCommand::Stop),
            Err(CommandBusError::Full)
        );
    }

    #[test]
    fn invalid_tempo_does_not_corrupt_snapshot() {
        let mut session = Session::default();
        session.apply(SessionCommand::SetTempo(-1.0));
        assert_eq!(session.snapshot().tempo_bpm, 120.0);
    }
}
