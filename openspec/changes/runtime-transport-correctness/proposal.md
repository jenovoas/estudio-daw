# Proposal

## Why

El transporte de escritorio puede desfasarse del audio renderizado al pausar, y la reproducción no siempre respeta los límites del clip. La superficie actual tampoco comunica un flujo de arreglo utilizable de DAW y no alcanza la dirección visual previamente aprobada, inspirada en Ableton Live 12.

## What Changes

- Congelar el procesamiento DSP/consumo PCM durante la pausa y mantener alineada la planificación MIDI al pausar/reanudar.
- Planificar sólo los eventos MIDI dentro del inicio y la duración de cada clip; calcular el tiempo de la toma de demostración con el PPQ y tempo declarados.
- Hacer que la reproducción en tiempo real falle de manera segura, sin entrar en pánico, y limitar los tonos de `SineSynth` al intervalo MIDI válido.
- Reorganizar la interfaz Tauri como un espacio de arreglo denso y oscuro, con jerarquía clara de transporte, pistas, clips y ajustes de audio, inspirado en Live 12.
- Corregir la documentación del producto para describir la ruta de reproducción conectada en Tauri e indicar los límites experimentales restantes.

## Capabilities

### New Capabilities

- `runtime-midi-playback`: planificación MIDI acotada por clip, semántica de pausa y comportamiento seguro ante fallos de la devolución.
- `desktop-workstation-ui`: superficie de arreglo y transporte de escritorio orientada a DAW.

### Modified Capabilities

Ninguna. El repositorio aún no tiene especificaciones principales sincronizadas; estas capacidades nuevas establecen los contratos de comportamiento sin presentar deltas anteriores como canónicos.

## Impact

El cambio afecta `crates/audio-platform`, `crates/audio-engine`, `crates/synth` y `crates/ui-shell`. Se alinearán README y la documentación nativa de audio/planes de renderizado. No se prevén cambios en el formato serializado del proyecto ni en el ruteo de hardware.
