# Guía de agentes — Estudio DAW

Este archivo define cómo debe trabajar cualquier agente de IA en este repositorio.
No reemplaza el README, OpenSpec ni los documentos de arquitectura: los enlaza y
protege como fuentes canónicas.

## Lectura obligatoria antes de modificar código

Leer, en este orden:

1. `README.md`, para comprender visión, alcance, hardware y estado del producto.
2. `openspec/changes/bootstrap-opendaw-architecture/proposal.md`.
3. `openspec/changes/bootstrap-opendaw-architecture/design.md`.
4. Las specs relevantes en
   `openspec/changes/bootstrap-opendaw-architecture/specs/`.
5. `openspec/changes/bootstrap-opendaw-architecture/tasks.md`.
6. El documento específico de `docs/` relacionado con el cambio.
7. `git status`, `git log` y el código existente antes de proponer estructuras
   nuevas.

No se debe reescribir, resumir de nuevo ni sustituir planificación existente para
justificar una implementación. Las mejoras de arquitectura se presentan como una
propuesta o delta explícito, conservando el material previo y su contexto.

## Fuentes de verdad y trazabilidad

- OpenSpec contiene requisitos, decisiones y tareas aprobadas.
- `README.md` contiene la visión consolidada y el mapa de capacidades.
- `docs/` explica contratos ya implementados o decisiones técnicas específicas.
- El código y las pruebas muestran el estado ejecutable.
- Git es la evidencia histórica: no declarar una función como implementada sin
  archivo, prueba y SHA que la respalden.
- La memoria auditable del proyecto vive en
  `/home/jnovoas/proyectos/personalvault/docs/estudio-daw/`.

Después de cada intervención material, añadir una entrada a
`BITACORA_AGENTES.md` en esa carpeta. La entrada debe incluir fecha, agente,
objetivo, SHA inicial/final, archivos modificados, pruebas ejecutadas, resultado y
pendientes. La bitácora es append-only: corregir mediante una entrada posterior,
no borrando el historial.

## Límites arquitectónicos

- Mantener el monolito modular y las fronteras Portable Domain, Native Core,
  Platform, Compute Runtime, Analysis Workers y futuro Web Adapter.
- El Portable Domain no depende de PipeWire, ALSA, GPU, GUI, Python ni LLM.
- UI, CLI, MIDI, scripting y agentes emiten comandos; no mutan directamente el
  modelo.
- El callback de audio no asigna, no bloquea, no hace I/O y no ejecuta modelos.
- Compilar el grafo DSP fuera del callback y ejecutar allí sólo el plan lineal
  precomputado.
- Mantener edición no destructiva, procedencia de medios, preview y undo/redo.
- La IA propone `ChangeSet`; el usuario conserva preview, aceptación y rollback.
- CPU es la ruta segura para tiempo real. GPU se usa mediante jobs y fallback
  medido, nunca por entusiasmo arquitectónico.
- No incorporar código de proyectos externos sin auditoría de licencia y
  procedencia.

## Implementación y compatibilidad

- Rust es el núcleo; Python queda aislado en workers de análisis.
- Linux, PipeWire/JACK, ALSA MIDI, KeyLab Essential 49 y AudioBox USB 96 son
  ciudadanos de primera clase.
- Conservar formatos versionados y contratos serializables.
- Añadir comentarios donde expliquen invariantes, ownership, tiempo real,
  unidades musicales o decisiones no obvias. No comentar sintaxis evidente.
- La CLI actual es un adaptador temporal de validación. Las capacidades deben
  diseñarse para ser consumidas por UI sin acoplar la UI a hardware o procesos.
- No duplicar buses, modelos o documentación. Si dos componentes parecen
  solaparse, definir primero sus responsabilidades y una ruta de migración.
- Consultar `docs/domain-command-bus.md` antes de ampliar o migrar comandos de
  proyecto; describe el contrato ejecutable y sus límites actuales.

## Flujo de trabajo

1. Inspeccionar el árbol y los cambios existentes; preservar trabajo ajeno.
2. Seleccionar la tarea OpenSpec vigente y comprobar sus dependencias.
3. Implementar el corte vertical más pequeño que respete la arquitectura.
4. Añadir pruebas deterministas y documentación de código proporcional al riesgo.
5. Ejecutar `cargo fmt --all`, pruebas del crate, `cargo test --workspace` y
   `git diff --check` cuando corresponda.
6. Revisar el diff completo antes de versionar.
7. Actualizar la bitácora de la vault con hechos verificables y SHA.
8. Crear commits pequeños y publicar en `main` cuando el usuario haya pedido
   versionar o el flujo vigente ya lo autorice.

