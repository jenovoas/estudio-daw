# Estudio DAW — Revisión de arquitectura

Este documento registra las decisiones de arquitectura que deben guiar la
siguiente fase. El objetivo es reducir deuda técnica antes de añadir más
instrumentos, sombreadores o interfaz.

## 1. Principio rector

Estudio DAW será un **monolito modular**: un solo producto distribuible, con
crates y procesos separados donde los límites reduzcan riesgos. No se crearán
microservicios de red para resolver problemas que pueden atenderse con módulos,
colas acotadas y contratos locales.

```text
                    ┌─────────────────────────┐
                    │ Interfaz / CLI / código en vivo │
                    └────────────┬────────────┘
                                 │ Comandos / eventos
                    ┌────────────▼────────────┐
                    │ Núcleo de dominio portable │
                    │ Proyecto · música · deshacer │
                    └──────┬─────────┬────────┘
                           │         │
             Instantáneas RT │         │ Trabajos / artefactos
                           │         │
                 ┌─────────▼───┐ ┌───▼──────────────┐
                 │ Motor de audio │ │ Entorno de procesos │
                 │ DAG · MIDI     │ │ Python · GPU        │
                 └──────┬───────┘ └──────┬───────────┘
                        │                │
                 ┌──────▼──────┐  ┌──────▼───────────┐
                 │ PipeWire /  │  │ mmap / artefactos │
                 │ JACK / ALSA │  │ caché de análisis │
                 └─────────────┘  └──────────────────┘
```

## 2. Límites obligatorios

### Dominio portable

Rust puro, sin PipeWire, ALSA, GPU, interfaz gráfica, Python ni modelos de lenguaje. Contiene:

- proyecto y migraciones;
- pistas, clips, takes y escenas;
- notas que reconocen la escala, acordes y tonalidad;
- transporte abstracto;
- comandos, eventos y operaciones para deshacer/rehacer;
- manifiestos de análisis;
- serialización y adaptador DAWproject.

### Motor de audio

Sólo recibe instantáneas y comandos previamente validados. Su llamada de
retorno no conoce JSON, disco, red, procesos auxiliares ni interfaz gráfica.
Compila un `RenderPlan` fuera de esa llamada y ejecuta búferes preasignados en
orden topológico.

### Plataforma

Contiene PipeWire, JACK, ALSA MIDI, descubrimiento de dispositivos y permisos.
Ningún tipo concreto de una plataforma debe filtrarse al Portable Domain.

### Modelos y adaptadores MIDI/medios

`estudio-daw-midi-types` contiene sólo estructuras MIDI serializables. ALSA y
los diagnósticos de puertos viven en `estudio-daw-midi-engine` y sus adaptadores.
`estudio-daw-project-model` consume los tipos MIDI puros y no incorpora
dependencias de PipeWire, ALSA, el motor de audio ni diagnósticos de plataforma.
La inspección de fuentes y la generación/validación de proxies con
`ffmpeg`/`ffprobe` se ejecutan desde `estudio-daw-media-adapter`, fuera del
modelo portable. La conversión DAWproject recibe/devuelve bytes en memoria; la
CLI o los adaptadores de aplicación abren y guardan las rutas de archivos.

### Entorno de cálculo

Expone una interfaz de trabajos CPU/GPU. `wgpu`/Vulkan se inicializa fuera del
entorno de audio. Cada trabajo declara memoria, transferencia, plazo y
precisión. Si no cumple el presupuesto, se ejecuta por CPU.

### Workers

Python es un proceso aislado para recuperación de información musical (MIR),
separación, transcripción y modelos. El núcleo recibe `ArtifactRef` y manifiestos
simbólicos; nunca audio masivo serializado en JSON.

### Interfaz

La interfaz sólo lee instantáneas y emite comandos. No modifica directamente
las estructuras del dominio. El primer prototipo puede usar egui; las superficies
de línea de tiempo, rollo de piano, forma de onda y espectrograma deben poder
migrar a renderizadores propios con wgpu.

## 3. Patrón de estado y mutaciones

No se usará abastecimiento completo de eventos desde el primer día. Se usará
**abastecimiento ligero de eventos**:

```text
Command → Validate → Apply → DomainEvent → Snapshot → Persist
                         │
                         └── Undo inverse/transaction
```

Cada comando debe tener:

- identificador y versión;
- autor (`user`, `script`, `agent`, `import`);
- precondiciones;
- operación inversa o instantánea transaccional;
- eventos resultantes;
- diagnóstico estructurado.

Las operaciones de IA nunca escriben directamente. Generan un `ChangeSet` con
preview, diff musical, coste estimado y undo agrupado.

## 4. Matriz de tecnologías

| Área | Decisión | Motivo | Riesgo controlado |
|---|---|---|---|
| Entrada/salida de audio | PipeWire + JACK + ALSA MIDI | Linux profesional y ruteo real | aislar las API en la capa de plataforma |
| Interfaz inicial | egui | iteración rápida y Rust/WASM | virtualizar superficies grandes |
| Superficie musical | wgpu | forma de onda, rollo de piano y cálculo GPU | alternativa por software/CPU |
| Primitivas DSP | evaluar dasp y tpt-dsp | reutilizar piezas seguras para tiempo real | auditoría de licencia/API |
| Grafo DSP | propio | diferenciador y control RT | pruebas de DAG y no-allocation |
| Complementos | CLAP primero, LV2 después, VST3 como adaptador | capacidades modernas y Linux | aislamiento, ABI y estados |
| Complementos propios | NIH-plug sólo en crate aislado | acelerar prototipos | mantenimiento/licencia VST3 |
| GPU | wgpu/Vulkan | portable y compatible con WASM futuro | transferencias/plazos |
| Procesos auxiliares | Python + mmap + protocolo versionado | aprendizaje automático y recuperación musical sin contaminar tiempo real | vigilancia y artefactos |
| Persistencia | paquete de proyecto + JSON versionado | legible, portable y migrable | no guardar audio grande dentro del documento |
| Calidad | cargo test, proptest, criterion, cargo-deny, tracing | prevenir regresiones | integración continua reproducible |

