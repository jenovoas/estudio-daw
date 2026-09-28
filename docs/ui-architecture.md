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
antes del callback. Esto no selecciona puertos físicos independientes por
pista ni implementa entradas, envíos, retornos o procesadores.

Los ajustes de aplicación permiten escoger un sink PipeWire para el stream
Master del próximo inicio. La opción automática conserva la preferencia por
AudioBox y usa el destino predeterminado cuando no hay una disponible. El
Mezclador permite asignar fuentes PipeWire y canales 1/2 por pista de audio;
Record captura las pistas armadas a WAV y Stop crea regiones de proyecto. La
enumeración y selección ocurren fuera del callback; aún faltan salidas físicas
independientes por pista, medidor de entrada y QA con AudioBox.

La escala de la interfaz es una preferencia local del WebView, no del proyecto.
Tauri controla el zoom entre 80 % y 150 %; Ctrl++/Ctrl+- ajusta en pasos de 10 %
y Ctrl+0 restaura el 100 %. Los campos de texto conservan sus teclas + y -.
Ctrl+1/2/3 cambia Arreglo/Session/Mezclador; Ctrl+S guarda, Ctrl+Mayús+S abre
Guardar como y Ctrl+Z/Ctrl+Mayús+Z deshace/rehace. Estos atajos se omiten en
campos editables.

Las pistas editables tienen una acción de quitar en Arreglo, Session y Mezclador.
El comando elimina sus clips, regiones y referencias de medios del proyecto,
reasigna las pistas que apuntaban a ella y se puede deshacer desde el historial;
no borra los archivos fuente del disco. El canal Master queda protegido en la UI.
Los controles arriba/abajo reordenan pistas mediante `MoveTrack` en las tres
superficies y conservan al Master en su mismo lado de la lista; el comando
refresca el plan si el motor está conectado.

La barra superior ofrece menús de Proyecto, Edición, Crear, Sesión, Vista y Transporte a
partir de un registro único de acciones, que también aporta las etiquetas y los
atajos de navegación, guardado e historial. Los menús contextuales de clips y
pistas filtran acciones existentes: seleccionar, preescuchar/quitar una región y
quitar una pista cuando esa pista lo permite. Proyecto abre el flujo de
importación de audio ya conectado y Sesión añade una escena. Sus acciones reutilizan los
manejadores visibles y sus comandos reversibles. El catálogo todavía no cubre
las familias completas de Proyecto, edición de clips, preferencias ni ayuda.
Transporte también ofrece Play/Pause/Stop/Record, pánico, metrónomo y los
controles existentes del rango A/B; cada opción sigue el estado habilitado del
control que reutiliza.
En el menú contextual de una pista no Master también se puede duplicar: el
comando crea IDs nuevos para pista, clips, fuentes y casillas, y conserva la
procedencia de las fuentes compartidas sin editar sus archivos.
En el contexto de un clip MIDI, «Cuantizar a rejilla actual» usa su PPQ
persistido y el valor del selector de rejilla; el comando sólo cuantiza Note
On/Off y conserva controladores. La acción se oculta cuando la rejilla está en
modo libre.
El mismo menú contextual permite duplicar el clip MIDI a continuación del
original. El comando asigna un ID único y conserva el contenido de la toma;
la nueva posición también puede ajustarse arrastrando con la rejilla activa.

La barra de herramientas lateral y las pestañas del encabezado cambian entre
Arreglo, Session y Mezclador. Session presenta pistas por columnas y escenas
por filas; sus escenas y asignaciones de clip son comandos reversibles, aunque
se pueden reordenar con controles arriba/abajo. El lanzamiento permanece
deshabilitado hasta conectar el planificador. El botón
de ajustes abre Preferencias de audio. Al seleccionar un clip MIDI o una región
de audio en Arreglo, el panel inferior presenta un inspector con pista,
ubicación y duración. Para audio también permite cambiar numéricamente posición,
ganancia y desvanecimientos; las regiones se pueden mover y recortar por sus
bordes con la rejilla seleccionada (1/16, 1/8, negra, compás o libre). El recorte
del inicio avanza también el desplazamiento de fuente y las modificaciones usan
el historial del proyecto. En Arreglo, los clips MIDI se pueden mover con la
rejilla seleccionada y duplicar desde su menú contextual. «Dividir en cursor»
aparece sólo cuando el cursor cae dentro del clip. El corte crea dos clips
contiguos, cierra notas activas y las rearticula al comienzo de la segunda parte;
restaura allí el último CC, pitch bend, presión y programa. Esto puede producir
un nuevo ataque en notas sostenidas, y SysEx previo no se copia. Los cambios
usan el historial y el plan activo se actualiza si el motor está conectado. El
inspector abre un piano roll acotado de 36 teclas y permite insertar corcheas
con clic; cada inserción conserva los eventos previos, se puede deshacer y
refresca el plan activo. Aún faltan selección, movimiento, cambio de duración,
velocidad y borrado de notas existentes; el registro nuevo usa canal 1.

El navegador también enumera las regiones de audio importadas del proyecto y
permite filtrarlas por nombre, fuente, pista y disposición de canales. Cada
elemento se puede preescuchar de forma aislada o seleccionar para enfocar su
región en Arrangement; todavía no representa una biblioteca general de medios.

En modo navegador el dominio portable podrá compartir modelos y comandos, pero el
backend de audio/compute será otro adaptador (por ejemplo Web Audio/AudioWorklet y
WebGPU cuando estén disponibles). No se presupone intercambio zero-copy con el
runtime nativo de Tauri.
