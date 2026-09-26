# Frontera entre core y UI

Durante la fase inicial usamos la CLI para ejercitar el dominio con entradas
reproducibles y poder depurar sin una interfaz gráfica incompleta. La CLI no es
el destino del producto: funciona como adaptador temporal sobre las mismas
operaciones que usará la UI.

```text
UI nativa / WASM / CLI / scripting
              ↓
  `estudio-daw-application`
              ↓
  CommandBus → Project + Session models
              ↓
      Platform adapters (PipeWire, MIDI, ffmpeg)
```

Reglas de diseño:

- El core no importa widgets, eventos de ventana ni formatos de UI.
- La UI envía comandos tipados y recibe snapshots/eventos observables.
- Las operaciones mutables deben ser agrupables para undo/redo.
- La CLI puede mantenerse como herramienta de diagnóstico y automatización.
- El futuro adaptador WASM usará el mismo dominio, sustituyendo sólo los
  servicios de filesystem, audio y procesos externos.

Las funciones actuales como `attach_media_source`, `add_audio_clip`,
`trim_audio_clip` y `ensure_track_audio_proxy` son los primeros comandos del
dominio. `ProjectHistory::transact()` ya las puede envolver en un
`ChangeSet` transaccional y ofrece `undo()`/`redo()` para que la UI sólo tenga
que refrescar su snapshot. `ProjectSnapshot` incluye una revisión monotónica y
`drain_events()` entrega eventos de commit, undo y redo para actualizar sólo
los paneles afectados.

La crate `estudio-daw-application` concreta la frontera superior: conserva
`CommandRuntime` y su historial mientras el proyecto permanece abierto, valida y
despacha envelopes, y expone eventos y snapshots sin hacer visible
`ProjectHistory`. También centraliza `open`, `save`, `save_as` y el reemplazo
atómico del JSON. Una UI debe conservar una instancia de `ProjectApplication`
durante toda la sesión; volver a abrir el archivo inicia un historial nuevo desde
el último estado guardado.
