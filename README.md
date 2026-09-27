# Estudio DAW

> Nota de identidad: el nombre `openDAW` ya corresponde a un DAW web existente,
> construido en TypeScript/Web Audio, con SDK propio y licencia AGPL/comercial.
> El nombre adoptado para este proyecto es **Estudio DAW**.

El proyecto existente será tratado como referencia técnica y posible fuente de
integración, no como una base que podamos copiar sin revisar su licencia y sus
límites de arquitectura.

Estudio DAW es un estudio de producción musical programable, educativo y extensible, diseñado primero para Linux.

La visión es construir una aplicación inspirada en el flujo creativo de Ableton Live, combinada con la solidez de un DAW de grabación y mezcla, live coding, análisis musical y un profesor/productor virtual. No busca copiar código, interfaz, marca ni assets propietarios: busca ofrecer una experiencia musical propia basada en ideas funcionales abiertas.

## Visión

Estudio DAW debe permitir producir una canción utilizando:

- guitarra electroacústica;
- voz y micrófono de estudio;
- interfaces de audio;
- controladores MIDI como Arturia KeyLab;
- instrumentos y efectos virtuales;
- clips y escenas;
- código musical;
- lenguaje natural;
- análisis de acordes, melodías, letras e instrumentos;
- partituras, tablaturas y arreglos editables;
- acompañamiento pedagógico personalizado.

La idea central es que tocar, programar, editar y conversar sean distintas formas de controlar el mismo proyecto musical.

```text
guitarra + voz + MIDI
        ↓
grabación y composición
        ↓
clips, escenas y arreglos
        ↓
mezcla, atmósferas y diseño sonoro
        ↓
análisis musical y profesor virtual
        ↓
producción reproducible mediante código
```

## Referencias funcionales

Estudio DAW tomará ideas de varias herramientas, sin depender de su interfaz ni de su código:

- **Ableton Live**: Session View, Arrangement View, clips, escenas, warping, escalas y producción performativa.
- **Ardour**: grabación, routing, mezcla, automatización, buses, exportación y flujo de estudio.
- **Bitwig Studio**: arquitectura moderna, Clip Launcher, modulación y dispositivos modulares en Linux.
- **Zrythm**: DAW open source, edición no destructiva, clips, piano roll, acordes y automatización.
- **Qtractor**: flujo Linux nativo de audio y MIDI.
- **Audacity**: edición precisa y herramientas de restauración, utilizada opcionalmente mediante integración externa.
- **SuperCollider, TidalCycles y Sonic Pi**: live coding, síntesis y patrones musicales.
- **openDAW existente**: referencia para SDK web, modelo headless, edición MIDI, escalas,
  instrumentos/efectos, grabación y futura capa WASM; se evaluará como integración o
  dependencia opcional, no como identidad del producto.

La combinación objetivo es:

```text
Ableton       composición no lineal y performance
Ardour        grabación, mezcla y routing
Bitwig        modulación y arquitectura modular
Live coding   composición programable
Estudio DAW    todo lo anterior + análisis musical + pedagogía + IA
```

## Capacidades principales

### Session View

Una matriz de clips de audio y MIDI para experimentar, improvisar y lanzar escenas:

- clips de audio, MIDI, acordes y automatización;
- escenas completas de verso, coro, puente o improvisación;
- lanzamiento cuantizado;
- loops y variaciones;
- follow actions;
- captura de una improvisación hacia Arrangement View;
- control MIDI en vivo.

### Arrangement View

Una línea temporal tradicional para terminar canciones:

- edición multipista;
- comping de tomas;
- punch in/out;
- automatización;
- edición vocal y de guitarra;
- marcadores de estructura;
- tempo y métrica;
- mezcla y exportación de stems.

### Edición de audio

La edición será no destructiva y basada en regiones, clips y operaciones:

- cortes, splits y consolidación;
- fades y crossfades;
- clip gain;
- slip editing;
- reverse;
- looping;
- time-stretch y pitch-shift;
- warp markers;
- cuantización de transitorios;
- comping;
- bounce/freeze;
- edición espectral futura.

El audio fuente nunca debe sobrescribirse automáticamente. Las operaciones deben producir una cadena editable y un render derivado cuando sea necesario.

### Audio, mezcla y routing

El motor debe soportar:

