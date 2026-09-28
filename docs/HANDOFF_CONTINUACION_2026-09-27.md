# Handoff de continuación — 2026-09-27

## Corte local actual — ajustes de lanzamiento por casilla de Session (2026-09-28)

- `ClipSlot` persiste `ClipLaunchQuantization`: global (valor por defecto para proyectos existentes), ahora, 1/16, 1/8, negra o compás según la métrica del proyecto; también `ClipLaunchMode`: Repetir o Una pasada (los proyectos existentes conservan Repetir). Session deja editar ambos por casilla; cambiar el clip conserva esos ajustes. Una pasada programa una reproducción MIDI o una instancia decodificada del audio; Repetir amplía ventanas fuera del callback. El modo se aplica al próximo lanzamiento.
- Los ajustes se guardan mediante el comando reversible `SetClipSlot` y aparecen en la instantánea Tauri. El lanzamiento de escena usa un único comando/tick global y cada clip respeta su modo; las casillas vacías conservan su clip activo. No se cambió la propuesta ni se marcaron tareas completas.
- Verificación: `cargo check -p estudio-daw-ui-shell`, `cargo fmt --all -- --check`, `node --check` de `main.js` y `platform-tauri.js`, `git diff --check` y `openspec validate workstation-arrangement-surface-v2 --strict` pasaron. No se ejecutaron pruebas.
- OpenSpec sigue en 10/44; 3.2 y 4.4 siguen parciales. Pendientes: reglas de lanzamiento adicionales, semántica final de escena/selección y QA visual/funcional; falta revisar e integrar los hallazgos del análisis paralelo de Meta Muse.

## Punto de reanudación actual — loop Session hasta detener (2026-09-28)

- El host rellena ventanas MIDI (`ExtendTrack`) y audio (`Append` al mixer) fuera del callback cuando la posición se acerca al final de la ventana. El clip marcado `looping` sigue sonando hasta ■. El mixer aplica `start_frame` relativo al `frame_cursor` al recibir el comando, para poder lanzar audio a mitad de un plan. `cargo check -p estudio-daw-ui-shell` pasa.
- Los botones ▶ de casillas de audio están habilitados con Play/pausa (mismo gesto que MIDI).
- OpenSpec 4.4/3.2 siguen abiertas: cuantización por clip, modos Live y QA.

## Punto de reanudación actual — Session audio, rejilla y loop (2026-09-28)

- Session lanza MIDI y audio a la rejilla del transporte (1/16–compás o ahora). El clip se repite hasta ■: MIDI 64 compases, audio 8 vueltas predecodificadas. El audio de Session entra por un mixer de pista con comando try_lock; no se recrean workers VST3. Analog Lab 4.6 sigue pendiente de QA en Tauri.
- Verificación: `cargo check -p estudio-daw-ui-shell`, pruebas `session`, formato, ambos `node --check`. Sin suite ni QA visual/auditiva.
- OpenSpec 10/44. 4.4/3.2 avanzan y siguen abiertas.

## Punto de reanudación actual — lanzamiento Session MIDI (2026-09-28)

- Sobre `066bc28`, Session lanza clips MIDI al siguiente compás sin cerrar PipeWire. ▶ en casilla o escena requiere Play/pausa; ■ detiene esa pista. El clip usa su PPQ y apaga las voces de la pista al entrar. Audio de Session, modos de lanzamiento y loop de clip siguen pendientes. 4.6 permanece abierta por QA Analog Lab.
- Verificación: `cargo check -p estudio-daw-ui-shell`, pruebas `quantizes_session_launches_to_the_next_bar` y `session_midi_events_follow_clip_ppq_from_the_launch_tick`, formato, ambos `node --check`. Sin suite ni QA visual.
- OpenSpec 10/44. Continuar: QA Analog Lab o audio de Session / cuantización por clip.

## Punto de reanudación actual — estado VST3 y Session (2026-09-28)

