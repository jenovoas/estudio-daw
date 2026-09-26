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

## Shell Tauri y transporte de datos

La dirección preferida para el shell de escritorio es Tauri con un frontend web
desacoplado de sus APIs. Tauri será un adaptador de presentación y sistema; la
decisión no mueve el dominio ni el motor de audio al WebView, ni impide crear un
adaptador web independiente en el futuro.

La comunicación tiene dos planos con límites distintos:

- **Control y estado:** comandos tipados, snapshots de proyecto, diagnósticos,
  progreso de jobs y telemetría compacta pueden cruzar el IPC serializado de
  Tauri. Los medidores envían agregados (pico/RMS por bloque o ventana), nunca
  muestras individuales.
- **Audio:** PCM, streams de captura/reproducción y buffers de DSP permanecen en
  el runtime Rust. El callback RT sigue usando buffers preasignados y colas
  bounded; no llama al WebView ni espera mensajes IPC.

Los proyectos serializan la estructura musical y referencias a medios; WAV, FLAC,
proxies y renders son artefactos externos al JSON. La UI identifica medios mediante
IDs/ref opacas y solicita operaciones al backend. El WebView no recibe permiso
general sobre el filesystem. Si se usa el protocolo asset para un preview, su
scope se limita a rutas autorizadas del proyecto o caché.

## Frontera de GPU y visualizaciones

La GPU acelera trabajos pesados mediante `compute-runtime`/`wgpu` fuera del
callback: espectrogramas, análisis, separación, convolución larga, time-stretch,
pitch-shift y renders offline, según benchmark y coste de transferencia. La CPU
continúa siendo la ruta segura para el callback, el transporte y el DSP pequeño de
baja latencia; cualquier procesamiento GPU en tiempo real requiere deadline,
buffers preparados y fallback CPU probado.

La GPU del runtime nativo y la del WebView son contextos distintos: no se asumirá
que comparten dispositivos, texturas o buffers sin copia. La interfaz recibe datos
derivados de tamaño acotado (por ejemplo, una pirámide min/max para waveform,
medidores agregados o tiles de espectrograma); la lectura de vuelta GPU se hace
asíncronamente y fuera del callback. El WebView puede usar su propia aceleración
para dibujar, pero eso no sustituye ni duplica el DSP nativo.

En modo navegador el dominio portable podrá compartir modelos y comandos, pero el
backend de audio/compute será otro adaptador (por ejemplo Web Audio/AudioWorklet y
WebGPU cuando estén disponibles). No se presupone intercambio zero-copy con el
runtime nativo de Tauri.
