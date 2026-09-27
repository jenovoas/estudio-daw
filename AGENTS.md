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

## Idioma de la documentación

Redactar y mantener en español todo el texto explicativo, los requisitos, las
tareas y los títulos descriptivos. Traducir la prosa inglesa existente cuando se
edite un documento. Se conservan nombres oficiales, identificadores de código,
comandos ejecutables y las etiquetas estructurales que exijan los analizadores
de formatos como OpenSpec; no se usan como excusa para añadir prosa en inglés.

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
Para explorar estructuralmente el código, usa la habilidad instalada `codebase-memory`.
<!-- codebase-memory-mcp:end -->

## Estado vigente — loop A/B de transporte — 2026-09-27

- El loop A/B persistido ya se conecta al playback Tauri: al iniciar Play con
  rango vigente, el plan limita audio en B y un coordinador prepara/publica el
  salto a A junto con su agenda MIDI, sin cerrar PipeWire. La primera vuelta
  adicional se prepara antes de abrir el stream; las siguientes se preparan
  fuera del callback. Errores del coordinador aparecen al consultar posición.
- Cambiar el rango con el transporte ya iniciado no modifica el coordinador de
  esa sesión; detener e iniciar aplica el rango nuevo. No afirmar QA acústica ni
  sincronía sample-accurate: el scheduler MIDI conserva su reloj de pared.
- Verificación de este corte: `cargo check -p estudio-daw-ui-shell`,
  `cargo fmt --all -- --check`, `git diff --check` y validación OpenSpec normal
  (válida con ocho avisos lingüísticos conocidos). No se ejecutaron pruebas.
  OpenSpec sigue en 9/35; 2.3 y 4.3 permanecen parciales hasta completar QA,
  rebuild al editar y los controladores/funciones aún pendientes.
- SHA inicial: `33836f5`; consultar `git log` para SHA final/publicación. Vault:
  registrar el avance append-only en `BITACORA_AGENTES.md` y actualizar
  `ESTADO_ACTUAL.md`.

## Avance posterior — búsqueda de transporte — 2026-09-27

- Un clic en la regla durante Play recompila el plan y lo publica al siguiente
  límite de bloque sin cerrar el stream PipeWire; la UI refleja la posición y
  el scheduler sustituye la agenda MIDI. Al buscar, audio salta al offset de la
  región y MIDI reconstruye las notas activas y CC64 anteriores al cursor.
- `workstation-arrangement-surface-v2` sigue en 9/35; 2.3/4.3 continúan
  parciales. Pendientes: bucle, cambios de proyecto en caliente, otros
  controladores MIDI, scheduler ligado al reloj del plan y QA visual/auditiva.
- Validación del corte: compilación del crate UI Shell, formato, sintaxis JS,
  diff check y validación OpenSpec pasaron; sin pruebas ni QA manual.
- Consultar el handoff y `git log` para los SHA publicados. Los artefactos
  generados de `.codebase-memory/` se restauran antes de versionar.
- El proyecto ahora persiste opcionalmente un `TransportLoopRange` a 960 PPQ.
  A/B se define desde el cursor y se guarda con un comando reversible; aún no
  acciona repetición. El siguiente corte debe coordinar el salto de fuentes de
  audio y agenda MIDI con un plan preparado fuera del callback.

## Continuación activa — 2.2 (2026-09-27)

El commit publicado `b375009` incluye el primer corte de 2.2 (sobre `f1fdf43`)
y dejó 2.1 cerrada. La implementación parcial
2.2 importa archivos mediante Tauri con copia/vínculo, metadatos `ffprobe`,
ubicación inicial por compás, fuente+región en una transacción y waveform min/max
real (`ffmpeg`, hasta 10 minutos) y preescucha de hasta 30 segundos en regiones
importadas. La tarea permanece abierta: faltan preescucha desde selector,
selección/mapeo de canales, edición visual de región/cursor y QA visual en
ejecución. Ver `docs/audio-import.md`, `docs/HANDOFF_CONTINUACION_2026-09-27.md`
y el avance no marcado como completo en `tasks.md`.

## Corrección de auditoría — 2026-09-27

- HEAD publicado al iniciar la corrección: `31ef702`; implementación local:
  `9d3eb46` (`fix: align Tauri transport and arrangement behavior`).
- En una anotación inicial se declaró completa `soundfont-instrument-rendering`
  4.3 por una medida de 10–14 ms hasta el monitor digital pre-DAC. La auditoría
  posterior estableció que esa evidencia no mide la ruta física tecla→parlante;
  OpenSpec vuelve a dejar 4.3 abierta. La confirmación auditiva del piano se
  conserva como evidencia de escucha, no como medición de latencia.