- Sobre `c0c38d6`, Stop/Guardar capturan el blob del plugin activo, lo escriben en `plugin-state/<pista>.bin` junto al proyecto y persisten la referencia con `SetTrackInstrument`. Play restaura ese estado antes de procesar; un archivo ausente o con SHA distinto aborta la carga y conserva la asignación. CONTROLES aparece también en Session. Las vueltas A/B recargan el último estado en disco.
- Verificación: `cargo check -p estudio-daw-ui-shell`, prueba `writes_and_restores_plugin_state_relative_to_the_project`, `cargo fmt --all -- --check`, `node --check` de `main.js` y `git diff --check`. Sin suite workspace ni QA Analog Lab.
- OpenSpec 10/44. 4.6.4 y 4.6.6 avanzan y siguen abiertas. Siguiente: QA Tauri Play → CONTROLES → cambiar preset → Stop/abrir. Bitácora: `/home/jnovoas/proyectos/personalvault/docs/estudio-daw/BITACORA_AGENTES.md`.

## Punto de reanudación actual — editor VST3 Linux (2026-09-28)

- HEAD publicado al iniciar este corte: `e2c015d`. Hay implementación local del editor nativo VST3 en Linux: el helper, en una sesión Hyprland/Wayland, crea la ventana padre por XWayland (`DISPLAY`), adjunta `IPlugView` con `X11EmbedWindowID` y atiende `CreateGui`/`CloseGui` en el hilo gráfico. Tauri expone `set_vst3_editor`; Arreglo y Mezclador muestran un slot de dispositivo con CONTROLES/OCULTAR. Play o pausa cargan el helper; Stop lo termina y oculta el estado local de la ventana. Analog Lab/yabridge usan ese puente XWayland; no hay host `IWaylandHost` nativo todavía.
- `CreateGui`/`CloseGui` pasan al plazo lento de 30 s. Abrir la GUI bloquea el worker de esa pista hasta que el plugin responde; el audio de la pista puede interrumpirse unos segundos. El callback PipeWire no crea ventanas ni habla con el helper.
- Verificación de este corte: `cargo check -p vst3-host --bins --tests`, `cargo check -p estudio-daw-ui-shell`, `cargo test -p vst3-host --lib slow_commands_are_classified_apart_from_the_per_block_ones`, `cargo fmt --all -- --check`, ambos `node --check` y `git diff --check`. No se ejecutó la suite del workspace ni QA en Tauri con Analog Lab.
- OpenSpec `workstation-arrangement-surface-v2` sigue 10/44. 4.6.2 y 4.6.6 registran este avance y permanecen abiertas: falta QA visual/auditiva, estado binario del plugin y el mismo acceso desde Session.
- Continuar: probar en Tauri Play → CONTROLES con Analog Lab V, luego persistir estado del plugin (4.6.4). No marcar 4.6.2 completa sin esa QA. Bitácora de bóveda: `/home/jnovoas/proyectos/personalvault/docs/estudio-daw/BITACORA_AGENTES.md`.

## Punto de reanudación actual — UI Tauri (2026-09-27)

- HEAD funcional al guardar este handoff: consultar `git log` (piano roll MIDI con gestos). Arreglo incluye movimiento, duplicación, cuantización y división reversibles de clips MIDI. El inspector muestra un piano roll de 36 teclas para insertar y seleccionar notas; permite editar tono/posición/duración/velocidad, borrar, mover por arrastre y redimensionar desde el borde mediante comandos que conservan eventos ajenos, soportan undo/redo y refrescan el plan conectado. Los comandos rechazan referencias obsoletas a eventos.
- La especificación OpenSpec pasa ahora `openspec validate workstation-arrangement-surface-v2 --strict`; se añadieron los tokens normativos `MUST` junto al español DEBE/DEBEN para satisfacer al analizador. El cambio sigue 9/35 tareas completas porque los cortes UI grandes 3.x/5.x aún no están terminados.
- Verificación actual del piano roll: pruebas focales de insertar/editar/borrar/undo/redo y rechazo de referencia obsoleta; `cargo test --workspace -- --test-threads=1` (139 pruebas), `cargo check --workspace`, formato, ambos `node --check`, diff check y OpenSpec estricta pasaron. FluidSynth/ALSA emitió avisos de dispositivo no configurado durante sus pruebas; pasaron. No se hizo QA visual en Tauri.
- Parciales actuales: 3.2 matriz editable sin lanzamiento (depende de 4.4); 3.3 edición de audio y clips MIDI movibles/divisibles/duplicables/cuantizables, sin más marcadores ni QA; 3.5 piano roll básico de 36 teclas permite insertar/seleccionar/editar tono, posición, duración y velocidad/borrar, mover por arrastre y redimensionar desde el borde, pero faltan expresión, editor de forma de onda y dispositivo contextual; 3.6 búsqueda/preescucha de regiones de audio del proyecto sin bibliotecas/historial/favoritos/drag-drop; 4.1/4.2 esperan QA y capacidades avanzadas; 5.1 registro frontend parcial, no tipado en Rust, sin catálogo completo ni prueba de integridad; 5.4 quitar/duplicar/reordenar pistas conectado pero sin altura/visibilidad/listas de tomas; 5.7 zoom y atajos/contextuales iniciales, faltan reasignación, navegación teclado y QA.
- Continuar con las especificaciones UI no bloqueadas, completar menús soportados y paneles de edición/navegador. Mantener el scheduler 4.4 como dependencia explícita y no habilitar lanzamientos falsos. Ejecutar la suite y QA visual sólo al cierre, como indicó el usuario. Bitácora de bóveda: `/home/jnovoas/proyectos/personalvault/docs/estudio-daw/BITACORA_AGENTES.md`.

