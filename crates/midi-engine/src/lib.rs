//! Modelo de eventos MIDI y grabación de takes.
//!
//! Este primer recorder vive fuera del callback de audio. La integración RT
//! definitiva recibirá eventos desde un ring bounded preasignado.

use alsa::seq::{EvCtrl, EvNote, Event, EventType, PortCap, PortType, Seq};
use estudio_daw_runtime_diagnostics::{
    find_alsa_midi_output_port, find_alsa_midi_port, normalize_alsa_event, NormalizedMidiEvent,
};
use serde::{Deserialize, Serialize};
use std::{
    ffi::CString,
    io::BufRead,
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};
use thiserror::Error;

pub const DEFAULT_PPQ: u32 = 480;

/// Entrada MIDI que puede convertirse en un control de la sesión.
///
/// Se mantiene separada de `RecordedMidiMessage`: una nota puede ser música
/// en una pista o un comando de transporte según el contexto de la sesión.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MidiControlInput {
    Note { channel: u8, note: u8 },
    ControlChange { channel: u8, controller: u32 },
    PitchBend { channel: u8 },
}

/// Acciones de alto nivel que la futura `CommandBus` aplicará a la sesión.
///
/// Los valores continuos se entregan aparte en `MidiControlCommand`; así el
/// binding permanece estático y resolverlo no necesita crear objetos dinámicos.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MidiControlAction {
    TogglePlay,
    Stop,
    ToggleRecord,
    ToggleLoop,
    NextScene,
    PreviousScene,
    MasterVolume,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct MidiBinding {
    pub input: MidiControlInput,
    pub action: MidiControlAction,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MidiControlCommand {
    pub action: MidiControlAction,
    /// Valor normalizado [0, 1] para controles continuos; los botones usan 1.
    pub value: f32,
}

impl MidiControlCommand {
    /// Traduce control físico a intención de dominio.
    ///
    /// El adaptador evita que `Session` dependa de MIDI: el dominio sólo
    /// recibe `SessionCommand` y puede ser controlado por otras superficies.
    pub fn to_session_command(self) -> estudio_daw_session::SessionCommand {
        use estudio_daw_session::SessionCommand;

        match self.action {
            MidiControlAction::TogglePlay => SessionCommand::TogglePlay,
            MidiControlAction::Stop => SessionCommand::Stop,
            MidiControlAction::ToggleRecord => SessionCommand::ToggleRecord,
            MidiControlAction::ToggleLoop => SessionCommand::ToggleLoop,
            MidiControlAction::NextScene => SessionCommand::NextScene,
            MidiControlAction::PreviousScene => SessionCommand::PreviousScene,
            MidiControlAction::MasterVolume => SessionCommand::SetMasterVolume(self.value),
        }
    }
}

/// Tabla de control MIDI editable desde la UI y persistible junto al proyecto.
///
/// La tabla sólo se modifica fuera del hilo RT. `resolve` únicamente recorre
/// bindings ya reservados y devuelve un comando pequeño, sin asignaciones.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MidiControlMap {
    pub bindings: Vec<MidiBinding>,
}

impl MidiControlMap {
    /// Mapa inicial útil para un live set con pads en el canal 10 MIDI.
    ///
    /// Es deliberadamente pequeño y explícito; la UI añadirá MIDI Learn para
    /// que cada usuario pueda cambiarlo sin recompilar ni alterar sus pistas.
    pub fn live_defaults() -> Self {
        Self::new(vec![
            MidiBinding {
                input: MidiControlInput::Note {
                    channel: 9,
                    note: 36,
                },
                action: MidiControlAction::TogglePlay,
            },
            MidiBinding {
                input: MidiControlInput::Note {
                    channel: 9,
                    note: 37,
                },
                action: MidiControlAction::Stop,
            },
            MidiBinding {
                input: MidiControlInput::Note {
                    channel: 9,
                    note: 38,
                },
                action: MidiControlAction::ToggleRecord,
            },
            MidiBinding {
                input: MidiControlInput::Note {
                    channel: 9,
                    note: 39,
                },
                action: MidiControlAction::ToggleLoop,
            },
            MidiBinding {
                input: MidiControlInput::Note {
                    channel: 9,
                    note: 40,
                },
                action: MidiControlAction::PreviousScene,
            },
            MidiBinding {
                input: MidiControlInput::Note {
                    channel: 9,
                    note: 41,
                },
                action: MidiControlAction::NextScene,
            },
            MidiBinding {
                input: MidiControlInput::ControlChange {
                    channel: 0,
                    controller: 7,
                },
                action: MidiControlAction::MasterVolume,
            },
        ])
    }

