# Proposal

## Why

Actualmente Estudio DAW elige internamente el período PipeWire y la profundidad de la cola PCM SoundFont, por lo que el usuario no puede ajustar desde el DAW la respuesta frente a la resistencia a interrupciones. Se necesitan perfiles seleccionables independientes para interpretación/grabación y reproducción multipista, con información honesta sobre el período efectivo del dispositivo y el búfer adicional de reproducción.

## What Changes

- Añadir en el DAW ajustes de audio para el período solicitado al dispositivo y un búfer de seguridad de reproducción independiente.
- Ofrecer perfiles iniciales de interpretación/grabación y reproducción multipista; permitir que el usuario los ajuste y consulte sus valores efectivos.
- Conservar una ruta de instrumento en vivo con monitorización de baja latencia y permitir búfer adicional en pistas de reproducción ya renderizadas cuando el motor lo admita.
- Conservar intactos los eventos MIDI, incluido el pedal de sustain CC64, al cambiar de perfil y durante la grabación/reproducción.
- Explicar cuándo PipeWire o el dispositivo de audio negocien un período efectivo distinto del solicitado.

## Capabilities

### New Capabilities

- `audio-runtime-control`: user-facing audio buffer profiles, effective settings, safe runtime changes, and separation of device period from playback safety buffering.

### Modified Capabilities

Ninguna. El repositorio todavía no tiene entradas canónicas en `openspec/specs`; los requisitos relacionados con el entorno de ejecución están actualmente en el cambio de arquitectura fundacional. Este cambio incorpora una capacidad verificable de forma independiente sin alterar ese plan histórico.

## Impact

- Áreas posiblemente afectadas: `audio-platform`, `synth`, `application`, la ventana de interfaz y los diagnósticos de ejecución.
- PipeWire sigue a cargo de negociar el cuanto del grafo; los períodos solicitado y efectivo deben mostrarse por separado.
- El búfer adicional de reproducción lo administra el DAW y no debe llamarse caché de hardware AudioBox ni protección contra interrupciones de PreSonus Studio One.
- Los preajustes son puntos de partida, no promesas de latencia fija de extremo a extremo ni de funcionamiento sin interrupciones.
