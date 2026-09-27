# Instrumentos MIDI nativos

## Shell Tauri: capacidades actuales y estado de la UI

La shell Tauri es experimental y parcial; no debe describirse como una
workstation terminada ni como una implementación de la Session View de Ableton.
La referencia de diseño y el inventario funcional comparado de Ableton Live 12
y Ardour están en
[`workstation-arrangement-surface-v2`](../openspec/changes/workstation-arrangement-surface-v2/).
La pantalla actual contiene controles de proyecto/transporte y una vista de
lectura de la sesión, pero su jerarquía visual, flujo creativo y sistema de
pistas de audio no cumplen todavía ese contrato.

Nuevo proyecto crea una pista MIDI SineSynth vacía a 120 BPM; Play no produce
sonido hasta añadir material. Demo MIDI carga siete notas de prueba SineSynth
sin depender de un SoundFont externo. Abrir carga un JSON de proyecto; Guardar
se habilita cuando existe una ruta y Guardar como permite elegirla. Transporte,
historial y perfiles de audio usan las operaciones existentes del bridge Tauri.

La shell puede presentar pistas/clips resumidos y una vista previa compacta de
notas del snapshot, y reproducir material MIDI de instrumento mediante el
runtime existente. No ofrece edición de clips/notas, Session clip launching,
reproducción de regiones de audio, grabación live ni monitorización de entrada.
El modelo ya incluye algunos metadatos y comandos de clips de audio, lo que no
equivale a un flujo funcional de pista de audio. Las tareas de OpenSpec siguen
abiertas hasta que los flujos end-to-end y la inspección visual sean verificados.

`estudio-daw-synth` ofrece dos fuentes: el sinte sinusoidal polifónico (fallback
sin dependencias) y un adaptador opcional a FluidSynth para reproducir
SoundFonts locales. FluidSynth se carga dinámicamente; compilar Estudio DAW no
requiere tener instalada su biblioteca.

## Ruta de señal

```text
KeyLab (ALSA MIDI)
  → hilo de entrada fuera de RT
  → cola acotada de eventos MIDI
  → worker dedicado (FluidSynth, al elegir SoundFont)
  → ring SPSC PCM preasignado
  → nodo fuente del RenderPlan
  → EQ de inserción → PipeWire (AudioBox si está disponible)
```

El callback PipeWire no llama a FluidSynth, no espera locks y no reserva
memoria: consume muestras del ring. Si está vacío, escribe silencio y aumenta
el contador de underruns. La cola de eventos también es acotada y cuenta los
descartes. El SoundFont se carga y el preset se valida antes de iniciar el
stream; si falla, el comando muestra el diagnóstico y no modifica ningún
proyecto. El sinte sinusoidal se conserva como fallback explícito.

Al terminar `midi-synth-live` o `midi-synth-play`, el CLI informa la ocupación
actual y el máximo observado del ring PCM, su capacidad, la duración equivalente
del pico (`frames / sample_rate`) y el periodo PipeWire solicitado. La ocupación
se actualiza al producir cada bloque y antes de que el callback lo consuma; por
eso el máximo describe cuánto audio
ya renderizado puede quedar por delante de un evento MIDI. No equivale a la
latencia completa desde una tecla hasta la salida acústica: esa medición requiere
sincronizar la entrada MIDI con un loopback físico de AudioBox y considerar el
hardware y PipeWire. En la prueba local de reproducción de una toma KeyLab, el
ring alcanzó 2048/2048 frames (42.67 ms a 48 kHz), con 288 frames al cierre,
cero underruns y cero errores del worker; el periodo PipeWire solicitado fue
32 frames (0.67 ms). Una apertura live de 3 s, sin tocar deliberadamente teclas,
observó el mismo pico, cero eventos MIDI descartados y cero underruns. Son
mediciones del puente PCM en este equipo, no una afirmación de latencia total.