## Actualización — piano roll inicial (2026-09-27)

- Añadido `AddMidiNote` al bus de dominio: valida parámetros, preserva eventos existentes, inserta Note On/Off en ticks/tiempo derivado de PPQ y tempo, amplía duración del clip, soporta undo/redo y refresca el plan en Tauri. El piano roll muestra 36 teclas y notas existentes; un clic inserta una corchea con velocidad 96 y canal MIDI 1.
- OpenSpec 3.5/5.3 actualizadas como avance parcial; todavía faltan editar/borrar notas existentes, controles de propiedades, y QA visual manual.

## Continuación — edición de notas en piano roll

- Se añadieron `UpdateMidiNote` y `RemoveMidiNote`. La instantánea expone índices Note On/Off, canal y ticks; los comandos comprueban identidad antes de editar/eliminar para rechazar selecciones caducadas. El inspector permite ajustar tono, inicio, duración y velocidad, además de borrar.
- Las pruebas focales y las 139 pruebas de workspace pasaron antes de añadir los gestos de arrastre; repetir la suite completa y validaciones antes de publicar este seguimiento. No se hizo QA visual.

## Continuación — gestos de nota en piano roll

- Arrastrar el cuerpo de una nota la mueve a pasos de corchea y cambia su tono según las filas; arrastrar el borde derecho modifica duración, con mínimo de una corchea. Ambos gestos usan `UpdateMidiNote`; se conserva el comando numérico como alternativa.

## Actualización — grabación de audio Tauri (avance OpenSpec 2.5)

- Continuación local desde `acad816` (entrada PipeWire por pista). El Mezclador
  ahora arma/desarma pistas de audio con entrada asignada. Record captura cada
  pista armada a WAV en `media/recordings/`; Stop finaliza la captura y registra
  fuente/región mediante `ImportAudio` en el cursor inicial. La región entra al
  historial reversible y usa el decodificador incremental ya conectado.
- El stream PipeWire de entrada alimenta un ring SPSC y el escritor WAV por rings
  separados; el callback no escribe al disco. Se requiere guardar el proyecto.
  Durante la primera toma se deshabilita pausa y se rechaza A/B, porque aún no
  hay cuenta previa ni captura por secciones. Si el ring WAV pierde muestras,
  se conserva la región y el error queda visible tras refrescar la instantánea.
- Verificación: `cargo fmt --all -- --check`, `cargo check --workspace`,
  `node --check` de ambos scripts frontend y `git diff --check` pasaron.
  `openspec validate workstation-arrangement-surface-v2 --strict` conserva
  ocho avisos lingüísticos preexistentes. No se ejecutaron pruebas ni QA
  funcional/acústica con AudioBox. El commit del corte se identifica en `git log`.
- OpenSpec 2.5 es avance parcial, no completa. Pendiente comprobar Record → Stop
  → reproducción → undo/redo en Tauri con entrada física; también cuenta previa,
  tomas por secciones, medidor de entrada, salidas físicas por pista y asegurar
  el manejo de desbordamientos. 2.4 sigue parcial por selección de salidas y QA.

## Estado previo — entrada física por pista (avance OpenSpec 2.4)

## Actualización — entrada física por pista (avance OpenSpec 2.4)

