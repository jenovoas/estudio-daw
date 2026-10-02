# Design

## Context

El crate `dsp` ya implementa `Equalizer` (biquad paramétrico, 6 tipos de filtro,
N bandas, proceso interleaved sin alloc en el callback). El crate `audio-engine`
ya tiene `EqualizerNode` que envuelve `Equalizer` e implementa `AudioNode`. Ambos
están listos para usarse; falta conectarlos al modelo de proyecto, al comando Tauri
y al `RenderPlan` del runtime.

Ver proposal.md § Why para motivación.

## Goals / Non-Goals

**Goals:**
- Persistir `Vec<EqBandConfig>` en `Track` del project-model
- Exponer `set_track_eq` como comando Tauri que actualiza el modelo y republica el plan
- Insertar `EqualizerNode` en el `RenderPlan` de cada pista con bandas activas
- Panel EQ en la UI que lea del snapshot y llame a `set_track_eq`

**Non-Goals:**
- Visualización de curva de respuesta en frecuencia
- EQ en master bus
- EQ dinámico o multibanda con sidechain

## Data Model

Añadir al `Track` del project-model:

```rust
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub eq_bands: Vec<EqBandConfig>,
```

`EqBandConfig` debe tener serialización estable y portable. Sus campos se exponen en
`camelCase` para el contrato Tauri/JS (`filterType`, `frequencyHz`, `gainDb`, `q`,
`enabled`); `EqFilterType` usa `snake_case` (`bell`, `low_shelf`, etc.).

## Comando Tauri

```text
set_track_eq(track_id: String, bands: Vec<EqBandConfig>) -> Result<UiSnapshot, String>
```

El comando valida la pista y el límite de 8 bandas, reemplaza el array completo,
actualiza el proyecto mediante el mecanismo normal de cambios, recompila el plan
si el motor está conectado y devuelve el snapshot actualizado. Una entrada inválida
no modifica el proyecto.

## RenderPlan

En `build_project_playback_with_end`, insertar `EqualizerNode` después de la fuente
de la pista (clips de audio o instrumento MIDI) y antes del gain/pan del mezclador.
El nodo se crea fuera del callback con el sample rate y número de canales reales y
se añaden todas las bandas persistidas, incluidas las desactivadas para conservar
su configuración. El DSP omite las bandas con `enabled: false`.

El cambio de configuración recompila y publica un `RenderPlan` completo usando el
mecanismo de intercambio ya existente. El callback adopta el nuevo plan en un límite
de bloque sin locks ni asignaciones.

## UI

Añadir `eqBands` al `TrackSummary` serializado. En el Device Rack, añadir una caja
EQ con una fila por banda: tipo, frecuencia, ganancia, Q, bypass y eliminar. Añadir
banda crea una configuración válida inicial; cada edición envía el array completo a
`platform.setTrackEq` y vuelve a renderizar desde el snapshot devuelto. El panel no
mantiene valores ficticios fuera del snapshot.

La UI debe limitar la creación a 8 bandas y mostrar errores del backend sin ocultarlos.
El frequency plot queda fuera de esta fase.

## Orden de implementación

1. Verificar y completar serialización de tipos DSP.
2. Añadir `eq_bands` al modelo y migración por serde default.
3. Exponer el campo y comando en Tauri.
4. Insertar el nodo en el runtime y cubrir errores de configuración.
5. Añadir API platform y panel UI.
6. Ejecutar pruebas unitarias, compilación y smoke test audible.

## Riesgos

- El runtime actual organiza el procesamiento como cadena y no como grafo; esta fase
  añade EQ al punto de inserción existente sin resolver todavía buses ni sends.
- La configuración persistida debe mantener compatibilidad con proyectos antiguos,
  por eso el campo usa default vacío.
- Una configuración inválida de filtro debe fallar antes de publicar un plan nuevo.