El worker conserva un ring preasignado de 2048 frames para absorber jitter.
Los perfiles fijan el objetivo de PCM pendiente de forma independiente del
periodo PipeWire: inicialmente 512 frames para Live/Record y 1024 para
reproducción SoundFont (10,67 ms y 21,33 ms a 48 kHz). La capacidad del ring no
equivale a la cola configurada ni a la latencia total.

En la verificación de 2026-09-27, `pw-top` mostró el sink AudioBox y ambos
clientes de Estudio DAW operando a quantum 256/48 kHz durante reproducción.
Cuatro ataques programados en un take de control aparecieron en el monitor
digital 10–14 ms después (mediana 11.5 ms), con WAV sin muestras descartadas,
cero underruns/errors en FluidSynth y nodos de aplicación, y 512 frames de pico
en el ring. Es una medición hasta el monitor digital previo al DAC; no incluye
la conversión y salida analógica de AudioBox ni la propagación acústica.

## SineSynth

El instrumento de prueba tiene 16 voces, frecuencias precalculadas, ataque y
release lineales; Note On con velocidad cero equivale a Note Off. No monitoriza
la entrada física de AudioBox.

## SoundFont local

Los comandos aceptan un SoundFont y preset opcionales. Banco y programa siguen
la numeración MIDI desde cero. Para descubrir los presets disponibles en un
banco local:

```bash
cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- \
  soundfont-presets /usr/share/soundfonts/FluidR3_GM.sf2
```

```bash
cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- \
  midi-synth-live 12 mi-toma.json "KeyLab Essential 49 MID" \
  --soundfont /usr/share/soundfonts/FluidR3_GM.sf2 --bank 0 --program 0 \
  --capture loopback.wav

cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- \
  midi-synth-play mi-toma.json \
  --soundfont /usr/share/soundfonts/FluidR3_GM.sf2 --bank 0 --program 0 \
  --capture loopback.wav
```

En Linux se buscan `libfluidsynth.so.3`, `.so.2` y `.so`. Instala la biblioteca
de runtime y un banco SF2 de forma independiente; la ruta del banco depende de
la distribución. Estudio DAW no descarga ni redistribuye SoundFonts. Si se
omite `--soundfont`, se usa SineSynth. Una ruta ilegible, preset inexistente o
runtime ausente se informa antes de iniciar el audio.

El modelo de proyecto guarda una referencia portable y un hash opcional, sin
copiar el banco. El transporte de escritorio prepara un worker por pista MIDI
que contiene clips, mezcla las fuentes en scratch preasignado y publica audio
mediante un RenderPlan en PipeWire. El SoundFont y preset de cada pista se
cargan antes de iniciar el stream; un error de ruta/preset devuelve un
diagnóstico al comando Play.

El botón Play reproduce los clips MIDI del proyecto siguiendo `start_tick`, el
PPQ de cada toma y el tempo del proyecto. Pause silencia el callback y congela
el scheduler; Play reanuda el mismo stream, y Stop cierra PipeWire y los workers.
El periodo usa el perfil actualmente seleccionado al abrir un stream. El
objetivo PCM se aplica por separado a cada worker SoundFont. La ruta actual no
reproduce todavía `AudioClip`, no graba desde el shell Tauri y no monitoriza MIDI
entrante desde ese shell.

## Probar SineSynth

```bash
cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- \
  midi-synth-live 12 mi-toma.json "KeyLab Essential 49 MID"
cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- \
  midi-synth-play mi-toma.json
```

El runtime selecciona AudioBox para salida cuando está disponible y conserva
la autoconexión PipeWire como fallback. La prueba de hardware previa capturó
una toma desde KeyLab y finalizó la reproducción PipeWire sin descartar eventos;
eso no es una evaluación subjetiva del timbre.

`midi-synth-live --capture salida.wav` graba la entrada física de AudioBox al
mismo tiempo que la toma MIDI y el render SoundFont. `midi-synth-play --capture
salida.wav` captura la reproducción de una toma guardada. Ambos informes dan el
tiempo del primer callback de captura respecto del origen monotónico de los
eventos MIDI. La captura física usa un cliente/hilo PipeWire asíncrono separado
del stream de salida RT para que el ADC no cambie el scheduling del render; WAV
y salida se guardan independientemente. El offset de primer callback sólo indica
inicio de captura y no es por sí mismo latencia tecla→sonido.