- El modelo guarda `TrackInputRoute` como clave opaca PipeWire y selección mono/estéreo de canales 1/2. `SetTrackInputRoute` aplica por historial reversible; el dominio valida sólo identidad y forma, sin depender del backend.
- El Mezclador enumera fuentes físicas y permite seleccionar la entrada/canales; el resumen muestra el sentido de la señal. La configuración se cambia con el transporte detenido y se aplica al siguiente Play.
- PipeWire captura cada ruta a un ring SPSC preasignado. El plan consume y mezcla la entrada a la pista antes de ganancia/pan y ruteo interno. Las fuentes ausentes producen error al iniciar.
- Verificación: `cargo fmt --all -- --check`, `cargo check --workspace`, `node --check` de ambos scripts frontend, `git diff --check` pasan. `openspec validate workstation-arrangement-surface-v2 --strict` conserva ocho avisos lingüísticos ya conocidos. No se ejecutaron pruebas ni QA manual de AudioBox.
- Pendiente en 2.4: salidas físicas por pista, consultar canales/capacidades reales, medidor de entrada y QA con AudioBox/desconexión. Grabación/armado y control explícito de monitorización corresponden a 2.5. No marcar 2.4 como completa.

## Actualización — búsqueda de transporte durante Play

- Continuación publicada: consultar `git log`; la búsqueda se aplica mediante un plan nuevo en el siguiente límite de bloque y conserva abierto PipeWire. Un clic en la regla reubica audio y MIDI; el scheduler sustituye la agenda y las regiones se decodifican desde la nueva posición.
- Verificación del corte: `cargo fmt --all`, `cargo check -p estudio-daw-ui-shell`, `cargo fmt --all -- --check`, `node --check` de `main.js` y `platform-tauri.js`, `git diff --check` y `openspec validate workstation-arrangement-surface-v2` pasan; ocho avisos lingüísticos conocidos. No se ejecutaron pruebas ni QA auditiva/visual.
- `workstation-arrangement-surface-v2` sigue en 9/35; 2.3 y 4.3 continúan abiertas. Pendientes: loop, rebuild al editar, otros controladores MIDI, alinear el scheduler al reloj del plan y QA. La búsqueda se ofrece sólo mientras está en Play; pausado se debe reanudar primero.
- No se modificó el dispositivo ni el callback para preparar planes. El control publica el plan desde Tauri y espera la recuperación del anterior fuera de RT; el scheduler usa todavía reloj de pared para los tiempos de eventos. Continuar con esa limitación documentada.
- El modelo ahora guarda `TransportLoopRange` opcional (960 PPQ) y la UI define A/B desde el cursor mediante `SetTransportLoopRange`, reversible. Esto sólo configura el rango; no activa aún el loop. Continuar preparando un salto coordinado para audio/MIDI.

## Reanudación — reproducción incremental de regiones (OpenSpec 2.3)

- HEAD publicado tras los cortes 2.3/4.3: `c534142` (`feat: muestra posición real del transporte`). La búsqueda inicial desde el cursor de Arreglo está en curso; consultar `git status/log` al reanudar.
- `AudioPcmDecoder` usa `ffmpeg` para transmitir PCM f32 estéreo. Un worker por región llena un ring PCM SPSC de un segundo, limitado a 64 regiones y precargado antes de abrir PipeWire. `AudioClipMixerNode` mezcla en el plan ya compilado con MIDI y aplica posición inicial, desplazamiento/duración, canales, ganancia y fades. El callback sólo consume el ring y mezcla en scratch preasignado.
- Pause congela el plan; Stop libera los workers/cancela `ffmpeg`. Cambiar regiones mientras corre guarda el proyecto pero no reconstruye el plan activo; detener e iniciar carga los cambios. Al iniciar desde detenido se puede reproducir desde el cursor de Arreglo y buscar durante Play desde la regla sin cerrar PipeWire. La repetición coordinada de audio/MIDI aún no está conectada.
- `workstation-arrangement-surface-v2` permanece en 9/35; 2.2 sigue abierta por QA visual y 2.3 sigue abierta por las capacidades de transporte ausentes y QA acústica. No marcar ninguna completa por esta implementación parcial.
- En el corte `c534142`, la búsqueda durante Play seguía pendiente; quedó implementada luego en `c5e7ec4`. La posición del cabezal viene de `TransportClock` con frames procesados por PipeWire. Continúa pendiente el loop coordinado.
- Al iniciar/buscar desde el cursor, el scheduler restaura notas activas y sustain CC64 por clip. Otros controladores previos no se reconstruyen todavía.
- Verificación del corte local: `cargo check -p estudio-daw-ui-shell`, `cargo fmt --all -- --check`, `node --check` para `main.js` y `platform-tauri.js`, `git diff --check` y validación OpenSpec pasan; no se ejecutaron pruebas ni QA física.
- Próximo paso: conectar el rango A/B persistido al salto coordinado de audio/MIDI, preparar el plan alternativo fuera del callback y comprobar fin de rango sin que el audio pase del punto B. Después continuar con rebuild al editar y las demás dependencias de 2.3/4.3.

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

