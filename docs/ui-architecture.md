# Frontera entre core y UI

Durante la fase inicial usamos la CLI para ejercitar el dominio con entradas
reproducibles y poder depurar sin una interfaz gráfica incompleta. La CLI no es
el destino del producto: funciona como adaptador temporal sobre las mismas
operaciones que usará la UI.

```text
UI nativa / WASM / CLI / scripting
              ↓
      Application Command API
              ↓
   Project model + Session + Audio engine
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
que refrescar su snapshot.
