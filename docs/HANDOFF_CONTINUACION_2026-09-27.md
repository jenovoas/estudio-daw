# Handoff de continuación — 2026-09-27

## Actualización — búsqueda de transporte durante Play

- Continuación publicada: consultar `git log`; la búsqueda se aplica mediante un plan nuevo en el siguiente límite de bloque y conserva abierto PipeWire. Un clic en la regla reubica audio y MIDI; el scheduler sustituye la agenda y las regiones se decodifican desde la nueva posición.
- Verificación del corte: `cargo fmt --all`, `cargo check -p estudio-daw-ui-shell`, `cargo fmt --all -- --check`, `node --check` de `main.js` y `platform-tauri.js`, `git diff --check` y `openspec validate workstation-arrangement-surface-v2` pasan; ocho avisos lingüísticos conocidos. No se ejecutaron pruebas ni QA auditiva/visual.
- `workstation-arrangement-surface-v2` sigue en 9/35; 2.3 y 4.3 continúan abiertas. Pendientes: loop, rebuild al editar, otros controladores MIDI, alinear el scheduler al reloj del plan y QA. La búsqueda se ofrece sólo mientras está en Play; pausado se debe reanudar primero.
- No se modificó el dispositivo ni el callback para preparar planes. El control publica el plan desde Tauri y espera la recuperación del anterior fuera de RT; el scheduler usa todavía reloj de pared para los tiempos de eventos. Continuar con esa limitación documentada.

## Reanudación — reproducción incremental de regiones (OpenSpec 2.3)

- HEAD publicado tras los cortes 2.3/4.3: `c534142` (`feat: muestra posición real del transporte`). La búsqueda inicial desde el cursor de Arreglo está en curso; consultar `git status/log` al reanudar.
- `AudioPcmDecoder` usa `ffmpeg` para transmitir PCM f32 estéreo. Un worker por región llena un ring PCM SPSC de un segundo, limitado a 64 regiones y precargado antes de abrir PipeWire. `AudioClipMixerNode` mezcla en el plan ya compilado con MIDI y aplica posición inicial, desplazamiento/duración, canales, ganancia y fades. El callback sólo consume el ring y mezcla en scratch preasignado.
- Pause congela el plan; Stop libera los workers/cancela `ffmpeg`. Cambiar regiones mientras corre guarda el proyecto pero no reconstruye el plan activo; detener e iniciar carga los cambios. Al iniciar desde detenido se puede reproducir desde el cursor de Arreglo; todavía no existe búsqueda en caliente ni bucle.
- `workstation-arrangement-surface-v2` permanece en 9/35; 2.2 sigue abierta por QA visual y 2.3 sigue abierta por las capacidades de transporte ausentes y QA acústica. No marcar ninguna completa por esta implementación parcial.
- Avance publicado de 4.3: `TransportPositionNode` publica la posición musical calculada por `TransportClock` a partir de los frames procesados por PipeWire; la UI dibuja y muestra el cabezal. Play desde detenido usa el cursor. Sigue pendiente búsqueda durante reproducción y loop; la implementación no usa reloj de pared para mover el cabezal.
- Al iniciar desde el cursor, el scheduler restaura NoteOn aún activos por clip antes de abrir el flujo. Los controladores previos (incluido sustain) no se reconstruyen todavía.
- Verificación del corte local: `cargo check -p estudio-daw-ui-shell`, `cargo fmt --all -- --check`, `node --check` para `main.js` y `platform-tauri.js`, `git diff --check` y validación OpenSpec pasan; no se ejecutaron pruebas ni QA física.
- Próximo paso aprobado: terminar la búsqueda inicial desde el cursor y continuar con loop/seek en ejecución y actualización del plan según 2.3/4.3; después abrir 2.4/2.5 según OpenSpec. Mantener el callback libre de asignación, bloqueo e I/O.

## Contexto histórico — edición de regiones de audio 2.2

- HEAD de partida: `57f0e5f` (`feat: previsualiza e importa canales de audio`), publicado en `main`; esta continuación añade interacción de mover/recortar/quitar región.
- Progreso OpenSpec: 9/35. 2.2 sigue abierta. Importación, copia/vínculo, metadatos, preescucha previa, elección mono/estéreo, waveform y ubicación por compás están implementados. Los gestos de edición usan comandos reversibles; quitar una región conserva el registro y los bytes de la fuente.
- Límites: recorte sólo hacia dentro; no se puede reextender con ratón. Ahora hay cursor de inserción local en Arreglo, pero falta QA visual/auditiva de la app en ejecución. La selección de canal aún no dirige la reproducción. No marcar 2.2 completa.
- Documento específico: `docs/audio-import.md`. Esta continuación pasó `cargo check --workspace`, fmt check, sintaxis de JS y diff check; no se ejecutó la suite de pruebas ni QA visual/auditiva.
- QA visual del flujo de importación/cursor/edición sigue pendiente, pero no bloquea el trabajo de implementación aprobado en 2.3. No volver a alterar los artefactos generados de `.codebase-memory/` en commits de producto.

## Punto exacto de reanudación