    /// Perfil observado en el puerto `KeyLab Essential 49 DAW`.
    ///
    /// El KeyLab expone los botones de transporte como notas en el canal 1,
    /// mientras que los pads del puerto MIDI principal usan el canal 10.
    pub fn keylab_daw_defaults() -> Self {
        Self::new(vec![
            MidiBinding {
                input: MidiControlInput::Note {
                    channel: 0,
                    note: 94,
                },
                action: MidiControlAction::TogglePlay,
            },
            MidiBinding {
                input: MidiControlInput::Note {
                    channel: 0,
                    note: 93,
                },
                action: MidiControlAction::Stop,
            },
            MidiBinding {
                input: MidiControlInput::Note {
                    channel: 0,
                    note: 95,
                },
                action: MidiControlAction::ToggleRecord,
            },
        ])
    }

    /// Selecciona un perfil inicial según el puerto solicitado.
    pub fn for_port_query(query: &str) -> Self {
        if query.to_ascii_lowercase().contains("daw") {
            Self::keylab_daw_defaults()
        } else {
            Self::live_defaults()
        }
    }

    pub fn new(bindings: Vec<MidiBinding>) -> Self {
        Self { bindings }
    }

    pub fn bind(&mut self, input: MidiControlInput, action: MidiControlAction) {
        if let Some(binding) = self
            .bindings
            .iter_mut()
            .find(|binding| binding.input == input)
        {
            binding.action = action;
        } else {
            self.bindings.push(MidiBinding { input, action });
        }
    }

    /// Convierte un evento normalizado en un comando de sesión.
    ///
    /// Note-off y Note-on con velocidad cero no activan botones. Esto evita
    /// dobles disparos al usar pads o teclados que emiten ambos mensajes.
    pub fn resolve(&self, event: &NormalizedMidiEvent) -> Option<MidiControlCommand> {
        let (input, value) = match event {
            NormalizedMidiEvent::NoteOn {
                channel,
                note,
                velocity,
                ..
            } if *velocity > 0 => (
                MidiControlInput::Note {
                    channel: *channel,
                    note: *note,
                },
                1.0,
            ),
            NormalizedMidiEvent::ControlChange {
                channel,
                controller,
                value,
                ..
            } => (
                MidiControlInput::ControlChange {
                    channel: *channel,
                    controller: *controller,
                },
                (*value as f32 / 127.0).clamp(0.0, 1.0),
            ),
            NormalizedMidiEvent::PitchBend { channel, value, .. } => (
                MidiControlInput::PitchBend { channel: *channel },
                ((*value as f32 + 8_192.0) / 16_383.0).clamp(0.0, 1.0),
            ),
            _ => return None,
        };

        self.bindings
            .iter()
            .find(|binding| binding.input == input)
            .map(|binding| MidiControlCommand {
                action: binding.action,
                value,
            })
    }
}

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