Tracktion Engine queda como referencia de capacidades y arquitectura, no como dependencia:
su motor es C++/JUCE y usa licencia dual GPL3/comercial. JUCE y NIH-plug requieren
auditoría de licencia antes de cualquier distribución comercial.

## 5. Lecciones de la competencia

- **Bitwig**: separar el lanzador de clips y el arreglista como secuenciadores relacionados, no
  como una sola vista con estados ambiguos. Incorporar captura de improvisación,
  clips enlazados y adaptación a escalas.
- **Ardour**: tomar en serio el ruteo, los buses, la compensación de latencia, la exportación
  y la automatización precisa por muestra.
- **Zrythm**: conectar escalas, acordes, pista de acordes y rollo de piano como un mismo
  modelo musical.
- **Waveform/Tracktion Engine**: estudiar la separación entre motor e interfaz, renderizado
  en segundo plano, caché y transporte multinúcleo.
- **REAPER**: priorizar ligereza, acciones, scripting, extensibilidad y diagnóstico
  visible.
- **Ableton**: usar como referencia de flujo creativo, no como modelo interno ni
  copia de interfaz.

## 6. Deuda técnica detectada y corrección

| Deuda actual | Corrección v2 |
|---|---|
| CLI concentra demasiadas responsabilidades | crate `cli` delgado sobre comandos |
| `MidiTake` embebido directamente en clip | `TakeRef` + almacén versionado de artefactos |
| transporte disperso entre grabador y reproductor | `TransportSnapshot` global |
| falta deshacer formal | `CommandBus` + `ChangeSet` |
| entorno ALSA mezclado con el dominio | `platform-linux` |
| GPU sólo sondeada | `ComputeJob` + planificador + referencia de rendimiento real |
| falta un grafo de audio real | `audio-engine` con `RenderPlan` |
| faltan migraciones | versión de esquema + migradores probados |
| sin observabilidad consistente | `tracing` + diagnósticos con correlación |
| plugins no definidos | contrato de host, estado y sandbox |

## 7. Fases revisadas

### Fase A — arquitectura ejecutable

- separar crates y contratos;
- estabilizar `Project`, `Session`, `TransportSnapshot`;
- implementar `CommandBus`, `ChangeSet` y undo;
- migrar CLI a comandos de dominio;
- añadir `cargo-deny`, benchmarks y trazas.

### Fase B — audio vertical

- stream PipeWire real;
- DAG de una pista MIDI, instrumento, mixer y master;
- AudioBox capture/playback;
- medición de xruns, latencia y asignaciones;
- perfiles KeyLab/AudioBox.

### Fase C — interfaz mínima

- ventana nativa;
- mezclador y transporte;
- rollo de piano adaptado a escalas;
- línea de tiempo con virtualización;
- diagnóstico de audio/GPU visible.

### Fase D — cálculo

- FFT y convolución CPU optimizadas;
- sombreador WGSL equivalente;
- benchmark incluyendo transferencia;
- planificador real CPU/GPU;
- degradación segura y caché de procesos gráficos.

### Fase E — producción musical

- audio clips y proxies;
- automatización;
- complementos CLAP/LV2;
- instrumentos nativos;
- análisis Python y stems.

### Fase F — profesor y WASM

- manifiesto simbólico;
- ChangeSets generados por IA;
- ejercicios y progreso;
- dominio portable compilado a WASM;
- colaboración y navegador como fases posteriores.

## 8. Criterios de “listo” para no acumular deuda

Una nueva capacidad no entra al núcleo si no tiene:

1. dueño de estado claramente definido;
2. contrato de error y diagnóstico;
3. estrategia de undo o justificación de inmutabilidad;
4. prueba de serialización/migración si persiste;
5. prueba offline si toca DSP o compute;
6. presupuesto de tiempo real si toca audio;
7. alternativa si depende de GPU, complemento, dispositivo o proceso auxiliar;
8. decisión de licencia y procedencia si reutiliza código externo.

## Fuentes de la auditoría

- [Bitwig: Arranger y Clip Launcher](https://www.bitwig.com/userguide/latest/one_daw_two_sequencers/)
- [Bitwig: arquitectura y formatos abiertos](https://www.bitwig.com/modern-foundations/)
- [Ardour: features y plugins](https://ardour.org/features.html)
- [Zrythm: acordes, escalas y plugins](https://www.zrythm.org/cs/index.html)
- [Tracktion Engine](https://github.com/Tracktion/tracktion_engine/blob/develop/FEATURES.md)
- [Tracktion Engine: licencia](https://github.com/Tracktion/tracktion_engine/blob/develop/LICENSE.md)
- [CLAP](https://github.com/free-audio/clap)
- [dasp](https://github.com/RustAudio/dasp)
- [tpt-dsp](https://github.com/tpt-solutions/tpt-dsp)
- [wgpu compute](https://docs.rs/wgpu/latest/wgpu/struct.ComputePass.html)
- [egui](https://docs.rs/egui/latest/egui/index.html)
- [Slint renderers](https://docs.slint.dev/latest/docs/slint/guide/backends-and-renderers/backends_and_renderers/)
