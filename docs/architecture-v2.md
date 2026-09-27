# Estudio DAW — Architecture Reset v2

Este documento congela las decisiones de arquitectura que deben guiar la siguiente
fase. El objetivo es reducir deuda técnica antes de añadir más instrumentos,
shaders o UI.

## 1. Principio rector

Estudio DAW será un **modular monolith**: un solo producto distribuible, con
crates y procesos separados donde los límites reduzcan riesgo. No se crearán
microservicios de red para resolver problemas que pueden resolverse con módulos,
colas bounded y contratos locales.

```text
                    ┌─────────────────────────┐
                    │ UI / CLI / Live Coding   │
                    └────────────┬────────────┘
                                 │ Commands / Events
                    ┌────────────▼────────────┐
                    │ Portable Domain Core    │
                    │ Project · Music · Undo  │
                    └──────┬─────────┬────────┘
                           │         │
             RT snapshots  │         │ Jobs / artifacts
                           │         │
                 ┌─────────▼───┐ ┌───▼──────────────┐
                 │ Audio Engine │ │ Worker Runtime   │
                 │ DAG · MIDI   │ │ Python · GPU     │
                 └──────┬───────┘ └──────┬───────────┘
                        │                │
                 ┌──────▼──────┐  ┌──────▼───────────┐
                 │ PipeWire /  │  │ mmap / artifacts │
                 │ JACK / ALSA │  │ analysis cache   │
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

### Compute Runtime

Expone una interfaz de jobs CPU/GPU. `wgpu`/Vulkan se inicializa fuera del audio
runtime. Cada job declara memoria, transferencia, deadline y precisión. Si no
cumple el presupuesto, se ejecuta por CPU.

### Workers

Python es un proceso aislado para MIR, separación, transcripción y modelos. El
core recibe `ArtifactRef` y manifiestos simbólicos; nunca audio masivo serializado
en JSON.

### UI

La UI solo lee snapshots y emite commands. No muta estructuras del dominio
directamente. El primer prototipo puede usar egui; los canvas de timeline,
piano-roll, waveform y espectrograma deben poder migrar a renderers wgpu propios.

## 3. Patrón de estado y mutaciones

No se usará event sourcing completo desde el primer día. Se usará **event-sourcing
ligero**:

```text
Command → Validate → Apply → DomainEvent → Snapshot → Persist
                         │
                         └── Undo inverse/transaction
```

Cada comando debe tener:

- identificador y versión;
- autor (`user`, `script`, `agent`, `import`);
- precondiciones;
- operación inversa o snapshot transaccional;
- eventos resultantes;
- diagnóstico estructurado.

Las operaciones de IA nunca escriben directamente. Generan un `ChangeSet` con
preview, diff musical, coste estimado y undo agrupado.

## 4. Matriz de tecnologías

| Área | Decisión | Motivo | Riesgo controlado |
|---|---|---|---|
| Audio I/O | PipeWire + JACK + ALSA MIDI | Linux profesional y routing real | aislar APIs en Platform |
| UI inicial | egui | iteración rápida y Rust/WASM | virtualizar canvas grandes |
| Canvas musical | wgpu | waveform, piano roll y GPU compute | fallback software/CPU |
| DSP primitives | evaluar dasp y tpt-dsp | reutilizar piezas RT-safe | auditoría de licencia/API |
| Grafo DSP | propio | diferenciador y control RT | pruebas de DAG y no-allocation |
| Plugins | CLAP primero, LV2 después, VST3 como adapter | capacidades modernas y Linux | sandbox, ABI y estados |
| Plugins propios | NIH-plug sólo en crate aislado | acelerar prototipos | mantenimiento/licencia VST3 |
| GPU | wgpu/Vulkan | portable y compatible con WASM futuro | transferencias/deadlines |
| Workers | Python + mmap + protocolo versionado | ML y MIR sin contaminar RT | watchdog y artefactos |
| Persistencia | project bundle + JSON versionado | legible, portable y migrable | no guardar audio grande inline |
| Calidad | cargo test, proptest, criterion, cargo-deny, tracing | prevenir regresiones | CI reproducible |

Tracktion Engine queda como referencia de features y arquitectura, no dependencia:
su engine es C++/JUCE y usa licencia dual GPL3/comercial. JUCE y NIH-plug requieren
auditoría de licencia antes de cualquier distribución comercial.

## 5. Lecciones de la competencia

- **Bitwig**: separar Launcher y Arranger como secuenciadores relacionados, no
  como una sola vista con estados ambiguos. Incorporar captura de improvisación,
  clips enlazados y scale-awareness.
- **Ardour**: tomar en serio routing, buses, compensación de latencia, exportación
  y automatización sample-accurate.
- **Zrythm**: conectar escalas, acordes, chord track y piano roll como un mismo
  modelo musical.
- **Waveform/Tracktion Engine**: estudiar separación entre engine y UI, render
  background, cache y transporte multi-CPU.
- **REAPER**: priorizar ligereza, acciones, scripting, extensibilidad y diagnóstico
  visible.
- **Ableton**: usar como referencia de flujo creativo, no como modelo interno ni
  copia de interfaz.

## 6. Deuda técnica detectada y corrección

| Deuda actual | Corrección v2 |
|---|---|
| CLI concentra demasiadas responsabilidades | crate `cli` delgado sobre comandos |
| `MidiTake` embebido directamente en clip | `TakeRef` + artifact store versionado |
| transporte disperso entre recorder/player | `TransportSnapshot` global |
| sin undo formal | `CommandBus` + `ChangeSet` |
| runtime ALSA mezclado con dominio | `platform-linux` |
| GPU sólo sondada | `ComputeJob` + scheduler + benchmark real |
| sin audio graph real | `audio-engine` con `RenderPlan` |
| sin migraciones | schema version + migrators probados |
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

### Fase C — UI mínima

- ventana nativa;
- mixer y transporte;
- piano roll scale-aware;
- timeline con virtualización;
- diagnóstico de audio/GPU visible.

### Fase D — compute

- FFT y convolución CPU optimizadas;
- shader WGSL equivalente;
- benchmark incluyendo transferencia;
- scheduler real CPU/GPU;
- degradación segura y cache de pipelines.

### Fase E — producción musical

- audio clips y proxies;
- automatización;
- CLAP/LV2;
- instrumentos nativos;
- análisis Python y stems.

### Fase F — profesor y WASM

- manifiesto simbólico;
- ChangeSets generados por IA;
- ejercicios y progreso;
- domain portable compilado a WASM;
- colaboración y navegador como fases posteriores.

## 8. Criterios de “listo” para no acumular deuda

Una nueva feature no entra al core si no tiene:

1. dueño de estado claramente definido;
2. contrato de error y diagnóstico;
3. estrategia de undo o justificación de inmutabilidad;
4. prueba de serialización/migración si persiste;
5. prueba offline si toca DSP o compute;
6. presupuesto de tiempo real si toca audio;
7. fallback si depende de GPU, plugin, dispositivo o worker;
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