- pistas de audio, MIDI, instrumento, bus, retorno y master;
- routing libre entre pistas, buses, dispositivos y hardware;
- sends e inserts;
- sidechain;
- monitorización;
- compensación de latencia;
- automatización sample-accurate cuando sea viable;
- grupos de mezcla;
- freeze, bounce y render offline;
- exportación de mezcla, stems y múltiples formatos.

### Arquitectura híbrida CPU/GPU

Estudio DAW usará CPU y GPU de forma especializada, con fallback CPU completo.
La CPU será responsable del callback de audio, transporte, MIDI, routing y DSP
pequeño de baja latencia. La GPU se reservará para cargas paralelas o pesadas:

- FFT, espectrogramas y visualización de formas de onda;
- convolución larga y oversampling offline;
- time-stretch, pitch-shift y procesamiento espectral;
- limpieza de audio y separación de stems;
- generación de proxies y renders offline;
- análisis musical y operaciones sobre buffers grandes.

La primera abstracción prevista es `wgpu`, usando Vulkan en Linux cuando esté
disponible. Ningún nodo GPU podrá bloquear el callback de audio: los buffers se
preparan fuera del hilo RT, el scheduler compara el coste de transferencia y cada
operación conserva una ruta CPU segura. El objetivo es reducir el coste total del
proyecto, no trasladar trabajo a la GPU cuando la transferencia resulte más cara.

La política inicial y el benchmark CPU se pueden ejecutar así:

```bash
cargo test -p estudio-daw-compute-runtime
cargo run -q -p estudio-daw-compute-runtime --bin compute-benchmark
cargo run -q -p estudio-daw-compute-runtime --bin gpu-probe
```

### Plugins

Estudio DAW será principalmente un host extensible, no una reimplementación inmediata de todo el catálogo de instrumentos y efectos.

Formatos objetivo para escritorio:

- CLAP como prioridad abierta;
- LV2 para el ecosistema Linux;
- VST3 para compatibilidad práctica;
- MIDI, MPE, OSC, MCU/HUI y MIDI learn.

Opciones open source a integrar o usar como referencia:

- LSP Plugins;
- Guitarix;
- Surge XT;
- Decent Sampler;
- Dragonfly Reverb;
- x42 Plugins;
- Faust;
- plugins propios en Rust.

Opciones comerciales Linux a considerar sin hacerlas dependencias obligatorias:

- u-he: Diva, Zebra, Hive, Repro, Presswerk, Satin y otros;
- Pianoteq;
- Bitwig como referencia de arquitectura y flujo Linux.

La compatibilidad de cada plugin dependerá de su formato, arquitectura, licencia, sistema de activación y soporte Linux.

### Escalas, tonalidad y armonía

El core musical será scale-aware. Tonalidad, escala, modo, grados, acordes y afinación serán datos de primera clase del proyecto.

El sistema debe contemplar:

- tonalidad global;
- escala por clip;
- modos;
- menor natural, armónica y melódica;
- pentatónicas y blues;
- escalas personalizadas;
- guía visual en el piano roll;
- fold to scale;
- fit to scale;
- bloqueo de notas;
- cuantización melódica;
- transposición por grados;
- arpegiadores y generadores scale-aware;
- sugerencia de acordes diatónicos;
- análisis de funciones armónicas;
- afinaciones alternativas en una fase posterior.

El usuario debe poder elegir entre:

```text
Guía       resaltar la escala sin modificar notas
Asistencia sugerir o aplicar correcciones reversibles
Estricto   impedir o transformar notas fuera de la escala
```

Las notas cromáticas intencionales deben conservarse cuando el usuario así lo decida.

### Partituras, tablaturas y notación

Estudio DAW incluirá un editor y reproductor de partituras conectado al mismo modelo de notas, escalas, acordes, letras, MIDI y audio.

Capacidades previstas:

- pentagrama y múltiples pentagramas;
- clave de Sol, Fa y otras claves;
- compases, armaduras y cambios de métrica;
- notas, silencios, ligaduras y puntillos;
- articulaciones y dinámicas;
- tempo y marcas de expresión;
- acordes y cifrado americano;
- letras sincronizadas con notas;
- tablatura de guitarra;
- diagramas de acordes;
- transposición por tonalidad o intervalo;
- reproducción con seguimiento visual;
- edición vinculada al piano roll;
- importación desde MIDI y audio analizado;
- exportación a MIDI, MusicXML, MEI, PDF, SVG y formatos de texto musical.