- Tauri sí enlaza Play/Pause/Stop con PipeWire. Pause ya no procesa el plan y
  por tanto no consume el ring PCM ni avanza nodos síncronos; eventos fuera de
  `duration_ticks` no se programan. Hay regresiones para pausa, límite del clip,
  nota MIDI fuera de rango y slot vacío.
- La shell tiene arreglo musical como superficie principal, con grilla y clips
  de color inspirados en Ableton Live 12 sin activos de terceros. Sigue sin
  edición de notas/arreglo ni grabación live.
- Verificación sobre `9d3eb46`: `cargo fmt --all`,
  `cargo test --workspace -- --test-threads=1`, `cargo check --workspace`,
  `openspec validate runtime-transport-correctness --strict` y
  `git diff --check` pasaron.
- Auditoría aún abierta para un cambio separado: límites de dependencias del
  Portable Domain/ALSA/ffmpeg, reloj/posición musical Tauri y recompilación del
  plan tras cambios de proyecto. La semántica de RenderPlan quedó corregida en
  `1126436`; este punto ya no está pendiente.

## Corrección del contrato RenderPlan — 2026-09-27

- `1126436` elimina la API `connect` y el orden topológico que no correspondían
  con los buffers in-place reales. `RenderPlan` ahora documenta y ejecuta una
  cadena serial de nodos en orden de inserción; el sumado paralelo queda en
  `InstrumentMixerNode` con scratch preasignado.
- OpenSpec `render-plan-chain-contract` está completa (6/6). Workspace test y
  check, fmt, `openspec validate --strict` y diff check pasaron.
- Continúa pendiente una propuesta OpenSpec separada para dependencias del
  Portable Domain y el reloj/recompilación de transporte Tauri.

## Pause FluidSynth y revisión OpenSpec — 2026-09-27

- El seguimiento `268cb9a` conecta la bandera Pause del transporte al worker
  SoundFont: el worker sigue atendiendo control MIDI/Stop, pero no genera PCM ni
  avanza voces mientras está pausado. La cola queda estacionaria; la prueba
  focal pasó con FluidR3 instalado.
- `runtime-transport-correctness` queda completa (12/12); la suite workspace,
  check, fmt, validación OpenSpec estricta de ambos cambios y diff check pasan.
- `render-plan-chain-contract` queda completa (6/6): RenderPlan es cadena
  serial in-place y mezcla paralela explícita. Los dos commits locales desde
  `b3a6fad` son `1126436` y `268cb9a`; el commit `54ae78e` sincroniza sus
  requisitos a `openspec/specs/{audio-render-chain,desktop-workstation-ui,
  runtime-midi-playback}/`. Consultar `git status/log` para la sincronización
  publicada más reciente.
- La bitácora append-only fue actualizada. Restan Portable Domain y reloj/
  posición/rebuild de Tauri, que requieren su propio diseño OpenSpec.

## Handoff de auditoría portable y documentación en español — 2026-09-27

- La anotación anterior que declaraba `soundfont-instrument-rendering` 4.3
  completa quedó corregida: `tasks.md` vuelve a dejarla `[ ]`. La medición de
  10–14 ms corresponde al monitor digital previo al DAC, no a la latencia física
  tecla→parlante. El usuario sí confirmó la escucha del piano; falta una prueba
  de retorno físico que permita sostener la medición que exige el handoff.
- OpenSpec `workstation-arrangement-surface-v2`, tarea 1.4, ya está `[x]` con
  evidencia de dependencias y pruebas. `project-model` usa `midi-types` puro;
  `command-bus` no arrastra ALSA; inspección/FFmpeg/FFprobe viven en
  `media-adapter`; la asociación de proxies usa un comando reversible con
  validación de procedencia. DAWproject se convierte en memoria desde/hacia
  bytes; el acceso a rutas queda en la CLI. Tauri consulta instantáneas y emite
  comandos.
- Verificación de esta intervención: `cargo fmt --all -- --check`,
  `cargo test --workspace -- --test-threads=1`, `cargo check --workspace`,
  `git diff --check` y las seis validaciones OpenSpec pertinentes pasaron. Los
  árboles de dependencias de `project-model` y `command-bus` no incluyen ALSA
  ni PipeWire. La validación de `workstation-arrangement-surface-v2` presenta
  ocho recomendaciones lingüísticas; se conservan los requisitos normativos en
  español.
- Los párrafos explicativos de OpenSpec y los metadatos de crates editados se
  tradujeron al español. Se conservan únicamente identificadores técnicos y las
  etiquetas obligatorias del formato OpenSpec (`Purpose`, `Requirements`,
  `Requirement`, `Scenario`, `MUST`, `MAY`, `SHALL`), que su analizador requiere.
