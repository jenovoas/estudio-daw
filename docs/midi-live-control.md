# Control MIDI live

Este documento describe el primer flujo de control MIDI de Estudio DAW. La
meta es que un controlador físico pueda manejar transporte, grabación, loops y
escenas sin depender del teclado del computador.

## Separación de puertos

El KeyLab Essential 49 expone dos puertos ALSA con responsabilidades distintas:

| Puerto | Uso |
|---|---|
| `Arturia KeyLab Essential 49 MID` | Teclas, pads y datos musicales que se graban o envían al instrumento. |
| `Arturia KeyLab Essential 49 DAW` | Botones de transporte y superficie de control. |

No se deben mezclar ambos flujos: una nota de transporte no debe terminar en
una pista MIDI, y una nota musical no debe detener la sesión.

## Perfil observado del puerto DAW

El perfil se implementa en `MidiControlMap::keylab_daw_defaults()` dentro de
`crates/midi-engine/src/lib.rs`:

| Nota MIDI | Acción |
|---:|---|
| 94 | Play/Pause |
| 93 | Stop |
| 95 | Record |

Los valores son cero-basados internamente; por eso el monitor puede mostrar
canal 1 aunque el contrato Rust use `channel: 0`.

## Reproducción live

Para reproducir una toma y controlar el transporte desde el KeyLab:

```bash
cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- \
  midi-play-live mi-toma.json "FLUID Synth" "KeyLab Essential 49 DAW"
```

Para reproducir el primer clip de un proyecto:

```bash
cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- \
  project-play-live proyecto.json "" "FLUID Synth" "KeyLab Essential 49 DAW"
```

El lector MIDI vive en un hilo auxiliar y envía comandos pequeños al hilo de
reproducción. El reloj de reproducción no queda bloqueado por ALSA.

## Grabación live

La grabación escucha el puerto musical y usa el puerto DAW sólo como control:

```bash
cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- \
  midi-record-live mi-toma-live.json \
  "KeyLab Essential 49 MID" \
  "KeyLab Essential 49 DAW"
```

El primer pulsado de Record inicia `MidiRecorder`; el segundo lo detiene y
guarda la toma. Los eventos del puerto DAW nunca se guardan como notas.

## Diseño interno

El flujo actual es:

```text
ALSA port
   ↓
NormalizedMidiEvent
   ↓
MidiControlMap::resolve()
   ↓
MidiControlCommand
   ↓
PlaybackCommand / LiveSessionState
```

`MidiControlMap` es configurable y serializable. Su tabla se modifica fuera
del callback de audio; `resolve()` sólo consulta bindings ya existentes y no
crea asignaciones dinámicas. La futura UI añadirá MIDI Learn persistente para
que el usuario pueda pulsar un control y asignarlo a cualquier acción.

## Limitaciones conocidas

- El perfil DAW actual cubre Play, Stop y Record del KeyLab observado.
- Escenas y volumen master ya tienen acciones de dominio, pero todavía deben
  conectarse al mezclador y al launcher de clips real.
- La comunicación actual usa canales entre hilos; el motor de audio RT deberá
  migrarla a un ring bounded lock-free antes de entrar en el callback.
- La grabación live usa tempo fijo de 120 BPM; el futuro `TransportSnapshot`
  proporcionará tempo, métrica y posición global.

## Pruebas

Las pruebas unitarias cubren normalización de CC, resolución de pads, perfil DAW
y aplicación de comandos al estado live:

```bash
cargo test -p estudio-daw-midi-engine
```