#[derive(Debug, Error)]
pub enum LiveControlError {
    #[error("no se pudo encontrar el puerto MIDI: {0}")]
    Discovery(#[from] estudio_daw_runtime_diagnostics::DiagnosticsError),
    #[error("error ALSA durante el control MIDI: {0}")]
    Alsa(#[from] alsa::Error),
}

#[derive(Debug, Error)]
pub enum PlaybackError {
    #[error("no se encontró un destino MIDI de salida para '{0}'")]
    DestinationNotFound(String),
    #[error("no se pudo descubrir la salida MIDI: {0}")]
    Discovery(#[from] estudio_daw_runtime_diagnostics::DiagnosticsError),
    #[error("error ALSA durante la reproducción: {0}")]
    Alsa(#[from] alsa::Error),
}

/// Estado mínimo del transporte. Se mantiene independiente de ALSA para que
/// la futura interfaz gráfica pueda reutilizarlo sin abrir dispositivos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportState {
    Stopped,
    Playing,
    Paused,
}

impl TransportState {
    pub fn play(self) -> Self {
        Self::Playing
    }

    pub fn pause(self) -> Self {
        match self {
            Self::Playing => Self::Paused,
            other => other,
        }
    }

    pub fn resume(self) -> Self {
        match self {
            Self::Paused => Self::Playing,
            other => other,
        }
    }

    pub fn stop(self) -> Self {
        Self::Stopped
    }
}

/// Estado mínimo que puede mutar un controlador MIDI durante un live.
///
/// Es intencionalmente independiente del backend ALSA y de la interfaz
/// gráfica. Más adelante será absorbido por `Session` y su `CommandBus`, pero
/// ya permite probar la semántica del control en tiempo real.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveSessionState {
    pub transport: TransportState,
    pub recording: bool,
    pub loop_enabled: bool,
    pub scene_index: i32,
    pub master_volume: f32,
}

impl Default for LiveSessionState {
    fn default() -> Self {
        Self {
            transport: TransportState::Stopped,
            recording: false,
            loop_enabled: false,
            scene_index: 0,
            master_volume: 1.0,
        }
    }
}

impl LiveSessionState {
    /// Aplica un comando MIDI y devuelve el estado actualizado.
    ///
    /// Esta mutación ocurre fuera del callback de audio. El motor RT recibirá
    /// posteriormente una versión compacta mediante un ring lock-free.
    pub fn apply_control(&mut self, command: MidiControlCommand) {
        match command.action {
            MidiControlAction::TogglePlay => {
                self.transport = match self.transport {
                    TransportState::Playing => TransportState::Paused,
                    TransportState::Paused | TransportState::Stopped => TransportState::Playing,
                };
            }
            MidiControlAction::Stop => {
                self.transport = TransportState::Stopped;
                self.recording = false;
            }
            MidiControlAction::ToggleRecord => {
                self.recording = !self.recording;
                if self.recording && self.transport == TransportState::Stopped {
                    self.transport = TransportState::Playing;
                }
            }
            MidiControlAction::ToggleLoop => self.loop_enabled = !self.loop_enabled,
            MidiControlAction::NextScene => self.scene_index = self.scene_index.saturating_add(1),
            MidiControlAction::PreviousScene => {
                self.scene_index = self.scene_index.saturating_sub(1).max(0)
            }
            MidiControlAction::MasterVolume => {
                self.master_volume = command.value.clamp(0.0, 1.0);
            }
        }
    }
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
        if input.event_input_pending(true)? > 0 {
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

/// Graba MIDI hasta que el botón Record del puerto DAW vuelve a pulsarse.
///
/// El teclado musical y sus controles DAW pueden ser puertos ALSA distintos.
/// Ambos se suscriben al mismo puerto local, pero sólo los eventos del puerto
/// musical entran en la toma. El botón Record únicamente cambia el estado del
/// recorder y nunca se guarda como nota musical.
pub fn record_alsa_midi_live(
    input_query: &str,
    control_query: &str,
    tempo_bpm: u32,
) -> Result<MidiTake, LiveRecordError> {
    let seq = Seq::open(None, None, true)?;
    let Some((input_source, input_label)) = find_alsa_midi_port(input_query)? else {
        return Err(LiveRecordError::Discovery(
            estudio_daw_runtime_diagnostics::DiagnosticsError::PipeWire(format!(
                "no se encontró entrada MIDI para '{input_query}'"
            )),
        ));
    };
    let Some((control_source, control_label)) = find_alsa_midi_port(control_query)? else {
        return Err(LiveRecordError::Discovery(
            estudio_daw_runtime_diagnostics::DiagnosticsError::PipeWire(format!(
                "no se encontró control MIDI para '{control_query}'"
            )),
        ));
    };
    let client_name = CString::new("Estudio DAW Live MIDI Recorder").expect("literal sin NUL");
    seq.set_client_name(&client_name)?;
    let port_name = CString::new("live-recording-input").expect("literal sin NUL");
    let local_port = seq.create_simple_port(
        &port_name,
        PortCap::WRITE | PortCap::SUBS_WRITE,
        PortType::MIDI_GENERIC | PortType::APPLICATION,
    )?;

    for source in [input_source, control_source] {
        let subscription = alsa::seq::PortSubscribe::empty()?;
        subscription.set_sender(source);
        subscription.set_dest(alsa::seq::Addr {
            client: seq.client_id()?,
            port: local_port,
        });
        seq.subscribe_port(&subscription)?;
    }

    println!(
        "Listo para grabar: MIDI={input_label}; control={control_label}. Pulsa Record para iniciar y detener."
    );
    let control_map = MidiControlMap::keylab_daw_defaults();
    let mut recorder = MidiRecorder::new(tempo_bpm, DEFAULT_PPQ);
    let mut recording = false;
    let mut input = seq.input();
    loop {
        if input.event_input_pending(true)? == 0 {
            thread::sleep(Duration::from_millis(1));
            continue;
        }
        let event = input.event_input()?;
        let normalized = normalize_alsa_event(&event);
        if event.get_source() == control_source {
            if let Some(command) = control_map.resolve(&normalized) {
                if command.action == MidiControlAction::ToggleRecord {
                    if recording {
                        let take = recorder.stop()?;
                        println!("Toma live finalizada: {} eventos.", take.events.len());
                        return Ok(take);
                    }
                    recorder.start()?;
                    recording = true;
                    println!("Grabación live iniciada.");
                }
            }
        } else if recording {
            recorder.record(&normalized)?;
        }
    }
}

/// Ejecuta el mapa de control MIDI sobre una entrada ALSA.
///
/// El callback recibe comandos de sesión, no eventos ALSA. Esto permite que
/// una UI, el transporte o el futuro `CommandBus` reutilicen el mismo lector.
/// La función vive fuera del callback de audio: el siguiente paso será
/// publicar sus comandos en un ring bounded hacia el motor RT.
pub fn run_alsa_midi_control<F>(
    query: &str,
    control_map: &MidiControlMap,
    mut on_command: F,
) -> Result<(), LiveControlError>
where
    F: FnMut(MidiControlCommand),
{
    let seq = Seq::open(None, None, true)?;
    let Some((source, label)) = find_alsa_midi_port(query)? else {
        return Err(LiveControlError::Discovery(
            estudio_daw_runtime_diagnostics::DiagnosticsError::PipeWire(format!(
                "no se encontró puerto MIDI para '{query}'"
            )),
        ));
    };
    let client_name = CString::new("Estudio DAW MIDI Control").expect("literal sin NUL");
    seq.set_client_name(&client_name)?;
    let port_name = CString::new("control-input").expect("literal sin NUL");
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

    println!("Control MIDI activo desde {label}. Presiona Ctrl+C para salir.");
    let mut input = seq.input();
    loop {
        if input.event_input_pending(true)? > 0 {
            let event = input.event_input()?;
            let normalized = normalize_alsa_event(&event);
            if let Some(command) = control_map.resolve(&normalized) {
                on_command(command);
            }
        } else {
            thread::sleep(Duration::from_millis(1));
        }
    }
}

/// Reproduce una toma MIDI hacia un destino ALSA MIDI, respetando PPQ y tempo.
///
/// La planificación ocurre fuera del callback de audio. Este primer adaptador
/// usa eventos directos ALSA; el motor RT reemplazará el sleep por su reloj.
pub fn play_midi_take(take: &MidiTake, query: &str) -> Result<usize, PlaybackError> {
    play_midi_take_internal(take, query, None)
}

/// Reproduce una toma con controles de transporte desde la terminal.
///
/// Controles — todos requieren Enter:
/// `p` pausa/reanuda, `s` detiene, `l` activa/desactiva loop y `q` sale.
/// La entrada de teclado vive en otro hilo; el hilo que envía MIDI sólo
/// consulta un canal de comandos y no bloquea el envío ALSA.
pub fn play_midi_take_interactive(take: &MidiTake, query: &str) -> Result<usize, PlaybackError> {
    let (commands_tx, commands_rx) = mpsc::channel();
    thread::spawn(move || {
        for line in std::io::stdin().lock().lines().flatten() {
            let command = match line.trim().to_ascii_lowercase().as_str() {
                "p" => PlaybackCommand::TogglePause,
                "s" | "q" => PlaybackCommand::Stop,
                "l" => PlaybackCommand::ToggleLoop,
                _ => continue,
            };
            if commands_tx.send(command).is_err() || command == PlaybackCommand::Stop {
                break;
            }
        }
    });
    play_midi_take_internal(take, query, Some(commands_rx))
}

/// Reproduce una toma controlada directamente desde el controlador MIDI.
///
/// El lector MIDI corre en un hilo auxiliar y sólo publica comandos pequeños
/// al hilo de reproducción. De esta forma el puerto ALSA no bloquea el reloj
/// musical ni introduce lógica de dispositivo dentro del futuro callback RT.
pub fn play_midi_take_live(
    take: &MidiTake,
    output_query: &str,
    control_query: &str,
) -> Result<usize, PlaybackError> {
    let (commands_tx, commands_rx) = mpsc::channel();
    let control_map = MidiControlMap::for_port_query(&control_query);
    let control_query = control_query.to_string();
    thread::spawn(move || {
        let result = run_alsa_midi_control(&control_query, &control_map, |command| {
            let playback_command = match command.action {
                MidiControlAction::TogglePlay => Some(PlaybackCommand::TogglePause),
                MidiControlAction::Stop => Some(PlaybackCommand::Stop),
                MidiControlAction::ToggleLoop => Some(PlaybackCommand::ToggleLoop),
                // La grabación y el volumen se conectarán al CommandBus de
                // sesión cuando el reproductor deje de ser sólo de takes.
                MidiControlAction::ToggleRecord
                | MidiControlAction::NextScene
                | MidiControlAction::PreviousScene
                | MidiControlAction::MasterVolume => None,
            };
            if let Some(playback_command) = playback_command {
                let _ = commands_tx.send(playback_command);
            }
        });
        if let Err(error) = result {
            eprintln!("Control MIDI finalizado: {error}");
        }
    });
    play_midi_take_internal(take, output_query, Some(commands_rx))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlaybackCommand {
    TogglePause,
    ToggleLoop,
    Stop,
}

fn play_midi_take_internal(
    take: &MidiTake,
    query: &str,
    commands: Option<Receiver<PlaybackCommand>>,
) -> Result<usize, PlaybackError> {
    let seq = Seq::open(None, None, false)?;
    let Some((destination, label)) = find_alsa_midi_output_port(query)? else {
        return Err(PlaybackError::DestinationNotFound(query.into()));
    };
    let client_name = CString::new("Estudio DAW MIDI Player").expect("literal sin NUL");
    seq.set_client_name(&client_name)?;
    let port_name = CString::new("playback-output").expect("literal sin NUL");
    let local_port = seq.create_simple_port(
        &port_name,
        PortCap::READ | PortCap::SUBS_READ,
        PortType::MIDI_GENERIC | PortType::APPLICATION,
    )?;
    println!(
        "Reproduciendo {} eventos hacia {label}...",
        take.events.len()
    );
    if commands.is_some() {
        println!("Controles: [p] pausa/reanuda  [s] detiene  [l] loop  [q] sale");
    }
    let mut state = TransportState::Playing;
    let mut loop_enabled = false;
    let mut started_at = Instant::now();
    let mut accumulated = Duration::ZERO;
    let mut event_index = 0;
    let mut sent = 0;
    while event_index < take.events.len() {
        if let Some(commands) = &commands {
            for command in commands.try_iter() {
                match command {
                    PlaybackCommand::TogglePause if state == TransportState::Playing => {
                        accumulated += started_at.elapsed();
                        state = state.pause();
                        println!("Transporte en pausa.");
                    }
                    PlaybackCommand::TogglePause if state == TransportState::Paused => {
                        started_at = Instant::now();
                        state = state.resume();
                        println!("Transporte reanudado.");
                    }
                    PlaybackCommand::ToggleLoop => {
                        loop_enabled = !loop_enabled;
                        println!(
                            "Loop {}.",
                            if loop_enabled {
                                "activado"
                            } else {
                                "desactivado"
                            }
                        );
                    }
                    PlaybackCommand::Stop => {
                        send_all_notes_off(&seq, local_port, destination)?;
                        println!("Transporte detenido.");
                        return Ok(sent);
                    }
                    _ => {}
                }
            }
        }
        if state == TransportState::Paused {
            thread::sleep(Duration::from_millis(10));
            continue;
        }
        let event = &take.events[event_index];
        let micros = event.tick.saturating_mul(60_000_000)
            / u64::from(take.tempo_bpm.max(1))
            / u64::from(take.ppq.max(1));
        let elapsed = accumulated + started_at.elapsed();
        let target = Duration::from_micros(micros);
        if target > elapsed {
            thread::sleep((target - elapsed).min(Duration::from_millis(10)));
            continue;
        }
        if let Some(mut alsa_event) = to_alsa_output_event(&event.message) {
            alsa_event.set_source(local_port);
            alsa_event.set_dest(destination);
            alsa_event.set_direct();
            seq.event_output_direct(&mut alsa_event)?;
            sent += 1;
        }
        event_index += 1;
        if event_index == take.events.len() && loop_enabled {
            event_index = 0;
            started_at = Instant::now();
            accumulated = Duration::ZERO;
            println!("Loop: nueva vuelta.");
        }
    }
    seq.drain_output()?;
    println!("Reproducción finalizada: {sent} eventos enviados.");
    Ok(sent)
}

fn send_all_notes_off(
    seq: &Seq,
    local_port: i32,
    destination: alsa::seq::Addr,
) -> Result<(), PlaybackError> {
    for channel in 0..16 {
        let mut event = Event::new(
            EventType::Controller,
            &EvCtrl {
                channel,
                param: 123,
                value: 0,
            },
        );
        event.set_source(local_port);
        event.set_dest(destination);
        event.set_direct();
        seq.event_output_direct(&mut event)?;
    }
    seq.drain_output()?;
    Ok(())
}

fn to_alsa_output_event(message: &RecordedMidiMessage) -> Option<Event<'static>> {
    match message {
        RecordedMidiMessage::NoteOn {
            channel,
            note,
            velocity,
        } => Some(Event::new(
            EventType::Noteon,
            &EvNote {
                channel: *channel,
                note: *note,
                velocity: *velocity,
                off_velocity: 0,
                duration: 0,
            },
        )),
        RecordedMidiMessage::NoteOff {
            channel,
            note,
            release_velocity,
        } => Some(Event::new(
            EventType::Noteoff,
            &EvNote {
                channel: *channel,
                note: *note,
                velocity: 0,
                off_velocity: *release_velocity,
                duration: 0,
            },
        )),
        RecordedMidiMessage::KeyPressure {
            channel,
            note,
            pressure,
        } => Some(Event::new(
            EventType::Keypress,
            &EvNote {
                channel: *channel,
                note: *note,
                velocity: *pressure,
                off_velocity: 0,
                duration: 0,
            },
        )),
        RecordedMidiMessage::ControlChange {
            channel,
            controller,
            value,
        } => Some(Event::new(
            EventType::Controller,
            &EvCtrl {
                channel: *channel,
                param: *controller,
                value: *value,
            },
        )),
        RecordedMidiMessage::PitchBend { channel, value } => Some(Event::new(
            EventType::Pitchbend,
            &EvCtrl {
                channel: *channel,
                param: 0,
                value: *value,
            },
        )),
        RecordedMidiMessage::ChannelPressure { channel, pressure } => Some(Event::new(
            EventType::Chanpress,
            &EvCtrl {
                channel: *channel,
                param: 0,
                value: *pressure,
            },
        )),
        RecordedMidiMessage::ProgramChange { channel, program } => Some(Event::new(
            EventType::Pgmchange,
            &EvCtrl {
                channel: *channel,
                param: 0,
                value: *program,
            },
        )),
        RecordedMidiMessage::SysEx { .. } | RecordedMidiMessage::Other => None,
    }
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
    fn resolves_transport_pad_without_allocating_a_new_binding() {
        let map = MidiControlMap::live_defaults();
        let command = map.resolve(&NormalizedMidiEvent::NoteOn {
            channel: 9,
            note: 36,
            velocity: 100,
            source: Addr {
                client: 28,
                port: 0,
            },
        });
        assert_eq!(
            command,
            Some(MidiControlCommand {
                action: MidiControlAction::TogglePlay,
                value: 1.0,
            })
        );
    }

    #[test]
    fn normalizes_cc_and_ignores_note_off() {
        let mut map = MidiControlMap::default();
        map.bind(
            MidiControlInput::ControlChange {
                channel: 0,
                controller: 7,
            },
            MidiControlAction::MasterVolume,
        );
        let command = map.resolve(&NormalizedMidiEvent::ControlChange {
            channel: 0,
            controller: 7,
            value: 64,
            source: Addr {
                client: 28,
                port: 0,
            },
        });
        assert!((command.unwrap().value - 64.0 / 127.0).abs() < f32::EPSILON);
        assert!(map
            .resolve(&NormalizedMidiEvent::NoteOff {
                channel: 0,
                note: 60,
                release_velocity: 0,
                source: Addr {
                    client: 28,
                    port: 0
                },
            })
            .is_none());
    }

    #[test]
    fn applies_midi_commands_to_live_session_state() {
        let mut state = LiveSessionState::default();
        state.apply_control(MidiControlCommand {
            action: MidiControlAction::ToggleRecord,
            value: 1.0,
        });
        state.apply_control(MidiControlCommand {
            action: MidiControlAction::ToggleLoop,
            value: 1.0,
        });
        state.apply_control(MidiControlCommand {
            action: MidiControlAction::NextScene,
            value: 1.0,
        });
        state.apply_control(MidiControlCommand {
            action: MidiControlAction::MasterVolume,
            value: 0.75,
        });
        assert_eq!(state.transport, TransportState::Playing);
        assert!(state.recording);
        assert!(state.loop_enabled);
        assert_eq!(state.scene_index, 1);
        assert_eq!(state.master_volume, 0.75);
    }

    #[test]
    fn selects_keylab_daw_transport_profile() {
        let map = MidiControlMap::for_port_query("Arturia KeyLab Essential 49 DAW");
        let command = map.resolve(&NormalizedMidiEvent::NoteOn {
            channel: 0,
            note: 94,
            velocity: 127,
            source: Addr {
                client: 28,
                port: 1,
            },
        });
        assert_eq!(
            command.map(|command| command.action),
            Some(MidiControlAction::TogglePlay)
        );
    }

    #[test]
    fn translates_midi_control_to_domain_command() {
        let command = MidiControlCommand {
            action: MidiControlAction::MasterVolume,
            value: 0.63,
        }
        .to_session_command();
        assert_eq!(
            command,
            estudio_daw_session::SessionCommand::SetMasterVolume(0.63)
        );
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

    #[test]
    fn transport_state_has_predictable_transitions() {
        assert_eq!(TransportState::Stopped.play(), TransportState::Playing);
        assert_eq!(TransportState::Playing.pause(), TransportState::Paused);
        assert_eq!(TransportState::Paused.resume(), TransportState::Playing);
        assert_eq!(TransportState::Playing.stop(), TransportState::Stopped);
        assert_eq!(TransportState::Stopped.pause(), TransportState::Stopped);
    }
}