## Actualización — loop A/B conectado a audio/MIDI

- Punto inicial: commit publicado `33836f5` (`feat: persiste rango de repetición A/B`). La implementación conecta ahora el rango al runtime Tauri: B limita la salida de audio y el plan alternativo reinicia audio/MIDI desde A mediante publicación en límite de bloque, manteniendo PipeWire abierto.
- La primera vuelta adicional se prepara antes de abrir el stream; el coordinador prepara vueltas posteriores fuera del callback. Pause detiene el avance y la aplicación del salto hasta reanudar. Los errores al preparar/aplicar se reflejan al consultar posición.
- Un rango editado durante una sesión iniciada se aplica al siguiente inicio de Play. El scheduler MIDI aún se basa en reloj de pared, así que no afirmar sincronía sample-accurate ni QA acústica.
- Verificación: `cargo check -p estudio-daw-ui-shell`, `cargo fmt --all -- --check`, `git diff --check`; `openspec validate workstation-arrangement-surface-v2` válida con ocho avisos lingüísticos conocidos. No se ejecutó ninguna suite de pruebas.
- OpenSpec sigue 9/35; las tareas 2.3 y 4.3 continúan abiertas por actualización del plan al editar, controladores MIDI restantes, sincronía del scheduler y QA funcional/acústica. Consultar el último commit de `main` para el SHA final.
- Siguiente: continuar 2.3 con actualización del plan activo cuando cambian regiones/proyecto; mantener en paralelo los pendientes de 4.3. Vault append-only actualizado en esta intervención.

## Actualización — selector de salida PipeWire y claridad arquitectónica

- Se añadió en Ajustes de audio la enumeración de sinks PipeWire y la selección de salida Master para el siguiente inicio del transporte. La opción automática conserva AudioBox como preferencia y recurre a la salida predeterminada; una selección manual ausente informa error antes de construir el plan. La selección es global de aplicación: todavía no asigna destinos físicos independientes a pistas ni implementa captura de entrada.
- `estudio-daw-project devices` enumeró en este equipo salidas HDMI, AudioBox USB 96, audio interno y Bluetooth. `cargo fmt --all -- --check`, `cargo check --workspace`, ambos `node --check`, diff check y validación OpenSpec pasaron; ésta conserva ocho avisos lingüísticos conocidos. No se ejecutaron pruebas.
- Se aclaró en `AGENTS.md` que escritorio integrado/modular describe el producto y sus límites por crates; `docs/architecture-v2.md` ya define el monolito modular como un producto integrado sin microservicios de dominio. No significa un solo crate ni prohíbe workers y procesos auxiliares.
- La ventana Tauri que se lanzó en esta sesión usa el binario anterior a este selector; no cerrar si hay cambios de proyecto sin guardar. Para revisar el selector, reiniciar cuando sea seguro. OpenSpec sigue 9/35; 2.2/2.3/2.4 siguen parciales y 2.4 sólo avanzó en selección de salida Master.
- El usuario revisó la ventana y confirmó que el zoom de interfaz funciona. La confirmación se registra como QA visual del zoom; la tarea 5.7 sigue parcial por el resto de atajos/menús.

## Actualización — recompilación al editar regiones

- Mover, recortar y quitar audio mientras está en Play actualiza el plan desde la posición musical vigente y lo publica en el siguiente límite de bloque; PipeWire sigue abierto. En pausa se conserva el cambio y se reconstruye al reanudar.
- El modelo para el coordinador de loop usa revisión atómica más snapshot protegido fuera del callback. Cada vuelta futura se prepara con la revisión más reciente. Publicación/retiro del plan y reemplazo de agenda se serializan con la búsqueda manual para evitar cruces entre plan y MIDI.
- Verificación local: `cargo check -p estudio-daw-ui-shell`, `cargo fmt --all -- --check`, ambos `node --check` y `git diff --check`; no se ejecutaron pruebas ni QA visual/acústica. OpenSpec sigue 9/35; 2.3 y 4.3 permanecen abiertas por QA, controladores MIDI restantes y sincronía del scheduler.
- Consultar `git log` para los SHAs de implementación/documentación que publican este avance y la actualización de tareas.