La opción `--capture-monitor` selecciona la fuente de monitor digital de salida de
AudioBox para medir la señal antes del DAC. Úsala junto con `--capture`; por
ejemplo, al reproducir una toma:

```bash
cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- \
  midi-synth-play mi-toma.json \
  --soundfont /usr/share/soundfonts/FluidR3_GM.sf2 --bank 0 --program 0 \
  --capture monitor.wav --capture-monitor
```

Esta ruta marca el stream como captura del monitor de un sink (`stream.capture.sink`,
`node.async`) y ejecuta esa captura fuera del hilo RT para que no condicione el
callback de reproducción. Mantiene el quantum global de PipeWire en el periodo
seleccionado en el perfil (256 frames iniciales, 5,33 ms a 48 kHz), que puede
cambiar el tamaño de bloque de otras aplicaciones; al terminar, PipeWire
recupera la configuración previa. El monitor se graba sin
devolver sus muestras al stream de reproducción, para evitar realimentación.
AudioBox se direcciona por `node.name`, como requiere `target.object`. No se
conectan ni reasignan las entradas físicas.

Para una medición acústica, el micrófono puede conectarse a Input 1 y colocarse
frente a los parlantes alimentados por AudioBox. La captura registra entonces
DAC, amplificación, altavoz, propagación por el aire, micrófono y ADC; distancia,
ganancia y sala afectan el resultado.

La AudioBox USB 96 expone un source monitor virtual de PipeWire para el sink de
salida (`alsa_output.…analog-stereo.monitor`). Es un puerto digital separado de
las entradas físicas: Input 1 conserva el micrófono e Input 2 la guitarra. El
CLI `--capture` captura la entrada física; `--capture-monitor` elige de manera
explícita el monitor digital del sink y solicita un quantum de 256 frames. El
monitor representa la señal digital del sink y no incluye DAC,
amplificación, parlantes, aire ni ADC. La medición física de tecla a sonido
requiere además sincronizar una entrada física o un loopback dedicado.

## Búfer de dispositivo y protección de dropout

Son controles distintos. El manual de AudioBox USB 96 describe `Safe Mode` de
Universal Control en Windows como un ajuste del búfer de entrada; el tamaño de
bloque del driver determina el búfer de dispositivo y puede subir para dar más
tiempo de procesamiento a costa de latencia. La perilla `Mixer` de AudioBox es
monitoreo analógico directo de sus entradas físicas.

`Dropout Protection` es una función del motor de **Studio One**, no una caché
incluida en el hardware AudioBox. Añade un búfer de reproducción independiente
del búfer de dispositivo; Studio One puede mantener baja la ruta de monitoreo y
usar el búfer adicional en reproducción. Ableton utiliza sus propios ajustes de
dispositivo, buffers y compensación; no usa Dropout Protection de Studio One.

