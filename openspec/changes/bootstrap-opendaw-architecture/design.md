# Diseño: arquitectura fundacional de Estudio DAW

## 0. Relación con el openDAW existente

El proyecto `openDAW` de [opendaw.org](https://opendaw.org/en) es una DAW web
open source existente, con SDK reutilizable, editor de notas, escalas, routing,
grabación, instrumentos, efectos y una arquitectura TypeScript/Web Audio. Su
licencia AGPL y sus términos de distribución deben revisarse antes de incorporar
código o dependencias. Estudio DAW lo tratará como:

- referencia de producto y UX para el futuro modo navegador;
- candidato a integración mediante SDK/adaptador, si los contratos y licencia lo
  permiten;
- fuente de ideas para interoperabilidad, no como nombre ni branding del proyecto;
- comparación técnica frente al motor nativo Rust/PipeWire/JACK que necesitamos
  para producción Linux local.

La estrategia será mantener una frontera de dominio portable: proyecto, notas,
escala, clips, comandos y manifiestos. Así podremos experimentar con un adaptador
web sin contaminar el callback nativo ni obligar al runtime Linux a depender de
TypeScript.

## 1. Arquitectura de alto nivel

```text
                    ┌─────────────────────────────┐
                    │ UI / editor / piano roll    │
                    └──────────────┬──────────────┘
                                   │ commands/events
                    ┌──────────────▼──────────────┐
                    │ Project + Session Model     │
                    │ versionado / undo / schema  │
                    └───────┬───────────┬─────────┘
                            │             │
              realtime      │             │ jobs/tools
                    ┌───────▼──────┐  ┌──▼──────────────┐
                    │ Rust Engine  │  │ Python Workers  │
                    │ audio/MIDI   │  │ MIR/AI          │
                    └───────┬──────┘  └──┬──────────────┘
                            │             │
                    ┌───────▼──────┐  ┌──▼──────────────┐
                    │ PipeWire /   │  │ LLM adapter     │
                    │ ALSA / JACK  │  │ tool calling    │
                    └──────────────┘  └─────────────────┘
```

## 2. Límites de tiempo real

El callback de audio solo podrá leer/escribir buffers preasignados y consultar estado atómico o estructuras lock-free. No podrá:

- asignar memoria;
- bloquear mutexes no diseñados para tiempo real;
- acceder a disco;
- hacer llamadas de red;
- invocar Python;
- ejecutar una LLM;
- escribir logs sincrónicos;
- esperar a otro hilo.

Las solicitudes externas se convierten en comandos de sesión, se validan fuera del callback y se aplican en un punto de sincronización musical seguro.

## 3. Workspace Rust propuesto

```text
crates/
  audio-engine/       streams, buffers, clock y callbacks
  midi-engine/        dispositivos, mensajes y routing MIDI
  transport/          tempo, compás, beat, sample position
  project-model/      tracks, clips, takes, automation y schema
  command-bus/        comandos, eventos, undo y redo
  dsp/                unidades DSP propias
  synth/              instrumentos nativos iniciales
  plugin-host/        CLAP/LV2/VST3 en fases posteriores
  scripting/          parser/runtime del lenguaje musical
  analysis-protocol/  jobs, artefactos y eventos con Python
  ui/                 aplicación gráfica
```

La interfaz pública entre estos crates debe preferir tipos de dominio explícitos sobre strings sueltos.

## 4. Workers Python

Python corre como proceso separado para aislar memoria, GIL, dependencias de PyTorch y fallos de modelos. Se comunica mediante un protocolo versionado sobre Unix socket o gRPC local. Los audios se referencian por rutas de artefacto; no se serializan muestras completas en JSON.

Workers iniciales:

- `separation`: voces, bajo, batería y acompañamiento;
- `transcription`: letras y timestamps;
- `pitch`: melodía vocal e instrumentos;
- `harmony`: acordes, tonalidad y beats;
- `feedback`: métricas y ejercicios;
- `tutor-adapter`: preparación de contexto para la LLM.

PyO3/maturin queda reservado para funciones pequeñas de alto rendimiento o bindings estables, no como canal principal para modelos largos.

## 5. Modelo de proyecto

Un proyecto debe ser portable, versionable y parcialmente legible:

```text
song.estudiodaw/
  project.json
  timeline.json
  tracks/
  midi/
  audio/
  analysis/
  presets/
  scripts/
  renders/
```

El audio grande se almacena como artefacto; el estado musical se almacena como metadatos y eventos. Cada operación mutante debe tener una representación serializable para undo/redo y colaboración futura.

## 6. Lenguaje musical

El lenguaje debe ser declarativo y editable en vivo. Debe describir transporte, pistas, clips, instrumentos, patrones, acordes, automatización y análisis. La primera versión no necesita permitir DSP arbitrario; debe controlar el modelo de sesión con seguridad.

Ejemplo conceptual:

```text
song "demo" {
  tempo 92
  key "E minor"

  track voice { input "interface:mic" }
  track guitar { input "interface:in1" }
  track bass { instrument "native.synth" }
}
```

## 7. Agente y profesor virtual

La LLM nunca modifica directamente archivos ni dispositivos. Solo puede invocar herramientas tipadas, con permisos y validación:

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
```

Las mutaciones peligrosas o destructivas requieren preview y confirmación. Toda operación debe poder deshacerse.

## 8. Decisiones pendientes

- Rust UI nativa frente a UI web embebida.
- PipeWire directo frente a abstracción CPAL/JACK.
- JSON-RPC, MessagePack o Protobuf para workers.
- Formato definitivo de proyecto.
- Parser propio frente a extensión de un lenguaje existente.
- Licencia del núcleo y de los assets.
- Estrategia de hosting de plugins.
- Modelo LLM y política de privacidad.

## 9. Auditoría de reutilización del openDAW existente

La documentación y el repositorio público del openDAW existente muestran un SDK
headless, grabación de audio/MIDI, escalas, dispositivos, bundles, gestión de
samples, DAWproject y módulos WASM. Su runtime está orientado a TypeScript/Web
Audio, por lo que no sustituye nuestro núcleo Linux nativo.

| Área | Decisión para Estudio DAW |
|---|---|
| Modelo de proyecto | Reutilizar conceptos de bundles y undo/redo; implementar modelo Rust con migraciones. |
| Interoperabilidad | Adoptar DAWproject como formato de intercambio, no como modelo interno único. |
| Web/WASM | Evaluar SDK/adaptador en un entorno aislado; mantener el dominio portable. |
| Audio runtime | Implementar DAG, arena RT y PipeWire/JACK propios en Rust. |
| MIDI y notas | Reutilizar semántica y casos de uso; implementar contrato nativo. |
| Instrumentos/efectos | Usar catálogo como referencia; auditar licencia antes de portar código. |
| Media | Inspirar `MediaSource`/`ProxyAsset`; usar caché local en disco Linux. |
| Scripting | Mantener Werkstatt/Spielwerk como referencia web; el agente nativo usará `ChangeSet`. |
| IA | Construir nuestro manifiesto simbólico, profesor bilingüe y análisis local. |
| Licencia | Registrar atribuciones y dependencias antes de incorporar cualquier componente. |

### Decisión de arquitectura

No haremos un fork directo. El producto tendrá tres superficies:

1. **Native Core**: Rust, PipeWire/JACK/ALSA, callback RT, DAG DSP, plugins y hardware.
2. **Portable Domain**: proyecto, notas scale-aware, clips, comandos, manifiestos,
   DAWproject y serialización.
3. **Web Adapter**: futura integración experimental con el SDK web existente o una
   implementación WASM propia.

El código externo sólo podrá incorporarse después de verificar licencia,
atribuciones, dependencia técnica y coste de mantener una bifurcación.