No pedir al usuario que ejecute pruebas que el agente puede ejecutar. Las pruebas
que requieren interpretación humana, tocar un instrumento o evaluar audio se
solicitan sólo cuando no existe una comprobación automática equivalente.

## Comunicación

Trabajar de forma continua y evitar confirmaciones repetitivas. Interrumpir al
usuario únicamente por un bloqueo real, una decisión que cambie el alcance o una
acción destructiva/externa que requiera autorización. Informar resultados,
riesgos y decisiones concretas; no repetir acuerdos ya asentados.

## Handoff persistente — 2026-09-26

- Proyecto: `/home/jnovoas/proyectos/estudio-daw`, rama `main`; último cambio
  publicado: `a6dbb23` (`feat: report SoundFont PCM queue latency`). Repositorio
  limpio al guardar este handoff.
- Vault auditable: `/home/jnovoas/proyectos/personalvault/docs/estudio-daw/`;
  último registro publicado en `dff48d6`. Mantener `BITACORA_AGENTES.md`
  append-only y actualizar `ESTADO_ACTUAL.md` cuando el estado cambie.
- Trabajo reciente: `SoundFontInstrumentWorker::pcm_queue_metrics()` expone
  frames actuales/pico/capacidad del ring PCM. `midi-synth-live` y
  `midi-synth-play` imprimen también duración equivalente de la cola y periodo
  PipeWire; no llamar a esos valores latencia extremo a extremo.
- Validación actual: `cargo test --workspace -- --test-threads=1`,
  `cargo check --workspace`, `cargo fmt --all -- --check`, `git diff --check` y
  `openspec validate soundfont-instrument-rendering --strict` pasaron. En
  hardware local: replay SoundFont 37.120 callbacks, pico PCM 2048 frames =
  42,67 ms a 48 kHz, cero drops/underruns/errores; live idle 3 s 8.960
  callbacks, cero eventos, drops/underruns/errores.
- Único pendiente explícito de esa tarea: OpenSpec
  `soundfont-instrument-rendering`, tarea 4.3 sigue `[ ]` hasta verificar notas
  intencionales del KeyLab en la ruta SF2 y medir latencia física tecla→AudioBox
  con loopback. No pedir al usuario pruebas repetidas; sólo solicitar esta
  intervención física si todavía es necesaria y no puede automatizarse.
- Próxima sesión: leer `README.md`, este `AGENTS.md`,
  `openspec/changes/soundfont-instrument-rendering/{proposal.md,design.md,
  tasks.md}` y `docs/native-instrument.md`; revisar `git status/log`; después
  continuar exactamente desde 4.3, sin reabrir decisiones ya tomadas ni marcarla
  completada con medidas sólo del ring. Mantener documentación y bitácora.
- Preferencia de colaboración del usuario: avanzar en flujo continuo, sin pedir
  confirmación para cada paso y ejecutar las pruebas que el agente puede hacer.

## Handoff para Neovim — 2026-09-26

- Estado guardado en el commit que acompaña este handoff (consultar `git log`);
  rama `main`, sin push solicitado. Revisar `git status` al abrir el proyecto.
- Se añadió captura WAV sincronizada con el origen monotónico MIDI en
  `midi-synth-live` y `midi-synth-play`, más el reporte del primer callback de
  captura. La comprobación de compilación/fmt/diff pasó. No se afirma una
  latencia tecla→sonido: los offsets observados son de inicio de captura y los
  análisis de onset acústico fueron demasiado variables.
- AudioBox USB 96: Input 1 es micrófono; Input 2 es guitarra. No reasignarlos.
  El sink de salida también expone su propio monitor virtual PipeWire
  (`alsa_output.…analog-stereo.monitor`). Al intentar capturarlo directamente,
  PipeWire llamó al callback casi muestra por muestra y FluidSynth acumuló
  millones de underruns. Se retiró ese experimento; no reintroducirlo sin
  resolver primero la planificación/quantum y desacoplar la captura.
- OpenSpec `soundfont-instrument-rendering`, tarea 4.3 continúa `[ ]`. Pendiente:
  estabilizar una medición KeyLab→monitor PipeWire sin alterar el playback y
  obtener confirmación auditiva del usuario; no volver a pedir pruebas de
  secuencia de teclas ya realizadas salvo que haga falta para esa medición.
- Para continuar: leer README, este archivo, proposal/design/specs/tasks de
  `soundfont-instrument-rendering` y `docs/native-instrument.md`; comprobar
  `git status/log`; continuar desde 4.3. El usuario cambia el flujo de edición
  a Neovim porque VS Code se vuelve pesado.

<!-- codebase-memory-mcp:start -->
For structural codebase exploration, use the installed `codebase-memory` skill.
<!-- codebase-memory-mcp:end -->