- Repositorio: `/home/jnovoas/proyectos/estudio-daw`.
- Rama: `main`; HEAD funcional de partida `e66b5fe8fe2e71b6d8be399a717f4a338de0f4f2`, publicado en `origin/main`. Este archivo y el puntero de `AGENTS.md` se guardan en un commit documental posterior; al reanudar, confirmar el HEAD exacto con `git log`.
- Árbol de trabajo limpio al guardar este handoff. Los cambios regenerados en `.codebase-memory/` se restauraron; no son trabajo del producto.
- Cambio OpenSpec activo: `workstation-arrangement-surface-v2`, **9/35 tareas completas**. Las tareas 0.2 y 2.1 se cerraron; consultar `tasks.md` y `design.md`.
- Bitácora de auditoría: `/home/jnovoas/proyectos/personalvault/docs/estudio-daw/BITACORA_AGENTES.md`. Estado: `/home/jnovoas/proyectos/personalvault/docs/estudio-daw/ESTADO_ACTUAL.md`.

## Siguiente trabajo

1. La tarea 2.2 tiene importación, cursor local de inserción y edición reversible; sigue pendiente QA visual. La implementación parcial de 2.3 decodifica y mezcla regiones con MIDI; continuar con loop, búsqueda/posición compartida y actualización del plan activo, según el alcance OpenSpec. 2.1 persiste la ruta interna al Master; el ruteo físico sigue pendiente.

## Acuerdos vigentes

- Ableton Live es la referencia visual y de flujo creativo. Ardour sirve únicamente para conceptos técnicos de señal/medios; no usarlo como pauta visual.
- Documentación y comunicación del proyecto en español. Traducir nombres de vistas/menús cuando haya equivalente; conservar nombres propios, identificadores de código y sintaxis estructural que OpenSpec requiere.
- Mantener el flujo continuo y no pedir confirmación de decisiones ya aprobadas. No requerir al usuario que ejecute comprobaciones que el agente puede realizar.
- Honestidad estricta: requisitos OpenSpec contienen lo aprobado; el código y las pruebas prueban lo implementado; Git prueba el historial. No marcar módulos, tareas, menús o pruebas como completos por aproximación.
- Aún no existe primer lanzamiento del producto: no presentar marcadores del formato JSON como versiones del producto.

## Estado documental y validación

- La auditoría Live quedó ampliada en `design.md` con el manual oficial del 2026-04-30, notas hasta Live 12.4.6, guías oficiales en español, páginas de ajustes, menús/contextos y matriz de trazabilidad. 0.2 cerrada documentalmente; sin afirmar paridad.
- El commit `159e87e` quitó la ejecución de Live en Linux como requisito de auditoría. `e66b5fe` tradujo las etiquetas de vistas agregadas. La actualización actual añadió una matriz de trazabilidad documental y cerró 0.2; no afirma paridad ni implementación.
- `openspec validate workstation-arrangement-surface-v2` pasó con ocho avisos de recomendación para los marcadores RFC ingleses en requisitos españoles. Se mantiene la redacción normativa española. `git diff --check` pasó. No se ejecutaron pruebas Rust en esta intervención documental.
- Antes de cualquier cambio de código, seguir el orden de lectura de `AGENTS.md`. La siguiente verificación de código debe abarcar al menos las pruebas focales y, cuando corresponda, formato, espacio de trabajo, validación OpenSpec y diff check; registrar comandos/resultados exactos.

## Pendientes que no deben perderse

- En `workstation-arrangement-surface-v2`, 2.2–6.4 siguen abiertas salvo lo que `tasks.md` marque `[x]`; la vista Session sólo expone encabezados/casillas vacías y el Mezclador es de sólo lectura. Lanzamiento, hardware, mezcla activa, menús y revisión visual no se consideran implícitos.
- En `soundfont-instrument-rendering`: la tarea 4.3 permanece abierta para la comprobación física de latencia que requiere la especificación. La escucha confirmada y la medición digital pre-DAC no prueban latencia tecla→parlante.
- La bitácora de la bóveda es append-only. Se añadieron entradas durante esta sesión; los archivos de la bóveda siguen como cambios locales por guardar, no se publicaron en remoto.
- El índice de código estaba fechado `2026-09-26T21:19Z` al iniciar esta sesión y se señaló como atrasado respecto a código/`git`. Actualizarlo sólo si hace falta y restaurar sus artefactos versionados generados al terminar. La evidencia de comportamiento es código, pruebas y Git.

## Secuencia al reanudar

1. Ejecutar `git status --short --branch` y `git log -5 --oneline`.
2. Leer `README.md`, `AGENTS.md`, propuesta/diseño/specs/tareas del cambio activo y `docs/ui-architecture.md` según la guía.
3. Leer este handoff y las entradas finales de la bitácora para confirmar que no hubo cambios posteriores.
4. Seguir con 2.2 tras leer el contrato de medios y revisar los adaptadores disponibles.

## Actualización — cierre de 2.1

- HEAD sigue en `0a615f83883337e2b459056b1b33b529b3f5f11e`; implementación/documentación local sin commit.
- `AddTrack` crea pista de audio estéreo, la conecta por ID al Master y crea ese canal en la misma transacción cuando falta. Session y Mezclador presentan el mismo resumen de la instantánea que Arreglo; Session muestra encabezados/casillas vacías y el Mezclador es de sólo lectura.
- Pasaron `cargo fmt --all`, `cargo test --workspace -- --test-threads=1`, `cargo check --workspace`, `cargo fmt --all -- --check`, `git diff --check` y `openspec validate workstation-arrangement-surface-v2`. La validación conserva ocho avisos lingüísticos sobre `MUST/SHALL`; no son fallos estructurales.
- Progreso: 9/35. Seguir con 2.3, que está parcialmente implementada. No está disponible el ruteo a hardware, edición de Session ni controles activos de mezcla.
