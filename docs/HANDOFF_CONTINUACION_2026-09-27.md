# Handoff de continuación — 2026-09-27

## Reanudación actualizada — flujo parcial de importación 2.2

- Publicación anterior: commit `f1fdf43` (`feat: crea pistas de audio con destino master`) en `main`, sincronizado con `origin/main`. El trabajo siguiente en 2.2 está en curso; consultar `git status` para el diff local y su SHA inicial/final en Git.
- Progreso OpenSpec: 9/35. 2.2 sigue abierta. Ya hay diálogo de selección de audio, opción de copiar a `media/` junto al proyecto o vincular, metadatos técnicos por `ffprobe`, posición por compás, una transacción reversible que registra fuente/región, waveform min/max vía `ffmpeg` (hasta 10 minutos y 512 bins solicitados por la UI), y preescucha Opus/WebAudio de hasta 30 segundos sobre regiones ya importadas, separada del transporte.
- Límites: falta preescuchar desde el selector antes de incorporar; la asignación es por cantidad compatible con la pista, sin mapeo/downmix; no hay cursor de edición ni controles visuales para mover/recortar/eliminar regiones; falta inspección visual de la app en ejecución. No marcar 2.2 completa.
- Documento específico: `docs/audio-import.md`. En el workspace pasaron la suite completa, check, fmt, sintaxis de JS, diff check y validación OpenSpec normal (ocho avisos normativos de idioma ya conocidos). Repetir luego de cualquier cambio antes de publicar.
- Siguiente: terminar 2.2 empezando por un diseño de preescucha acotada y fuera del transporte, luego mapeo de canales y edición visual de región; revisar seguridad/rendimiento del comando de waveform síncrono antes de pulir UI. No volver a alterar los artefactos generados de `.codebase-memory/` en commits de producto.

## Punto exacto de reanudación

- Repositorio: `/home/jnovoas/proyectos/estudio-daw`.
- Rama: `main`; HEAD funcional de partida `e66b5fe8fe2e71b6d8be399a717f4a338de0f4f2`, publicado en `origin/main`. Este archivo y el puntero de `AGENTS.md` se guardan en un commit documental posterior; al reanudar, confirmar el HEAD exacto con `git log`.
- Árbol de trabajo limpio al guardar este handoff. Los cambios regenerados en `.codebase-memory/` se restauraron; no son trabajo del producto.
- Cambio OpenSpec activo: `workstation-arrangement-surface-v2`, **9/35 tareas completas**. Las tareas 0.2 y 2.1 se cerraron; consultar `tasks.md` y `design.md`.
- Bitácora de auditoría: `/home/jnovoas/proyectos/personalvault/docs/estudio-daw/BITACORA_AGENTES.md`. Estado: `/home/jnovoas/proyectos/personalvault/docs/estudio-daw/ESTADO_ACTUAL.md`.

## Siguiente trabajo

1. Avanzar la tarea 2.2: la importación ya registra procedencia, permite copiar/vincular, preescucha previa, selección mono/estéreo persistida y ubicación inicial por compás. Siguen pendientes el cursor de edición, los controles visuales de región y el QA en ejecución. 2.1 ya persiste la ruta interna al Master y muestra pistas en Arreglo, Session y Mezclador desde una instantánea común; el ruteo físico y la reproducción siguen pendientes.

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

- En `workstation-arrangement-surface-v2`, 2.2–6.4 siguen abiertas salvo lo que `tasks.md` marque `[x]`; la vista Session sólo expone encabezados/casillas vacías y el Mezclador es de sólo lectura. Lanzamiento, edición, reproducción de audio, hardware, mezcla activa, menús y revisión visual no se consideran implícitos.
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
- Progreso: 9/35. Próxima tarea: 2.2. No está disponible el ruteo a hardware, reproducción de audio, edición de Session ni controles activos de mezcla.
