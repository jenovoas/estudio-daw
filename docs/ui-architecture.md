# Frontera entre el núcleo y la interfaz

Durante la fase inicial usamos la CLI para ejercitar el dominio con entradas
reproducibles y poder depurar sin una interfaz gráfica incompleta. La interfaz
de línea de comandos no es
el destino del producto: funciona como adaptador temporal sobre las mismas
operaciones que usará la interfaz.

```text
interfaz nativa / WASM / CLI / guiones
              ↓
  `estudio-daw-application`
              ↓
  CommandBus → modelos Project + Session
              ↓
      adaptadores de plataforma (PipeWire, MIDI, ffmpeg)
```

Reglas de diseño:

- El núcleo no importa controles gráficos, eventos de ventana ni formatos de interfaz.
- La interfaz envía comandos tipados y recibe instantáneas y eventos observables.
- Las operaciones mutables deben poder agruparse para deshacerlas y rehacerlas.
- La interfaz de línea de comandos puede mantenerse como herramienta de diagnóstico y automatización.
- El futuro adaptador WASM usará el mismo dominio, sustituyendo sólo los
  servicios del sistema de archivos, audio y procesos externos.

Las funciones actuales como `attach_media_source`, `add_audio_clip`,
`trim_audio_clip` y `ensure_track_audio_proxy` son los primeros comandos del
dominio. `ProjectHistory::transact()` ya las puede envolver en un
`ChangeSet` transaccional y ofrece `undo()`/`redo()` para que la interfaz sólo
tenga que actualizar su instantánea. `ProjectSnapshot` incluye una revisión
monotónica y `drain_events()` entrega eventos de confirmación, deshacer y
rehacer para actualizar sólo los paneles afectados.

El módulo `estudio-daw-application` concreta la frontera superior: conserva
`CommandRuntime` y su historial mientras el proyecto permanece abierto, valida y
despacha envelopes, y expone eventos y snapshots sin hacer visible
`ProjectHistory`. También centraliza `open`, `save`, `save_as` y el reemplazo
atómico del JSON. La interfaz debe conservar una instancia de `ProjectApplication`
durante toda la sesión; volver a abrir el archivo inicia un historial nuevo desde
el último estado guardado.

## Shell Tauri y transporte de datos

La dirección preferida para el shell de escritorio es Tauri con un frontend web
desacoplado de sus APIs. Tauri será un adaptador de presentación y sistema; la
decisión no mueve el dominio ni el motor de audio al WebView, ni impide crear un
adaptador web independiente en el futuro.

La comunicación tiene dos planos con límites distintos:

- **Control y estado:** comandos tipados, instantáneas de proyecto, diagnósticos,
  progreso de tareas y telemetría compacta pueden cruzar la comunicación entre
  procesos serializada de
  Tauri. Los medidores envían agregados (pico/RMS por bloque o ventana), nunca
  muestras individuales.
- **Audio:** PCM, flujos de captura/reproducción y búferes de DSP permanecen en
  el entorno de ejecución Rust. La llamada de retorno de tiempo real sigue
  usando búferes preasignados y colas acotadas; no llama a WebView ni espera
  mensajes entre procesos.

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

El contrato v1 de snapshots, medidores, waveform y tiles de espectrograma está
especificado en [`ui-bridge-contract-v1.md`](ui-bridge-contract-v1.md). Los IDs
de derivados son opacos; la UI no resuelve rutas del filesystem y el payload de
tiles binarios se solicita por separado del JSON.

El estado de mezcla pertenece a cada pista y se modifica mediante comandos de
proyecto reversibles. La pertenencia a un grupo sólo organiza pistas: no vincula
ni propaga activa, silencio, solo, ganancia o panorama. Cuando el proyecto tiene
un canal Master persistido, activa/silencio/ganancia del Master procesan la suma
final antes de su medidor; los proyectos legados sin ese canal muestran una
salida Master informativa sin controles editables. El selector del Mezclador
asigna como destino una pista de audio, un bus o Master mediante un comando
reversible. El runtime compila y valida rutas internas acíclicas (incluidos los
buses encadenados) antes de publicar el plan y usa buffers por pista reservados
antes del callback. Esto no selecciona puertos físicos de audio ni implementa
entradas, envíos, retornos o procesadores.

En modo navegador el dominio portable podrá compartir modelos y comandos, pero el
backend de audio/compute será otro adaptador (por ejemplo Web Audio/AudioWorklet y
WebGPU cuando estén disponibles). No se presupone intercambio zero-copy con el
runtime nativo de Tauri.