- Próximo trabajo del arreglo: 0.2 sigue abierta por requerir cotejo en la
  documentación del fabricante opción por opción; no requiere ejecutar Live en
  Linux. 1.4 está completa. Continuar con las tareas independientes según sus
  dependencias, sin presentar pendientes como hechos.

## Handoff para reinicio — 2026-09-27

### Avance posterior — reproducción de audio 2.3

- HEAD publicado al iniciar el corte: `183e938`. Hay implementación local pendiente de publicar: decodificación `ffmpeg` incremental en workers, rings PCM acotados, mezcla de regiones con MIDI y aplicación de offset, duración, canales, ganancia y fades.
- Verificación: `cargo check -p estudio-daw-ui-shell`, `cargo fmt --all -- --check`, `node --check crates/ui-shell/frontend/main.js`, `git diff --check` y `openspec validate workstation-arrangement-surface-v2` pasaron. No se ejecutaron pruebas ni QA acústica. OpenSpec 2.3 permanece abierta hasta cubrir loop, búsqueda/cursor compartido, actualización del plan en caliente y QA.
- Avance publicado de 4.3: la posición se deriva de `TransportClock` dentro del plan PipeWire; la UI presenta lectura y cabezal en Arreglo. La pausa congela el valor y Stop lo reinicia. Play desde detenido ahora acepta el cursor como origen; loop y seek durante reproducción siguen abiertos.
- Verificación local de 4.3: `cargo check -p estudio-daw-ui-shell`, fmt check, `node --check` de ambos scripts Tauri, diff check y validación OpenSpec pasan; no se ejecutó suite ni QA en hardware.
- Avance local adicional: Play desde detenido usa el cursor de Arreglo (480→960 ticks/negra); el decodificador salta al offset de fuente y el scheduler restaura NoteOn aún activos por clip. La restauración de controladores MIDI y la búsqueda durante reproducción siguen pendientes.
- 2.2 permanece abierta por QA visual de importación/edición. Continuar desde el plan aprobado; no detener el trabajo sólo porque una verificación humana quede pendiente.

Leer `docs/HANDOFF_CONTINUACION_2026-09-27.md` antes de continuar. HEAD
funcional base: `e66b5fe8fe2e71b6d8be399a717f4a338de0f4f2`; consultar `git log`
para el commit documental que guarda este handoff.
OpenSpec `workstation-arrangement-surface-v2` sigue en 7/35. Siguiente: cerrar
el cotejo documental oficial de Live 12.4.6 (0.2) sin exigir ejecutar Live en
Linux y continuar con el flujo vertical de pista de audio (2.1). Mantener la
documentación en español, la bitácora de bóveda append-only y todos los límites
de evidencia descritos en el handoff.

## Actualización de continuación — 2026-09-27

- OpenSpec `workstation-arrangement-surface-v2` avanzó a 8/35: tarea 0.2 cerrada mediante cotejo documental del manual y guías oficiales de Live 12.4.6 y una matriz de trazabilidad en `design.md`. No implica paridad ni implementación.
- Siguiente: tarea 2.1, flujo vertical de pista de audio. Consultar el handoff actualizado y comprobar código/dependencias antes de editar. La bitácora de la bóveda queda append-only.

## Estado de continuación — 2026-09-27

- `workstation-arrangement-surface-v2` está en 9/35. La tarea 2.1 añade ruteo interno al Master, instantánea de pistas con canales/destino y superficies de sólo lectura en Session y Mezclador; no implica ruteo físico ni reproducción de audio.
- Siguiente: tarea 2.2, importación y procedencia de medios. Revisar el handoff actualizado. La bitácora externa sigue append-only.

## Estado de continuación — edición de regiones 2.2

- HEAD publicado inicial de este corte: `57f0e5f` (`feat: previsualiza e importa canales de audio`). 2.2 sigue abierta y el cambio en curso conecta mover, recortar hacia dentro y quitar regiones desde el Arreglo, preservando la fuente y usando comandos reversibles.
- Verificación de código: `cargo check --workspace`, `cargo fmt --all -- --check`, `node --check crates/ui-shell/frontend/main.js` y `git diff --check`. No se ejecutó la suite ni se hizo QA visual/auditiva en la aplicación.
- El cursor de inserción local de Arreglo ya se coloca con clic en la regla o en un espacio vacío de pista; la importación usa su posición, y el campo de compás lo coloca al inicio de ese compás. Pendientes de 2.2: QA en ejecución y cotejo auditivo. El cursor aún no se comparte con Session/transporte; la selección de canales no implica ruteo de audio. Ver `docs/HANDOFF_CONTINUACION_2026-09-27.md` y `tasks.md`.