## Actualización — scheduler guiado por el reloj de audio

- El scheduler compara los ticks absolutos de cada evento con la posición publicada por `TransportPositionNode` después de procesar el bloque. Pause congela el reloj, y seek/loop reemplazan la agenda cuando el plan nuevo queda activo; ya no se acumula deriva de un `Instant` independiente.
- El scheduler sondea el atómico cada 1 ms y los instrumentos consumen eventos por bloque/worker. Esto alinea la agenda con el transporte, pero no garantiza aplicación sample-accurate. Falta QA funcional/acústica y restauración de otros controladores MIDI.
- Validar con `cargo check`, formato, sintaxis JS, diff check y OpenSpec; no ejecutar suites. Consultar el commit más reciente en `git log` y actualizar vault.

## Actualización — restauración de estado CC al navegar

- Al reconstruir el plan desde el cursor, se envía el último valor previo de cada CC 0–127 de los clips MIDI activos antes de NoteOn restaurados y eventos futuros. CC64 conserva su lógica de sustain y CC123 vacía las notas activas del canal.
- Pitch Bend, Key/Channel Pressure y Program Change aún no forman parte de `SynthMidiEvent` ni del estado restaurado al buscar. No declarar restauración completa de todos los tipos MIDI.
- Verificación: `cargo check -p estudio-daw-ui-shell`, fmt/fmt check y `git diff --check`; no se ejecutaron pruebas ni QA auditiva/visual. OpenSpec sigue 9/35.

## Actualización — restauración de más estados MIDI

- `SynthMidiEvent` ahora transporta Pitch Bend firmado, Key Pressure, Channel Pressure y Program Change además de notas/CC. FluidSynth los aplica en su worker; el sinte sinusoidal los acepta sin efecto. La biblioteca local exporta los cuatro símbolos FFI requeridos.
- Al buscar/iniciar a mitad de clip se restaura Pitch Bend, presión de canal, programa y presión por tecla para notas aún activas, además de CC 0–127 y notas sostenidas. SysEx permanece sin implementar.
- Verificación: `cargo check -p estudio-daw-synth -p estudio-daw-ui-shell`, formato y `nm -D /usr/lib/libfluidsynth.so` confirmó los símbolos requeridos. No se ejecutaron pruebas ni QA visual/auditiva. OpenSpec sigue 9/35.

## Continuación — pánico MIDI de transporte

- El botón «!» envía CC64=0 seguido de CC123 por los 16 canales de cada instrumento MIDI activo. El comando se entrega al scheduler fuera del callback; está disponible durante Play o pausa y no detiene ni altera el estado del proyecto.
- OpenSpec `workstation-arrangement-surface-v2` continúa 9/35 y 4.3 sigue abierta: falta metrónomo, precisión sample-accurate y QA funcional/acústica. La tarea no se marca completa.
- En esta intervención: formato, `cargo check -p estudio-daw-ui-shell`, `node --check` de scripts frontend, `git diff --check` y validación OpenSpec; no ejecutar suites de pruebas. Consultar `git log` para SHAs publicados y actualizar la bitácora de bóveda.

## Avance — metrónomo de transporte

- Publicado en `9afd8dc`. `MetronomeNode` mezcla un clic de seno corto en el plan de instrumentos; acentúa el primer pulso del compás según tempo/métrica y alinea el siguiente pulso desde la posición inicial del plan. Un `AtomicBool` compartido con planes de seek/loop permite activar/desactivar sin reconstruir ni tocar el callback desde Tauri.
- El botón «♪» conmuta el metrónomo desde detenido o durante Play y pausa; inicia apagado. Verificar auditivamente su pulso/acento cuando se haga QA de transporte.
- OpenSpec sigue en 9/35; 4.3 permanece abierta por sincronía sample-accurate de eventos MIDI y QA funcional/acústica. No ejecutar pruebas en esta intervención.

## Avance parcial — controles de mezcla de pista