La partitura no debe ser una imagen desconectada del proyecto. Una misma información musical debe poder verse como:

```text
partitura ↔ piano roll ↔ MIDI ↔ audio ↔ cifrado ↔ letra
```

MusicXML se utilizará como formato de intercambio y MEI como posible formato de representación/engraving. Verovio puede renderizar notación y convertir MusicXML a MEI, incluso mediante WebAssembly; VexFlow es otra opción para renderizado web de partituras y tablaturas. ([MusicXML](https://www.musicxml.com/publications/makemusic-recordare/notation-and-analysis/introduction/), [Verovio](https://www.verovio.org/), [VexFlow](https://vexflow.github.io/vexflow-docs/api/dev/))

#### Generación y mejora con IA

El profesor/productor virtual podrá:

- convertir una melodía vocal o MIDI en partitura;
- generar una partitura desde una progresión de acordes;
- armonizar una melodía;
- proponer contramelodías;
- crear arreglos para guitarra, piano, bajo, cuerdas o voz;
- simplificar una partitura para el nivel del usuario;
- mejorar digitaciones de guitarra;
- sugerir respiraciones y frases vocales;
- corregir ritmos imposibles o notación ambigua;
- detectar notas fuera de escala;
- explicar cada cambio realizado;
- generar ejercicios a partir de la partitura;
- crear una versión fácil, intermedia y avanzada.

La IA debe producir cambios estructurados sobre la partitura, no reemplazarla con una imagen opaca. Toda sugerencia debe poder compararse, aceptarse parcialmente, rechazarse o deshacerse.

Ejemplo:

```text
“Convierte mi melodía vocal en una partitura para voz y guitarra,
mantén la tonalidad de Mi menor, simplifica el ritmo del segundo verso
y propón una segunda voz solo en el coro.”
```

### Instrumentos digitales

Los instrumentos son uno de los desafíos técnicos principales. Estudio DAW será primero un excelente host y después desarrollará gradualmente su propio ecosistema.

Se diseñará un Instrument SDK común para instrumentos internos, plugins CLAP/VST3 y módulos WebAssembly:

```text
Instrument SDK
├── voice engine
├── polyphony y voice stealing
├── osciladores y filtros
├── envolventes y LFOs
├── matriz de modulación
├── MIDI/MPE
├── pitch bend y aftertouch
├── afinación
├── presets
├── automatización
└── renderizado native/WASM
```

Instrumentos propios previstos:

- sintetizador sustractivo;
- wavetable/FM;
- sampler multisample;
- instrumento granular;
- sampler de batería;
- piano/teclado;
- modelado de cuerdas;
- instrumentos orientados a voz, guitarra, ambient y texturas.

### Atmósferas y muros de sonido

Estudio DAW tendrá un sistema de diseño sonoro basado en capas, buses, moduladores, efectos y espacio.

Una atmósfera puede combinar:

```text
voz procesada
guitarra congelada
pad granular
ruido filtrado
reverb convolutiva
delay feedback
textura de campo
subgraves
```

Los Sound Walls deben permitir:

- duplicación y desafinación de capas;
- paneo y profundidad;
- expansión estéreo;
- filtros dinámicos;
- delays y reverbs en paralelo;
- feedback controlado;
- saturación;
- automatización espacial;
- macros como Density, Width, Movement, Darkness, Air y Warmth;
- escenas de sonido para verso, coro, puente o performance.

Efectos avanzados objetivo:

- reverberación algorítmica y convolutiva;
- shimmer reverb;
- delays multitap y reverse delay;
- granular synthesis;
- spectral freeze;
- filtros resonantes;
- wavefolding;
- distorsión multibanda;
- chorus, flanger y phaser;
- ring modulation;
- tape degradation;
- pitch shifting;
- looper;
- procesamiento Mid/Side;
- paneo binaural y espacial.

Los efectos pesados deben tener modos realtime y offline, con control de calidad, oversampling, latencia y consumo de CPU.

### Limpieza y restauración

Se priorizan herramientas no destructivas para:

- reducción de ruido;
- de-hum;
- de-click;
- de-crackle;
- de-clip;
- de-esser;
- gate;
- de-reverb;
- reparación espectral;
- normalización de pico y loudness;
- análisis LUFS, RMS, peak y true peak.

Audacity podrá utilizarse como editor externo mediante round-trip: Estudio DAW exporta una copia temporal, Audacity la procesa y el resultado vuelve como un nuevo take. La fuente original no se modifica.

### Metadatos

Se separarán los metadatos del proyecto de los metadatos embebidos en los archivos.

Metadatos del proyecto:

- artista;
- canción;
- versión;
- tonalidad;
- BPM;
- métrica;
- acordes;
- letra;
- instrumentos;
- tomas;
- notas de producción;
- historial pedagógico.

Metadatos de archivo:

- ID3;
- FLAC/Vorbis Comments;
- MP4/M4A;
- WAV/BWF;
- AIFF;
- portada;
- artista, álbum, género y fecha;
- idioma;
- compositor;
- copyright;
- ISRC cuando corresponda.

Se evaluarán TagLib y FFmpeg para lectura, escritura y conversión de metadata.

## Análisis musical local

Los análisis pesados deben correr localmente y fuera del hilo de audio:

- separación de stems;
- voz, bajo, batería y acompañamiento;
- transcripción de letras en inglés y español;
- timestamps de palabras;
- detección de tonalidad y tempo;
- acordes y beats;
- melodía vocal;
- líneas instrumentales;
- conversión audio a MIDI;
- rango y afinación vocal;
- comparación entre interpretación y referencia.
- extracción de notación, ritmo y estructura para generar partituras.

Herramientas previstas:

- Demucs;
- Whisper/faster-whisper;
- WhisperX;
- Basic Pitch;
- MT3;
- madmom/madmom-infer;
- librosa y pYIN;
- Parselmouth/Praat.

Todos los resultados deben incluir timestamps, versión del modelo y confianza cuando sea posible. El usuario debe poder corregirlos.

## Profesor y productor virtual

La LLM funcionará como profesor, productor, compositor y asistente de sesión, pero nunca tendrá control irrestricto del sistema.

Herramientas musicales tipadas previstas:

```text
create_track
load_instrument
set_transport
record_arm
create_clip
modify_midi
apply_effect
request_analysis
explain_music
generate_exercise
save_learning_event
render_preview
undo_last_change
```

Ejemplos de interacción:

```text
“Crea un bajo que siga las fundamentales del verso y deje espacio para la voz.”
“¿Por qué B mayor funciona aquí si estoy en Mi menor?”
“Haz que el coro tenga un muro de sonido más ancho, sin tapar la voz.”
“Dame un ejercicio para reconocer la tercera menor de esta canción.”
```

Las operaciones de la IA deben mostrar una previsualización, requerir confirmación cuando corresponda y poder deshacerse como una unidad.

## Live coding y proyecto reproducible

El proyecto tendrá una representación visual y otra declarativa. El usuario podrá modificar la misma sesión desde la interfaz, el lenguaje musical o el agente.

Ejemplo conceptual:

```text
song "demo" {
  tempo 92
  key "E minor"
  scale "harmonic minor"

  track voice { input "interface:mic" }
  track guitar { input "interface:in1" }
  track bass { instrument "native.synth" }
}
```

Todo cambio musical debe quedar registrado para permitir deshacerlo y rehacerlo,
revisarlo, aprender de él y reproducirlo de manera determinista.

## Arquitectura técnica

```text
Estudio DAW Core
├── project-model
├── transport
├── clips y escenas
├── MIDI events
├── automation
├── scales y harmony
├── notation y score model
├── scripting
├── command bus
└── serialization

Estudio DAW Runtime
├── realtime audio graph
├── mixer y routing
├── DSP
├── instruments
├── plugin host
└── latency compensation

Platform Backends
├── PipeWire / ALSA / JACK
├── native MIDI
├── Web Audio / AudioWorklet
├── Web MIDI
└── WebAssembly bindings

Python Workers
├── separation
├── transcription
├── pitch
├── harmony
├── notation
├── feedback
└── tutor adapter
```

Rust será responsable del núcleo determinista y del tiempo real. Python correrá como worker separado para modelos de IA y análisis. PyO3/maturin podrá usarse para bindings puntuales, pero los modelos largos no se ejecutarán dentro del callback de audio.

## Portabilidad WebAssembly

La aplicación tendrá dos objetivos:

```text
Estudio DAW Studio
  aplicación nativa Linux para hardware, plugins y baja latencia

Estudio DAW Web
  versión WASM para composición, edición, aprendizaje y colaboración
```

El modelo de proyecto, transporte, clips, escalas, scripting y parte del DSP deben ser portables a WebAssembly.

La versión web utilizará:

- Rust/WASM;
- Web Audio API;
- AudioWorklet;
- Web MIDI cuando esté disponible;
- WAM o plugins WASM;
- almacenamiento local y exportación de proyectos.

La versión nativa seguirá siendo la referencia para baja latencia, interfaces de audio, controladores MIDI y plugins CLAP/LV2/VST3.

## Principios de ingeniería

1. Linux-first y sin dependencia de Windows.
2. Rust para el núcleo determinista y de tiempo real.
3. Python para análisis musical y modelos de IA.
4. El core no debe depender de PipeWire, DOM, Python ni un proveedor LLM específico.
5. El callback de audio no puede asignar memoria, bloquear, acceder a disco, usar red ni invocar modelos.
6. La IA opera mediante herramientas musicales tipadas, no mediante clicks arbitrarios.
7. Ningún audio fuente se sobrescribe automáticamente.
8. Todo cambio musical debe ser reversible y quedar registrado.
9. Los resultados de análisis son editables y muestran incertidumbre.
10. Los plugins y assets externos deben respetar sus licencias.
11. La aplicación debe ser útil antes de tener instrumentos propios completos.
12. El host se construye primero; el ecosistema de instrumentos propios se construye progresivamente.

## Estructura del proyecto

```text
estudio-daw/
├── crates/       núcleo Rust y adaptadores
│   ├── session/   estado de sesión y CommandBus bounded
│   ├── midi-engine/
│   ├── audio-engine/ cadena DSP in-place y RenderPlan
│   ├── synth/       instrumento MIDI nativo experimental
│   ├── dsp/       procesamiento DSP modular
│   ├── command-bus/ comandos de dominio versionados
│   ├── application/ ciclo de vida y API común para UI/CLI
│   ├── ui-shell/    shell de escritorio Tauri y adaptador web
│   └── cli/       adaptador CLI (ejecutable estudio-daw-project)
├── python/       (pendiente) workers aislados de análisis
├── docs/         decisiones y documentación
├── tests/        pruebas unitarias, audio y MIDI
├── examples/     sesiones y scripts
└── openspec/     propuesta, diseño, specs y tareas
```

## Estado

Estado real: prototipo fundacional ejecutable; todavía no es un DAW de producción.
Existe un shell Tauri, modelo/API de aplicación, MIDI live, grabación de audio
PipeWire, motor DSP inicial, proxies, contratos de visualización y un sinte
sinusoidal polifónico conectado al `RenderPlan`. La edición visual y los workers
de análisis siguen en desarrollo; el sinte es un instrumento de validación, no
un banco de sonidos completo.

El flujo probado actualmente es:

```text
Arturia KeyLab → ALSA MIDI → toma JSON → MidiClip → FluidSynth → PipeWire
```

Comandos de laboratorio:

```bash
# Ver destinos MIDI ALSA
cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- midi-outputs

# Reproducir una toma directamente
cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- midi-play mi-toma.json "FLUID Synth"

# Reproducir un clip que ya pertenece a project.json
cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- project-play proyecto.json midi-clip-1 "FLUID Synth"

# Reproducir un clip controlando el transporte desde el puerto DAW del KeyLab
cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- project-play-live proyecto.json "" "FluidSynth" "KeyLab Essential 49 DAW"

# Grabar MIDI iniciando y deteniendo con el botón Record del KeyLab
cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- midi-record-live mi-toma-live.json "KeyLab Essential 49 MID" "KeyLab Essential 49 DAW"
```

Durante `project-play`, la terminal acepta `p` para pausar/reanudar, `s` para
detener, `l` para activar/desactivar loop y `q` para salir. Cada comando requiere
presionar Enter. Las operaciones de cuantización generan un archivo nuevo y no
sobrescriben la toma original.

La primera implementación debe comenzar por un vertical slice:

```text
KeyLab → MIDI → sintetizador propio → grabación MIDI → reproducción
interfaz de audio → grabación de guitarra/voz
proyecto → guardado y reapertura
```

El orden de trabajo y las dependencias por fases se mantienen en
[`docs/architecture-v2.md`](docs/architecture-v2.md) y en las tareas OpenSpec.
Entre los bloques grandes aún pendientes están el instrumento nativo conectado
al audio engine, edición/mixer visual, procesamiento GPU medido, plugins,
automatización, workers Python (stems, transcripción y análisis musical),
scripting, profesor IA e implementación WASM.

## Documentación de planificación

- [Architecture Reset v2](docs/architecture-v2.md)
- [Estado de sesión y CommandBus](docs/session-command-bus.md)
- [API de aplicación y ciclo de proyecto](docs/application-api.md)
- [Bus de comandos del dominio](docs/domain-command-bus.md)
- [Audio RenderPlan](docs/audio-render-plan.md)
- [Backend PipeWire](docs/audio-pipewire.md)
- [MediaSource y proxies](docs/proxy-assets.md)
- [Control MIDI live](docs/midi-live-control.md)
- [Frontera Core/UI](docs/ui-architecture.md)
- [Shell de escritorio Tauri](crates/ui-shell/tauri.conf.json)
- [Contrato versionado UI/backend v1](docs/ui-bridge-contract-v1.md)
- [Migraciones de `project.json`](docs/project-schema-migrations.md)
- [Primer instrumento MIDI nativo](docs/native-instrument.md)
- [Propuesta inicial](openspec/changes/bootstrap-opendaw-architecture/proposal.md)
- [Diseño](openspec/changes/bootstrap-opendaw-architecture/design.md)
- [Especificación de audio y MIDI](openspec/changes/bootstrap-opendaw-architecture/specs/runtime-audio-midi/spec.md)
- [Especificación del modelo de proyecto](openspec/changes/bootstrap-opendaw-architecture/specs/project-model/spec.md)
- [Especificación de workers de análisis](openspec/changes/bootstrap-opendaw-architecture/specs/analysis-workers/spec.md)
- [Especificación de scripting y agente](openspec/changes/bootstrap-opendaw-architecture/specs/scripting-and-agent/spec.md)
- [Tareas](openspec/changes/bootstrap-opendaw-architecture/tasks.md)

### Prototipo parcial de shell Tauri

La primera interfaz de escritorio usa Tauri como adaptador sobre
`estudio-daw-application`; el frontend está separado del bridge Tauri. Esta
ventana permite abrir/guardar proyectos JSON, inspeccionar pistas y clips,
crear una sesión vacía, añadir pistas MIDI/audio vacías o cargar una Demo MIDI,
reproducir clips MIDI de las pistas de instrumento y controlar transporte e
historial. Una pista de audio nueva persiste su disposición estéreo y su destino
interno al Master; Arreglo, Session y Mezclador muestran las pistas desde la
misma instantánea. Session sólo muestra encabezados/casillas vacías y Mezclador
es de sólo lectura. Se pueden importar archivos de audio a pistas existentes,
copiándolos a la carpeta `media` junto al proyecto o vinculando el original,
con metadatos básicos, preescucha corta antes de confirmar la importación,
selección mono/estéreo de canales de origen, posición inicial por compás y
forma de onda real reducida.
Es un prototipo de capacidades parciales: su composición
visual actual no satisface todavía el flujo creativo de Ableton Live 12; no
incluye lanzamiento de clips en Session, edición de clips/notas, reproducción
de regiones de audio, ruteo/downmix de canales durante la reproducción,
selección de salidas físicas, grabación live ni monitorización de entrada. No
se declara terminada la interfaz. La propuesta y las tareas abiertas de
[workstation-arrangement-surface-v2](openspec/changes/workstation-arrangement-surface-v2/)
registran Ableton Live 12 como referencia de UX prioritaria y Ardour sólo como
consulta técnica secundaria para el flujo de audio.

```bash
cargo run -p estudio-daw-ui-shell
```

El IPC transporta comandos y resúmenes serializables, nunca PCM ni buffers GPU.
La reproducción y DSP permanecen en Rust; futuras waveform/espectrogramas usarán
datos derivados y acotados. Para la primera compilación en Arch/Linux se requiere
WebKitGTK 4.1 y GTK 3, además de las dependencias de desarrollo de Tauri.
La guía de [diagnóstico de la UI](docs/ui-debugging.md) contiene comandos de
verificación, resultados esperados y límites funcionales conocidos.
