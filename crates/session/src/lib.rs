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

/// Resolución musical interna del transporte: 960 ticks por negra.
///
/// Es suficientemente precisa para edición MIDI y permite representar
/// subdivisiones habituales sin usar floats en la posición del proyecto.
pub const TICKS_PER_QUARTER: u32 = 960;

/// Reloj determinista que convierte frames de audio en ticks musicales.
///
/// El resto fraccional se conserva como enteros racionales, de modo que una
/// secuencia de callbacks de PipeWire produce exactamente la misma posición
/// que un único bloque equivalente. No consulta el reloj del sistema y por
/// eso también es portable al render offline y al adaptador WASM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportClock {
    position_ticks: u64,
    remainder: u128,
}

impl Default for TransportClock {
    fn default() -> Self {
        Self {
            position_ticks: 0,
            remainder: 0,
        }
    }
}

impl TransportClock {
    pub fn position_ticks(&self) -> u64 {
        self.position_ticks
    }

    pub fn reset(&mut self) {
        self.position_ticks = 0;
        self.remainder = 0;
    }

    /// Avanza el reloj y devuelve los ticks enteros producidos.
    pub fn advance_frames(&mut self, frames: u64, sample_rate: u32, tempo_bpm: f64) -> u64 {
        if frames == 0 || sample_rate == 0 || !tempo_bpm.is_finite() || tempo_bpm <= 0.0 {
            return 0;
        }

        // Milésimas de BPM hacen que el cálculo sea entero y estable, sin
        // exigir que el modelo de proyecto abandone su API f64 todavía.
        let milli_bpm = (tempo_bpm.clamp(20.0, 999.0) * 1_000.0).round() as u128;
        let numerator = u128::from(frames)
            .saturating_mul(milli_bpm)
            .saturating_mul(u128::from(TICKS_PER_QUARTER));
        let denominator = 60_u128
            .saturating_mul(1_000)
            .saturating_mul(u128::from(sample_rate));
        let total = self.remainder.saturating_add(numerator);
        let ticks = total / denominator;
        self.remainder = total % denominator;
        self.position_ticks = self.position_ticks.saturating_add(ticks as u64);
        ticks as u64
    }
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
    clock: TransportClock,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            snapshot: TransportSnapshot::default(),
            clock: TransportClock::default(),
        }
    }
}

impl Session {
    pub fn snapshot(&self) -> TransportSnapshot {
        self.snapshot
    }

    pub fn clock(&self) -> TransportClock {
        self.clock
    }

    /// Avanza la posición musical sólo mientras el transporte está en marcha.
    pub fn advance_audio_frames(&mut self, frames: u64, sample_rate: u32) -> u64 {
        if self.snapshot.state != TransportState::Playing {
            return 0;
        }
        let advanced = self
            .clock
            .advance_frames(frames, sample_rate, self.snapshot.tempo_bpm);
        self.snapshot.position_ticks = self.clock.position_ticks();
        advanced
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
                self.clock.reset();
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

    #[test]
    fn clock_is_deterministic_across_callback_boundaries() {
        let mut one_block = TransportClock::default();
        let mut many_blocks = TransportClock::default();

        one_block.advance_frames(48_000, 48_000, 120.0);
        for _ in 0..1_500 {
            many_blocks.advance_frames(32, 48_000, 120.0);
        }

        assert_eq!(one_block, many_blocks);
        assert_eq!(one_block.position_ticks(), 1_920);
    }

    #[test]
    fn paused_session_does_not_advance_transport() {
        let mut session = Session::default();
        assert_eq!(session.advance_audio_frames(48_000, 48_000), 0);
        session.apply(SessionCommand::Play);
        assert_eq!(session.advance_audio_frames(24_000, 48_000), 960);
        session.apply(SessionCommand::Pause);
        assert_eq!(session.advance_audio_frames(24_000, 48_000), 0);
        assert_eq!(session.snapshot().position_ticks, 960);
    }
}
