# Estado de sesión y CommandBus

El crate `estudio-daw-session` es la frontera entre las entradas de control y
los sistemas que ejecutan la sesión. No abre ALSA, no depende de PipeWire y no
procesa audio; por diseño puede ser probado sin hardware.

## Flujo

```text
KeyLab / UI / IA
       ↓
SessionCommand
       ↓
CommandBus bounded
       ↓
Session::apply()
       ↓
TransportSnapshot
       ↓
audio-engine / clips / UI
```

## Contratos

`TransportSnapshot` contiene el estado observable del transporte:

- estado `Stopped`, `Playing` o `Paused`;
- tempo BPM;
- posición en ticks;
- loop;
- grabación;
- escena activa;
- volumen master normalizado.

`SessionCommand` representa intenciones, no mensajes de hardware. Esto permite
que MIDI, teclado, scripting y profesor IA controlen el mismo estado sin
duplicar reglas.

## Bus bounded

`CommandBus::bounded(capacity)` usa una cola acotada y `try_send`. Nunca bloquea
al productor: si una ráfaga llena la cola, devuelve `CommandBusError::Full`.
El consumidor ejecuta `drain_into(&mut session)` fuera del callback RT.

La implementación actual usa `std::sync::mpsc::sync_channel` como contrato
portable. El motor de audio reemplazará internamente esta frontera por un ring
lock-free preasignado cuando entre en el hilo RT, conservando los mismos
comandos y snapshots.

## Ejemplo

```rust
let bus = CommandBus::bounded(256);
bus.dispatch(SessionCommand::Play)?;
bus.dispatch(SessionCommand::SetMasterVolume(0.8))?;

let mut session = Session::default();
bus.drain_into(&mut session);
assert_eq!(session.snapshot().state, TransportState::Playing);
```

## Estado actual

`MidiControlCommand::to_session_command()` traduce el control físico al dominio
sin hacer que `Session` dependa de MIDI. El comando
`midi-control-monitor` ya recorre ese camino completo: lector ALSA → adaptador
MIDI → `CommandBus` → `Session` → snapshot impreso.

El reproductor live mantiene todavía una cola de comandos específica para
transporte; la siguiente migración la sustituirá por el mismo bus global.
