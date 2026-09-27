# Handoff de continuación — 2026-09-27

## Punto exacto de reanudación

- Repositorio: `/home/jnovoas/proyectos/estudio-daw`.
- Rama: `main`; HEAD funcional de partida `e66b5fe8fe2e71b6d8be399a717f4a338de0f4f2`, publicado en `origin/main`. Este archivo y el puntero de `AGENTS.md` se guardan en un commit documental posterior; al reanudar, confirmar el HEAD exacto con `git log`.
- Árbol de trabajo limpio al guardar este handoff. Los cambios regenerados en `.codebase-memory/` se restauraron; no son trabajo del producto.
- Cambio OpenSpec activo: `workstation-arrangement-surface-v2`, **7/35 tareas completas**. Consultar `tasks.md`; no inferir progreso adicional por commits o estado del modelo.
- Bitácora de auditoría: `/home/jnovoas/proyectos/personalvault/docs/estudio-daw/BITACORA_AGENTES.md`. Estado: `/home/jnovoas/proyectos/personalvault/docs/estudio-daw/ESTADO_ACTUAL.md`.

## Siguiente trabajo

1. Continuar el cotejo documental de la tarea 0.2 usando exclusivamente el manual, las notas de versión y las guías oficiales de Ableton. Live no es compatible oficialmente con Linux; no pedir instalarlo ni bloquear el trabajo esperando una inspección de su ventana. El inventario sigue abierto hasta relacionar las opciones estáticas documentadas pertinentes con acciones, comandos, tareas o exclusiones justificadas. Marcar como condicional lo que dependa de dispositivo/selección y que el fabricante no enumere. No afirmar paridad.
2. Después avanzar la tarea 2.1 del flujo de pista de audio. La ventana ya tiene una acción Tauri para crear una pista vacía y mostrarla en el arreglo, pero no hay ruta de salida efectiva al Master ni superficies Sesión y Mezclador conectadas a una misma instantánea. No marcar 2.1 completa por la creación actual; revisar dependencias/código y entregar el flujo vertical real.

## Acuerdos vigentes

- Ableton Live es la referencia visual y de flujo creativo. Ardour sirve únicamente para conceptos técnicos de señal/medios; no usarlo como pauta visual.
- Documentación y comunicación del proyecto en español. Traducir nombres de vistas/menús cuando haya equivalente; conservar nombres propios, identificadores de código y sintaxis estructural que OpenSpec requiere.
- Mantener el flujo continuo y no pedir confirmación de decisiones ya aprobadas. No requerir al usuario que ejecute comprobaciones que el agente puede realizar.
- Honestidad estricta: requisitos OpenSpec contienen lo aprobado; el código y las pruebas prueban lo implementado; Git prueba el historial. No marcar módulos, tareas, menús o pruebas como completos por aproximación.
- Aún no existe primer lanzamiento del producto: no presentar marcadores del formato JSON como versiones del producto.

## Estado documental y validación

- La auditoría Live quedó ampliada en `design.md` con el manual oficial del 2026-04-30, notas hasta Live 12.4.6, guías oficiales en español, páginas de ajustes, menús/contextos y mapa de capacidades/tareas del proyecto.
- El commit `159e87e` quitó la ejecución de Live en Linux como requisito de auditoría. `e66b5fe` tradujo las etiquetas de vistas agregadas. La tarea 0.2 continúa abierta porque falta terminar el cotejo documental; no por falta de instalación.
- `openspec validate workstation-arrangement-surface-v2` pasó con ocho avisos de recomendación para los marcadores RFC ingleses en requisitos españoles. Se mantiene la redacción normativa española. `git diff --check` pasó. No se ejecutaron pruebas Rust en esta intervención documental.
- Antes de cualquier cambio de código, seguir el orden de lectura de `AGENTS.md`. La siguiente verificación de código debe abarcar al menos las pruebas focales y, cuando corresponda, formato, espacio de trabajo, validación OpenSpec y diff check; registrar comandos/resultados exactos.

## Pendientes que no deben perderse

- En `workstation-arrangement-surface-v2`: 0.2, todas las tareas 2.x–6.x siguen abiertas salvo lo que `tasks.md` marque `[x]`; las tareas de Session/Arreglo/Mezclador, transporte, reproducción de audio, menús y revisión visual no se consideran implícitas.
- En `soundfont-instrument-rendering`: la tarea 4.3 permanece abierta para la comprobación física de latencia que requiere la especificación. La escucha confirmada y la medición digital pre-DAC no prueban latencia tecla→parlante.
- La bitácora de la bóveda es append-only. Se añadieron entradas durante esta sesión; los archivos de la bóveda siguen como cambios locales por guardar, no se publicaron en remoto.
- El índice de código estaba fechado `2026-09-26T21:19Z` al iniciar esta sesión y se señaló como atrasado respecto a código/`git`. Actualizarlo sólo si hace falta y restaurar sus artefactos versionados generados al terminar. La evidencia de comportamiento es código, pruebas y Git.

## Secuencia al reanudar

1. Ejecutar `git status --short --branch` y `git log -5 --oneline`.
2. Leer `README.md`, `AGENTS.md`, propuesta/diseño/specs/tareas del cambio activo y `docs/ui-architecture.md` según la guía.
3. Leer este handoff y las entradas finales de la bitácora para confirmar que no hubo cambios posteriores.
4. Seguir con 0.2 mediante fuentes oficiales; pasar a 2.1 cuando se cierre ese cotejo o haya que alternar con trabajo de código independiente.
