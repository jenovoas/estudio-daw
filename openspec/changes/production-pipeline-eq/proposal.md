# Proposal

## Why

Estudio DAW tiene un ecualizador paramétrico completo implementado en el crate `dsp`
(6 tipos de filtro biquad: Bell, LowShelf, HighShelf, LowPass, HighPass, Notch) y un
`EqualizerNode` listo para insertar en el `RenderPlan` del motor de audio. Sin embargo,
no existe ningún comando Tauri que lo exponga, ningún campo en el modelo de proyecto
que persista las bandas, y ningún panel en la UI que permita al productor usarlo.

El EQ es la herramienta más fundamental de la fase de edición y mezcla en cualquier
DAW profesional. Sin él, el usuario no puede moldear el timbre de una pista de audio,
eliminar frecuencias problemáticas, ni preparar un mix balanceado.

## What

Conectar el EQ paramétrico existente al flujo completo: modelo de proyecto → comando
Tauri → RenderPlan del motor de audio → panel de control en la UI.

El usuario podrá:
- Añadir, editar y desactivar bandas de EQ por pista (hasta 8 bandas)
- Elegir el tipo de filtro por banda (Bell, LowShelf, HighShelf, LowPass, HighPass, Notch)
- Ajustar frecuencia (Hz), ganancia (dB) y Q por banda con controles directos
- Ver el estado del EQ reflejado inmediatamente en el audio de reproducción

## Scope

**In scope:**
- Campo `eq_bands: Vec<EqBandConfig>` en `Track` del project-model
- Serialización/deserialización de las bandas en `project.json`
- Comando Tauri `set_track_eq(track_id, bands)` → devuelve `UiSnapshot`
- Inserción del `EqualizerNode` en el `RenderPlan` cuando la pista tiene bandas activas
- Panel EQ en la UI del Device Rack (bandas, tipo, Hz, dB, Q, toggle enabled)
- El EQ aplica a pistas de audio y MIDI (post-instrumento)

**Out of scope:**
- Visualización de curva de frecuencia (frequency response plot) — fase posterior
- EQ dinámico — requiere DSP adicional no implementado
- EQ en el master bus — pendiente del grafo de mezcla (fase 3)
- Automatización de parámetros de EQ — pendiente de la fase de automatización

## Acceptance Criteria

1. `set_track_eq` persiste las bandas en `project.json` y sobrevive a recargar el proyecto
2. Al reproducir con el motor conectado, el EQ procesa el audio en tiempo real
3. Desactivar una banda (enabled: false) elimina su efecto sin borrar la configuración
4. El panel EQ en la UI muestra el estado real leído del snapshot y actualiza al cambiar
5. Compilación limpia sin warnings nuevos; tests existentes pasan sin modificación