La ruta actual de Estudio DAW tampoco implementa Dropout Protection adaptativo.
El perfil inicial solicita `period_frames=256` a PipeWire y define por separado
el objetivo de frames de la cola PCM. La medición de ataques
10–14 ms descrita arriba corresponde sólo al render y al monitor digital de
Estudio DAW; no mide ni emula la protección de reproducción de Studio One.
Referencias del fabricante: [manual de AudioBox USB 96](https://pae-web.presonusmusic.com/downloads/products/pdf/AudioBoxUSB96_Manual_del_propietario_ES_26062018.pdf), [Dropout Protection y monitoreo de baja latencia en Studio One](https://support.presonus.com/hc/en-us/articles/9223446792461-Studio-One-6-Audio-Dropout-Protection-and-Low-Latency-Monitoring-FAQ).

### Perfiles de Estudio DAW

El shell Tauri ofrece los perfiles **Live / Grabar** y **Reproducción
multipista** en «Buffers y latencia». El periodo solicitado y el objetivo de
cola PCM se editan por separado; los valores se guardan en
`$XDG_CONFIG_HOME/estudio-daw/audio-runtime.json` (o
`~/.config/estudio-daw/audio-runtime.json`) y no dentro del proyecto.

Los comandos `midi-synth-live` y `audio-record` consumen el periodo del perfil
Live/Record; `midi-synth-play` consume el perfil Multitrack Playback al abrir un
stream nuevo. Los valores iniciales son 256 frames de periodo y objetivos PCM de
512 frames en vivo y 1024 frames en reproducción. A 48 kHz equivalen
respectivamente a 5,33 ms de periodo, 10,67 ms de objetivo de cola live y 21,33
ms de objetivo PCM de reproducción; estas duraciones describen componentes, no
latencia física total. `audio-record` captura WAV mediante su propio ring de
captura y no consume el objetivo de cola PCM del worker SoundFont.
El tamaño de bloque de PipeWire efectivo aún no se expone por el shell y se
muestra como no disponible. Cambiar preferencias no reconfigura un stream que
ya está corriendo; el siguiente inicio usa el perfil elegido para su modo.

En la verificación de hardware del 2026-09-27 se usaron valores XDG temporales.
Live/Record solicitó 256 frames y `pw-top` observó quantum 256/48 kHz durante la
entrada KeyLab; se recibió 1 evento MIDI sin drops. La grabación WAV con periodo
256 guardó 188.928 muestras intercaladas sin descartes; `pw-top` mostró la
fuente AudioBox sugiriendo 256 y el driver de salida a 1024 durante esa muestra.
Playback solicitó 512 frames y sink/cliente operaron a quantum 512/48 kHz. Con
FluidR3_GM (SF2 de prueba, no Analog Lab), el objetivo de cola PCM de 1024 frames
resultó en 1024 frames actuales/pico de un ring de 2048, sin underruns, errores
del worker ni drops MIDI. La reproducción se escuchó en el equipo conectado a
AudioBox.

`pw-top` reportó ERR totales 235 para el sink y 16.140 para la fuente; la
medición no aisló deltas por corrida, así que esos totales no se atribuyen a un
perfil concreto. El quantum volvió al valor idle de 1024 al terminar los
streams, y `clock.force-quantum` permaneció en 0. Los perfiles se leen al abrir
cada stream.

El shell Tauri ahora conecta Play/Pause/Stop con un stream PipeWire. Play compila
los clips MIDI del proyecto, respeta posición de clip, PPQ y tempo, asigna un
instrumento por pista MIDI y mezcla las fuentes con scratch preasignado. El
periodo solicitado y el margen PCM del perfil seleccionado se usan al abrir el
stream y los workers SoundFont. Se direcciona al sink AudioBox cuando está
enumerado; de otro modo se usa la ruta automática de PipeWire. Pause silencia
la salida y suspende el scheduler sin bloquear el callback; Play reanuda y Stop
cierra el stream y los workers.

Este corte no reproduce `AudioClip`, no graba ni monitoriza entradas live desde
Tauri y no reporta el quantum efectivo. La captura WAV y MIDI live siguen
disponibles mediante CLI y respetan el perfil Live/Grabar. El margen PCM se
aplica a la cola de cada worker SoundFont; SineSynth no mantiene esa cola.
Sustain CC64 de los takes se entrega a FluidSynth y SineSynth.

## Pruebas y límites

Los tests verifican render FluidSynth, paso de notas por el worker y ausencia de
asignaciones en el nodo PCM y el handoff de RenderPlan bajo un allocator de
conteo. La prueba de ocupación verifica que el nodo registra los frames
disponibles antes de consumirlos. Si
`/usr/share/soundfonts/FluidR3_GM.sf2` no existe, los tests del banco local se
omiten. Todavía no hay control semántico de sustain/CC ni editor de presets. La
API hot-swap está disponible para un host que conserve el endpoint de control,
pero la CLI y UI aún no ofrecen un selector interactivo durante reproducción.