- Publicado en `037aa15` y extendido a las tres superficies en `9cd8e42`. Session, Arrangement y Mezclador presentan ACT/M/S y sliders de ganancia/panorama para pistas no master. Las interacciones emiten `SetTrackMixer` reversible; el backend refresca el plan en Play y deja las ediciones listas para aplicar al reanudar si está pausado.
- El plan hace efectivos active/mute/solo/gain/pan para MIDI y regiones de audio. El master no ofrece controles, y no hay medidores ni agrupación; no marcar 4.1 completa.
- Verificación local: fmt/fmt check, `cargo check -p estudio-daw-ui-shell`, ambos `node --check`, diff check y OpenSpec normal (ocho avisos lingüísticos conocidos). No se ejecutaron suites ni QA visual/acústica. No marcar 4.1 completa.


## Avance — movimiento de clips MIDI en Arrangement

- Commit de implementación: `c0340821b2f3643b23c6783eebf243d542067434` (`feat: move MIDI clips in arrangement`), pendiente de publicación al guardar este handoff.
- Los clips MIDI se arrastran en Arrangement; el gesto sigue la rejilla seleccionada y convierte pulsos a ticks con el PPQ propio del clip. `ProjectCommand::MoveMidiClip` conserva toma/eventos, entra al historial y Tauri refresca el plan conectado.
- Verificación: `cargo test -p estudio-daw-command-bus -- --test-threads=1` (15 pasaron), `cargo test --workspace -- --test-threads=1` pasó, `cargo check --workspace`, `cargo fmt --all -- --check`, ambos `node --check`, `git diff --check` y `openspec validate workstation-arrangement-surface-v2 --strict` pasaron. No hubo QA visual manual.
- OpenSpec continúa en 9/35; 3.3 y 5.3 siguen parciales. Pendiente división/duplicación MIDI, marcadores y QA visual/funcional. Siguiente corte según el handoff aprobado: seguir 2.2, completar el flujo del navegador/inspección de medios de audio y revisar adaptadores ya existentes antes de añadir rutas paralelas.


## Continuación — duplicación de clips MIDI

- Commit `9bd3e134f850245e915326149293fce661ac6bae` (`feat: duplicate MIDI clips in arrangement`) publicado en `main`.
- El menú contextual duplica un clip MIDI al final del original con ID único, duración y toma iguales. La copia no cambia slots de Session; `DuplicateMidiClip` es una sola transacción reversible y Tauri recompila el plan activo si corresponde.
- Verificación conjunta de movimiento y duplicación: `cargo test -p estudio-daw-command-bus -- --test-threads=1` (16 pasaron), `cargo test --workspace -- --test-threads=1` (135 pruebas aprobadas), `cargo check --workspace`, fmt/fmt check, ambos `node --check`, `git diff --check` y OpenSpec estricto pasaron. Sin QA visual manual.
- OpenSpec sigue 9/35; 3.3/5.3 avanzan parcialmente. Faltan división MIDI, piano roll, edición de notas, marcadores y QA visual/funcional. 2.2 mantiene implementado el flujo de audio y pendiente QA manual de importación/gestos/fuente.


## Avance — división de clips MIDI en el cursor

- Commit `65b808fa64ac11d2f76bab177ff1c8dde1ade4dc` (`feat: split MIDI clips at arrangement cursor`) publicado en `main`.
- El menú contextual ofrece «Dividir en cursor» sólo dentro de los límites del clip. El comando reversible deja la primera parte con el ID original y crea la segunda contigua con ID único; las notas abiertas se cierran y rearticulan en el corte, y se restaura el estado previo de CC, pitch bend, presión y programa. Un Note Off en el punto de corte cierra la primera parte; SysEx anterior no se repite. El plan conectado se recompila. La rearticulación puede producir un nuevo ataque en notas sostenidas.
- Verificación: `cargo test --workspace -- --test-threads=1` (137 aprobadas), `cargo test -p estudio-daw-project-model -p estudio-daw-command-bus -- --test-threads=1` (40 aprobadas tras ajuste final de ID), `cargo check --workspace`, fmt/fmt check, ambos `node --check`, `git diff --check` y OpenSpec estricto pasaron. Sin QA visual manual.
- OpenSpec sigue 9/35 y 3.3/5.3 parciales. Pendientes: piano roll/edición de notas, marcadores, operaciones MIDI adicionales y QA visual. 2.2 mantiene pendiente la pasada manual de importación, edición y conservación de fuentes.
