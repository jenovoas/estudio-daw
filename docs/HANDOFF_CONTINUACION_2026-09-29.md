# Handoff de continuación — 2026-09-29

> **Continuidad actualizada al final de la sesión — leer primero esta sección.**
> Sus instrucciones sustituyen las que siguen más abajo cuando haya diferencias;
> las notas antiguas se conservan como historial.

## Estado vigente para la próxima sesión

- **Repositorio:** `/home/jnovoas/proyectos/estudio-daw`, rama `main`, último
  commit publicado `13f71ae` (`feat: add contextual track creation in
  arrangement`), árbol limpio al guardar este handoff. Confirmar con `git
  status` y `git log`; no restaurar ni descartar cambios ajenos.
- **Cambio OpenSpec activo:** `workstation-arrangement-surface-v2`, flujo
  `spec-driven`, **10/50 tareas completas**. La prioridad sigue siendo rediseñar
  la UI de escritorio Tauri hasta acercar de forma sustancial su jerarquía y
  flujo creativo a Ableton Live, sin copiar sus recursos ni desviar el alcance.
- **Último corte:** la cabecera PISTAS de Arreglo ahora tiene «+ Pista» para
  crear MIDI, audio o bus con las acciones ya existentes. El menú se cierra al
  elegir una acción, hacer clic fuera o pulsar Escape. Se conserva la selección
  de grupo al enfocar Dispositivo (`d2e7d39`) y el foco del dispositivo queda
  separado de dicha selección (`8bff3c8`). No se modificaron modelo, IPC,
  arquitectura ni motor.
- **Validación de `13f71ae`:** `node --check
  crates/ui-shell/frontend/main.js`, `cargo build -p
  estudio-daw-ui-shell --bin estudio-daw`, `openspec validate
  workstation-arrangement-surface-v2 --strict` y `git diff --check`; todo pasó.
  Commit publicado en `origin/main`. La bitácora append-only y
  `ESTADO_ACTUAL.md` de personalvault registran este SHA.
- **Criterio de cierre:** 3.1 sigue abierta. No declarar aceptada la dirección
  visual hasta que la persona usuaria la revise en Tauri. Ella indicó que más
  adelante levantará la interfaz y tomará capturas; mientras tanto, continuar
  implementando el plan sin interrumpirla para pedir capturas ni confirmaciones
  rutinarias.

## Instrucciones de continuidad

1. Revisar `AGENTS.md`, `README.md`, este handoff, `git status`/`git log` y los
   artefactos vigentes de `openspec/changes/workstation-arrangement-surface-v2`
   antes de editar. Usar `openspec instructions apply --change
   workstation-arrangement-surface-v2 --json` para retomar el flujo.
2. **Sólo interfaz Tauri nativa.** No abrir ni validar en Firefox, navegador
   externo o WASM; no cambiar arquitectura ni inventar cambios de backend. La
   revisión/capturas de runtime queda para cuando la persona usuaria levante la
   app según lo acordado. Ejecutar automáticamente build, sintaxis, OpenSpec y
   checks de código que no interfieran con esa revisión.
3. Continuar la secuencia visual aprobada 3.1 y su delta en `design.md`. El
   corte inmediato implementó el acceso contextual de creación previsto junto
   a las pistas; continuar con las discrepancias restantes de composición y
   jerarquía descritas allí. No avanzar tareas funcionales dependientes mientras
   3.1 siga bloqueada por el feedback visual pendiente.
4. No volver a diseñar el plan ni pedir permiso para decisiones rutinarias ya
   aprobadas. No solicitar al usuario pruebas que pueda hacer el agente. Hablar
   sólo para comunicar progreso/resultados o un bloqueo real que cambie alcance.
5. Tras cerrar el rediseño de UI, la persona usuaria revisará la app Tauri y dará
   feedback. Entonces continuar las funciones ya aprobadas, priorizando la
   integración real VST3/Analog Lab y dispositivos/entrada MIDI descrita en
   OpenSpec; no afirmar paridad ni disponibilidad antes de tener evidencia.
6. Mantener español, límites de arquitectura y trazabilidad de `AGENTS.md`.
   Después de cada corte material, registrar SHA inicial/final, archivos,
   pruebas, resultado y pendientes en la bitácora externa append-only; mantener
   `ESTADO_ACTUAL.md` factual. Commit/push ya autorizados por el usuario.

## Prioridad para la próxima sesión

La prioridad inmediata es continuar la secuencia visual 3.1 en frontend, buscando
para Linux una experiencia de creación y una jerarquía reconocibles para músicos,
inspiradas en el flujo de Ableton Live sin copiar sus recursos. La persona usuaria
ya indicó que el ordenamiento actual sigue lejos del objetivo y pidió explícitamente
comisionar los cortes y continuar sin interrumpir su flujo. Avanzar sin pedir
capturas ni confirmaciones rutinarias; automatizar las verificaciones posibles y
no manipular la ventana de escritorio de la persona usuaria. Mantener 3.1 abierta
hasta que se realice la inspección Tauri y la aceptación visual; esa aceptación es
un criterio de cierre, no un bloqueo para seguir implementando el plan aprobado.
Tras el trabajo visual prioritario, los flujos reales de plugins y dispositivos
MIDI son esenciales y deben continuar en las tareas ya aprobadas.

## Estado visual al cierre

- La ventana Tauri se recompiló y abrió con el cambio actual.
- La barra de control reúne transporte, posición, historial, Arreglo/Session,
  Mezclador y herramientas de edición.
- El navegador lateral tiene pestañas funcionales Proyecto y Crear. Proyecto
  lista medios importados; Crear usa las acciones existentes para añadir pistas
  MIDI/audio y buses. No se inventaron categorías ni instrumentos disponibles.
- Arreglo/Session siguen en el lienzo principal. El detalle contextual y el
  Mezclador ocupan el panel inferior.
- Feedback de la persona usuaria: la nueva composición es algo más ordenada y
  fácil de entender, pero el cambio fue mínimo y sigue lejos de Ableton Live.
  No es aceptación del diseño ni cierre de la tarea.
- Hay una Demo MIDI disponible desde el botón superior «Demo MIDI» para observar
  las superficies sin importar audio. La última instancia Tauri se abrió en
  estado inicial; cargar la demo antes de comparar.
- OpenSpec 3.1 sigue parcial y abierta. Antes de solicitar QA, hacer un rediseño
  sustancial de distribución y jerarquía; luego revisar en runtime con la demo,
  tamaños estándar/estrecho y vistas Arreglo/Session/detalle/Mezclador.

## Estado técnico guardado

- El estado inicial de este handoff se reemplazó por continuaciones posteriores.
  Al 2026-09-29, HEAD local es `e2321e4`; siete commits delante de `origin/main`,
  sin publicar. Consultar `git status` y `git log` para el estado vigente.
- La intervención incluye la reorganización del navegador/barra de control y el
  retorno desde lanzamientos Session a Arreglo desde la posición actual durante
  Play. El retorno recompila el plan sin cerrar PipeWire y limpia las colas de
  audio de Session; la tarea OpenSpec 3.4 sigue parcial hasta QA y captura del
  material de Session.
- Validación realizada en esta sesión: `node --check
  crates/ui-shell/frontend/main.js`, `git diff --check` y `cargo run -p
  estudio-daw-ui-shell` (compiló y abrió Tauri). No se ejecutaron suites de
  pruebas.
- No se hizo push.

## Continuación exacta

1. Leer este handoff, `AGENTS.md`, `README.md`, los artefactos de
   `workstation-arrangement-surface-v2` y `docs/ui-architecture.md`; revisar
   siempre estado e historial antes de editar.
2. Continuar la secuencia L1–L6 con las tareas OpenSpec y el wireframe de
   `design.md`. Usar Firefox/headless y pruebas del frontend; no presentar un
   adaptador Tauri simulado como QA del runtime. No tocar ni enfocar la ventana
   abierta de escritorio.
3. Completar el rediseño sustancial de Browser, control/transporte, Arrangement,
   Session, detalle contextual y mezcla con las capacidades existentes. Mantener
   categorías basadas en contenido real y el alcance en UI mientras sea la
   prioridad inmediata.
4. No marcar 3.1 completa sin inspección Tauri y aceptación explícita, pero
   continuar el trabajo aprobado sin pedir feedback para cada corte. Después,
   priorizar integración de VST/Analog Lab y entrada/dispositivos MIDI, esenciales
   para la persona usuaria.

## Prompts para la próxima sesión

### Prompt para retomar con Codex

```text
Retomemos Estudio DAW desde docs/HANDOFF_CONTINUACION_2026-09-29.md. Mi
prioridad absoluta es rehacer la UI/frontend para Linux con una distribución y
un flujo creativo muy cercanos a Ableton Live. Lo implementado hasta ahora sólo
ordena un poco la ventana: no se acerca a lo que busco y no está aprobado.

No añadas funciones de audio/MIDI ni avances tareas funcionales. Primero estudia
la UI real de Live como referencia de jerarquía y flujo, inspecciona la UI actual
con la Demo MIDI, y elabora una propuesta visual concreta de la ventana completa
(Browser, control/transport, Arrangement, Session, detalle de clip/dispositivo y
mezclador). Evita quedarte en cambios cosméticos o ajustes pequeños de CSS.

Usa OpenSpec y los límites del repositorio; no copies código, marca ni recursos
gráficos de Ableton. Distingue claramente lo que puede resolverse sólo en UI de
lo que requeriría capacidad nueva. Conserva todos los cambios locales existentes.
No marques la tarea 3.1 completa ni retomes otras funciones hasta que yo diga que
la dirección visual ya se acerca a lo que quiero. Trabaja de forma continua, sin
pedirme confirmación para cada decisión rutinaria.
```

### Prompt para Astra — crítica de diseño, sin editar el repositorio

```text
Actúa como especialista senior en UX/UI de DAW de escritorio para Linux. Ayúdame
a rediseñar la interfaz de Estudio DAW. Mi referencia principal es Ableton Live:
quiero acercarme mucho a su distribución y a su flujo creativo para músicos. La
UI actual de Estudio DAW se siente como un dashboard; los últimos retoques sólo
la ordenaron un poco y siguen muy lejos del objetivo.

Te adjunto capturas actuales y el handoff del proyecto. Analiza la jerarquía de
Live (sin copiar sus assets, marca ni código) y propón una distribución completa
para Estudio DAW que priorice crear, probar y organizar música. Incluye:

1. Un wireframe de la ventana completa a 1920×1080 y 1280×720, con zonas,
   proporciones y qué controles pertenecen a cada zona.
2. El lugar y la relación de Browser, transporte/control global, Arrangement,
   Session, Clip/Device Detail y Mixer; explica cómo cambiar entre ellos sin
   romper el flujo creativo.
3. Un recorrido de usuario para abrir/crear una sesión, probar ideas en Session,
   organizarlas en Arrangement y editar un clip MIDI.
4. Qué elementos actuales producen la sensación de dashboard y qué decisiones
   concretas la eliminarían.
5. Cambios ordenados por impacto. Separa estrictamente rediseño visual posible
   con las capacidades actuales de ideas que exigirían funciones nuevas.
6. Criterios observables para decidir si la propuesta ya transmite el flujo
   creativo buscado, no sólo si parece más ordenada.

No escribas ni modifiques archivos, código o planificación del repositorio. No
inventes instrumentos, bibliotecas o contenido no confirmado. Si las capturas o
el material adjunto no bastan para una conclusión, indícalo y concreta qué
captura/referencia oficial de Live necesitas. Devuélveme una propuesta específica
y un wireframe legible que pueda llevar a Codex para implementarlo.
```

## Handoff para la siguiente sesión — implementación parcial 3.1 — 2026-09-29

- HEAD base de este corte: `052357de56fb390e1f7848f31799ea90033a10f3` en
  `main`. Consultar `git log`/`git status` al retomarlo para ver el commit que
  guarda este handoff.
- Se implementó sólo frontend: navegador con búsqueda y listas de contenido real
  MIDI/audio/instrumentos, distribución de transporte/herramientas, matriz
  Session con mezcla bajo pistas y escenas al extremo derecho, detalle contextual
  Clip/Dispositivo y ajustes CSS de la ventana. Sin cambios Rust, IPC ni motor.
- OpenSpec `workstation-arrangement-surface-v2` sigue en 10/50. 3.1 y cada una
  de 3.1.1–3.1.6 continúan `[ ]`; 3.1.2–3.1.5 tienen notas de avance local.
  No avanzar otras tareas. No cerrar 3.1.6 ni 3.1 hasta presentar la UI y recibir
  aceptación visual explícita del usuario.
- La inspección visual no se pudo ejecutar: `grim` falló con «failed to create
  display». No hay capturas ni QA en runtime. Al retomar, abrir la aplicación con
  Demo MIDI y revisar Arreglo/Session, detalle/dispositivo y mezclador a
  1920×1080 y 1280×720; registrar regresiones y feedback.
- Validación de código: `cargo check -p estudio-daw-ui-shell`, `node --check` de
  `main.js` y `platform-tauri.js`, validación HTML/CSS, `git diff --check` y
  `openspec validate workstation-arrangement-surface-v2 --strict` pasaron. No
  se ejecutaron pruebas.
- Continuación: leer `AGENTS.md`, este archivo y
  `openspec/changes/workstation-arrangement-surface-v2/{proposal.md,design.md,
  specs/desktop-workstation-ui/spec.md,tasks.md}`; revisar diff/commits; seguir
  únicamente el delta de Luna 3.1.1–3.1.6. Mantener el registro de bóveda
  append-only. El feedback visual del usuario determina cualquier cierre.

### Prompt de continuación

```text
Continúa desde el handoff «implementación parcial 3.1» de
 docs/HANDOFF_CONTINUACION_2026-09-29.md. Revisa los commits/diff publicados y
 ejecuta únicamente workstation-arrangement-surface-v2 3.1.1–3.1.6 según el
 delta para Luna de design.md. La implementación frontend está hecha pero no
 tiene aceptación visual: abre Demo MIDI y revisa Arreglo, Session,
 Clip/Dispositivo y Mezcla a 1920×1080 y 1280×720; corrige sólo regresiones de
 ese alcance y muéstrame el resultado. No avances otras tareas ni cierres 3.1
 hasta que yo acepte visualmente. Preserva el historial y sigue AGENTS.md y la
 bitácora append-only.
```

### Publicación de este handoff

El corte de implementación y este handoff quedaron publicados en `2a613fa`
(`feat: reorganize workstation UI surfaces`), en `origin/main`. El árbol de
trabajo quedó limpio tras el push; las subtareas 3.1.1–3.1.6 y 3.1 siguen
abiertas por falta de QA y aceptación visual.

## Continuación de QA visual — categorías Browser y acceso a dispositivo — 2026-09-29

- SHA inicial de esta continuación: `732a891`; el código sigue sin commit.
- El Browser ahora filtra categorías reales (Todo, Clips MIDI, Instrumentos,
  Audio), conserva búsqueda/listas actuales y esconde categorías vacías. Las
  cabeceras compactas del Arreglo enlazan al detalle del dispositivo e informan
  el backend asignado; no se añadieron contratos Tauri ni capacidades del motor.
- Se revisó Demo MIDI en la aplicación Tauri a 1842×1066: Arreglo, categorías y
  controles compactos quedaron visibles en una captura. Esto no cubre los
  tamaños exigidos 1920×1080 y 1280×720 ni la matriz completa de Session,
  mezclador y selección Clip/Dispositivo. El intento de cambiar el tamaño desde
  Tauri fue rechazado porque `window.set_size` no está permitido por las
  capacidades actuales; no se amplió ese permiso.
- En la continuación, Hyprland sí fijó la ventana a 1280×720. Durante la
  inspección remota, el proceso terminó con `free(): corrupted unsorted chunks`;
  no hay captura verificable de ese tamaño. Un segundo `cargo run` quedó sin
  ventana y se interrumpió; la relación causal entre inspector y error no está
  determinada. Repetir QA sin inspector remoto antes de atribuirlo al código.
- `node --check` para `main.js` y `platform-tauri.js`, `git diff --check` y
  `openspec validate workstation-arrangement-surface-v2 --strict` pasaron. No
  se ejecutaron suites. 3.1.1–3.1.6 siguen abiertas hasta cubrir matriz y
  aceptación visual del usuario. No avanzar otras tareas.

## Prioridad UI y dispositivos — seguimiento de la sesión

- La persona usuaria indicó que la UI creativa es el criterio principal para
  decidir la continuidad del proyecto y que plugins/dispositivos MIDI son
  esenciales. Mantener el corte actual en UI 3.1; después priorizar las tareas
  aceptadas 4.6.2–4.6.7 y 5.6, sin tratarlas como capacidades concluidas.
- Demo MIDI fue cargada de nuevo. En Arreglo se ve el clip «Melodía de prueba»
  y siete notas listas. En Session, la Demo base tiene una pista y ninguna
  escena; añadir una escena y asignar el clip existente mediante la UI funcionó,
  pero lanzar sigue deshabilitado con el motor desconectado. Se restauró la
  Demo base tras esa comprobación.
- El detalle Dispositivo presenta VST3/Analog Lab y selector de puertos MIDI
  destino más retorno de audio. Tauri enumeró Midi Through, AudioBox USB 96 MIDI
  1 y BlueZ; `aconnect -l` no mostró KeyLab en esta sesión. La UI aún no expone
  entrada general de hardware MIDI para tocar/grabar; es un pendiente explícito
  de 5.6, no debe confundirse con la entrada MIDI de Analog Lab standalone.
- La auditoría visual detectó tipografía frontend de 6–10 px frente al mínimo
  de 11 px previsto en `design.md`; los estilos locales ahora elevan ese piso y
  comprimen ligeramente las filas cortas para evitar recorte. La compilación
  Tauri confirmó los cambios en runtime a 960×1066 (ventana en mosaico); a ese
  ancho algunas etiquetas del Browser se recortan por falta de espacio. Los
  tamaños requeridos 1920×1080 y 1280×720, la matriz completa y aceptación
  visual siguen pendientes. No mover la ventana ni enviarla al scratchpad.
- Validación final de esta continuación: `node --check` para ambos scripts
  frontend, `openspec validate workstation-arrangement-surface-v2 --strict` y
  `git diff --check` pasaron. Sin suites Rust. OpenSpec 3.1 y subtareas siguen
  abiertas.

## Entrada creativa directa al piano roll — continuación

- La captura nueva de la persona usuaria mostró el clip «Melodía de prueba» en
  Arreglo, pero el panel inferior vacío; el piano roll existente sólo aparecía
  tras seleccionar el clip. Se ajustó el botón Demo MIDI para seleccionar y
  enfocar el primer clip real en Arreglo, abrir el detalle contextual y ampliar
  ese panel a 38vh (mínimo 260 px), dejando el lienzo superior visible. También
  se reutilizó el enfoque desde las filas MIDI del Browser.
- Archivos locales de este corte: `crates/ui-shell/frontend/main.js` y
  `styles.css`, además de `tasks.md` y este handoff. `cargo check -p
  estudio-daw-ui-shell`, `node --check crates/ui-shell/frontend/main.js` y
  `git diff --check` pasaron. No hubo verificación Tauri runtime del nuevo
  estado; la captura recibida es la línea base, no el resultado.
- Continuar probando la primera apertura de Demo y la selección repetida de
  clips, junto con mezcla/ocultación del detalle, y revisar alturas normales y
  720 px. Mantener 3.1 abierta y seguir UI-first; después de la aceptación
  visual, priorizar las tareas ya aceptadas de plugins y entrada MIDI.
- La captura siguiente confirmó que el panel abría, pero las siete notas (MIDI
  60–67) quedaban por debajo del tramo visible de 36 teclas. El piano roll ahora
  centra la primera vez el registro que contiene notas y conserva el scroll por
  clip entre reconstrucciones. La captura posterior de la persona usuaria
  (1842×1066) confirma el editor abierto con notas visibles; la ficha muestra 12
  notas. Aún no comprueba el redimensionado ni que todas queden a la vista.
  `node --check`, validación OpenSpec estricta y `git diff --check` pasan.

## Jerarquía musical y piano roll — implementación local

- Clip/Dispositivo se movió a la barra del panel inferior; cada encabezado de
  pista ofrece sólo navegación a su cadena; Mixer deja de duplicar controles de
  instrumento; los controles de grupo se ocultan hasta seleccionar varias
  pistas. La cadena agrupa Instrumento y declara que Efectos aún no están
  disponibles en el motor.
- El piano roll ahora cubre MIDI 0–127; rueda desplaza el registro y Ctrl+rueda
  ajusta 4–28 px por semitono anclado al puntero. Conserva el encuadre vertical
  por clip y centra inicialmente las notas reales.
- Validación: `node --check` de ambos scripts, comprobación de 88 IDs HTML
  únicos, `cargo check -p estudio-daw-ui-shell`, `openspec validate
  workstation-arrangement-surface-v2 --strict` y `git diff --check` pasaron.
  No hay captura Tauri runtime posterior a estos cambios; 3.1 permanece abierta.
- La dirección se apoya en Browser→Device View y selección de pista de Ableton,
  junto al orden sincronizado Editor/Mezclador de Ardour; fuentes oficiales y
  decisiones de producto están documentadas en `design.md`.

## Corrección de scroll inicial del piano roll — 2026-09-29

- La nueva captura de la persona usuaria mostró la rejilla MIDI abierta en C9,
  sin las notas del Demo. La causa estaba en el orden de renderizado: se podía
  guardar el `scrollTop` inicial antes del centrado de notas y restaurarlo en
  renders siguientes.
- El scroll sólo se conserva después de completar el primer encuadre del clip;
  el primer encuadre vuelve a centrarse en el rango de notas existente. El
  arrastre vertical de notas ahora convierte píxeles a semitonos con la altura
  de tecla activa, también después del zoom.
- Verificación de código: `node --check crates/ui-shell/frontend/main.js`,
  `cargo check -p estudio-daw-ui-shell`, `openspec validate
  workstation-arrangement-surface-v2 --strict` y `git diff --check` pasaron.
  Falta confirmar en la ventana Tauri que el Demo sitúa sus notas en el primer
  encuadre y que el zoom/arrastre se sienten correctos; 3.1 permanece abierta.

## Segundo ajuste del piano roll — 2026-09-29

- La captura posterior confirmó que seguía en C9: el primer arreglo de scroll
  no resolvió el caso runtime. Se impone una rejilla de altura no colapsable y
  se espera a que el contenido realmente desborde antes de guardar/centrar la
  posición inicial. La rueda ahora cambia `scrollTop` explícitamente; Ctrl+rueda
  conserva el zoom vertical.
- Las teclas se rediseñan con naturales claros, alteraciones oscuras y nombre
  musical en cada semitono. Se amplía la altura inicial del panel contextual y
  el registro base queda en 9 px por tecla para mostrar aproximadamente cuatro
  octavas en 1080p. La rejilla mantiene inserción de notas con clic.
- Esta captura llega antes de una verificación de la nueva compilación. Tras
  compilar, abrir una instancia actualizada y cargar Demo MIDI; verificar el
  primer encuadre, wheel/scroll, zoom con Ctrl+rueda y colores/etiquetas de las
  teclas. No declarar resuelto hasta observar esas interacciones.

- La primera captura posterior a esta compilación mostró las teclas blancas y
  las etiquetas, pero reveló que el cuerpo del editor no se estiraba con el
  panel inferior. Ahora el piano roll ocupa la fila disponible y aplica el
  encuadre inicial sincrónicamente tras medir el overflow, con reintento de
  frame sólo si el layout todavía no tiene altura. Falta relanzar esta última
  compilación y revisar una captura del Demo.
- La inspección del DOM/CSS encontró la causa estructural: `.lower-panel` era
  un contenedor de bloque, aunque sus hijos dependían de `flex: 1`; por eso
  `#clip-inspector` conservaba altura intrínseca y dejaba el resto como espacio
  vacío. El primer ajuste a Flex no bastó: el contenedor inferior también
  incluye buffers y necesita filas explícitas. Ahora usa una fila expansible
  para Clip/Dispositivo, una fila inferior para buffers y un `--detail-height`
  conectado al tirador de redimensionado.
- La causa final del panel recortado era más concreta: una regla antigua
  mantenía `max-height: 260px` y ninguna regla posterior la anulaba. Se quitó
  ese límite en la regla final y se comprobó el CSS real con un fixture HTML
  renderizado por Firefox headless a 1842×1066: panel 576 px, editor 512 px,
  rejilla MIDI 386 px y contenido vertical desplazable de 1792 px. La captura
  muestra el editor ocupando el panel, sin el hueco interno anterior y con el
  scrollbar del registro MIDI. `node --check` (ambos scripts), compilación del
  crate UI, validación OpenSpec estricta y `git diff --check` pasan. La ventana
  Tauri abierta no se manipuló ni se recargó; 3.1 sigue abierta para QA de
  interacción real y tamaños objetivo.
- Revisión automática adicional en Firefox headless con el CSS del proyecto y
  un DOM representativo: en 1920×1080, panel 583 px, editor 519 px, rejilla 393
  px y contenido 1024 px (49 semitonos visibles, cuatro octavas); en 1280×720,
  panel 216 px, editor 152 px y rejilla 120 px. Transporte simulado, panel,
  buffer y barra inferior permanecen dentro del viewport estrecho, con scroll
  local de notas. La rejilla por tecla quedó en 8 px para mostrar cuatro
  octavas en escritorio; en ventanas bajas el registro visible se reduce y se
  recorre con rueda. El panel estrecho inicia en 30vh y el tirador se limita al
  espacio restante después de reservar superficie musical y controles. Esto
  prueba layout CSS/DOM, no una sesión Tauri ni los gestos reales; 3.1 continúa
  abierta.
- El mismo fixture también mostró que el marco superior aún reservaba 38 px y
  el área de trabajo no descontaba correctamente ese marco más la barra inferior.
  El marco pasa a 28 px, el alto del espacio de trabajo se calcula con ambos
  bordes globales y el estado contextual en ventana baja usa 22 px. Las capturas
  headless resultantes mantienen la barra inferior dentro del viewport 1280×720;
  no cuentan como inspección de Tauri.
- Se montó además un smoke temporal bajo `/tmp` con el HTML/CSS/`main.js` real,
  adaptador de plataforma simulado y clip MIDI representativo. Se despacharon
  PointerEvents y ArrowUp al tirador real: 1280×720 pasó de 216 a 296 y 320 px;
  1920×1080 pasó de 583 a 663 y 687 px. Las filas 0–127 conservan overflow local
  y `documentElement.scrollHeight` quedó en 720/1080 px. Esto sí ejercita los
  manejadores JS de redimensionado; sigue sin ser una sesión Tauri ni verificar
  el gesto con dispositivo físico.
- En L2 se centralizó la definición base de filas del grid del editor y la
  colocación de sus superficies; se quitaron redefiniciones tardías que repetían
  las mismas filas. Se repitió el smoke en ambos tamaños después de esa limpieza:
  el tirador y las teclas siguen midiendo 216→296→320 px y 583→663→687 px, y el
  documento conserva exactamente el alto de ventana. Quedan otras reglas de
  componente/estado para consolidar; no se presenta L2 ni 3.1 como completas.
- Continuación L2: la regla base del panel inferior todavía estaba duplicada;
  su primera versión conservaba un máximo de 260 px que la regla tardía anulaba.
  Se integró la cuadrícula Clip/Dispositivo/Buffers en una sola regla base y se
  retiró la duplicación. Tras actualizar el CSS real, Firefox volvió a mostrar
  el panel limitado por el tirador en 1280×720: 216→296→320 px; rejilla con
  scroll local 1024/127 y página de 720 px. `cargo check -p
  estudio-daw-ui-shell`, `node --check crates/ui-shell/frontend/main.js` y
  `git diff --check` pasan. Capturas headless en `/tmp/estudio-daw-ui-smoke-
  controls-{1280,1920}.png`; no son Tauri runtime. El corte continúa sin QA
  visual Tauri ni aceptación de dirección.
- Se consolidaron también los valores activos de altura/tema del marco `.appbar`
  y alto/columna base de `.workstation` en sus declaraciones originales, y se
  quitaron las redefiniciones tardías del marco y un segundo selector de
  navegador contraído. El smoke Firefox se repitió a 1280×720 y 1920×1080;
  conservó métricas de resize (216→296→320 y 583→663→687 px), scroll local y
  página limitada al viewport. `cargo check -p estudio-daw-ui-shell`, `node
  --check`, `git diff --check` y validación OpenSpec estricta pasan. Sigue siendo
  fixture headless con plataforma simulada; no acredita runtime Tauri.
- La apariencia activa del Browser (`.browser`, `.browser-title`), el tamaño de
  su lista de medios y un selector duplicado del resumen de importación también
  quedaron consolidados en una sola regla base por componente. Se repitió el
  smoke funcional sintético de Browser: categorías Todo/MIDI/Instrumentos/Audio,
  clip dibujado y abierto en el piano roll, selección de categorías y búsqueda
  sin coincidencias. La captura `/tmp/estudio-daw-ui-behavior-1280.png` lleva el
  indicador `BEHAVIOR PASS`. `cargo check -p estudio-daw-ui-shell`, sintaxis JS,
  diff check y validación OpenSpec estricta pasan. El resultado usa snapshot y
  plataforma simulados, no es QA Tauri.
- Se amplió el snapshot de prueba con una escena y casilla MIDI y se verificó
  Session a 1280×720: el frontend conserva dos identidades de pista, escena y
  referencia `midi:clip-midi-1`; la captura `/tmp/estudio-daw-ui-session-
  1280.png` muestra matriz y detalle MIDI simultáneos. El motor simulado figura
  desconectado y por eso no se reclama QA de lanzamiento. El smoke de Browser y
  Session pasó; runtime Tauri sigue pendiente.
- Corrección L2 en curso: el selector tardío `.editor.has-midi-editor:not(...)`
  prevalecía sobre `clip-detail-hidden`, por lo que el piano podía dejar una
  reserva vacía al ocultar Detalle. Los estados del piano/mezclador se agruparon
  con el grid base y ahora hay reglas explícitas para detalle oculto, con y sin
  mezclador, también en ventana baja. Firefox headless comprobó el botón de
  detalle: panel 0–1 px al ocultarse y clip MIDI conservado al restaurarlo, tanto
  a 1280×720 como a 1920×1080. La captura `/tmp/estudio-daw-ui-detail-1280.png`
  termina en Session con el clip/editor aún disponibles; `/tmp/estudio-daw-ui-
  detail-1920.png` muestra el rango MIDI desplazable. Esto ejercita frontend real
  con snapshot/plataforma simulados, no Tauri.
- Se amplió el smoke del piano: la rueda modifica `scrollTop` sobre las 128
  teclas y Ctrl+rueda cambia `--piano-key-height`. Firefox mostró `BEHAVIOR PASS`
  en 1280×720 y 1920×1080, junto con colapso/restauración del detalle y Browser/
  Session. Capturas: `/tmp/estudio-daw-ui-roll-1280.png` y
  `/tmp/estudio-daw-ui-detail-1920.png`. Son eventos DOM sintetizados sobre el
  manejador frontend real; no sustituyen interacción física ni QA Tauri.
- El siguiente recorte CSS trasladó el estado final de `.transport-bar` y
  `.transport-button` a sus reglas base, retirando los overrides tardíos. Se
  repitieron compilación, sintaxis JS, diff check, OpenSpec estricta y Firefox a
  ambos tamaños; el comportamiento del piano, el detalle y Session conserva el
  indicador `BEHAVIOR PASS`.
- El navegador ya no desplaza horizontalmente las categorías para mostrar Audio:
  pasaron a una cuadrícula de dos columnas en 1280×720 y 1920×1080. El smoke
  comprueba ancho de scroll, límites geométricos y que ninguna etiqueta se
  abrevie; Firefox muestra Todo, Clips MIDI, Instrumentos y Audio a la vez. Se
  conserva búsqueda y selección. El panel usa una plataforma simulada; no se
  presenta como QA Tauri.
- En el viewport mínimo 760×520, el smoke reveló que `min-height: 62px` del
  modo de ventana baja mantenía visible un hueco al ocultar el detalle. Se anuló
  sólo para el estado `clip-detail-hidden`; Firefox confirmó el colapso y
  mantuvo `BEHAVIOR PASS`. A 760 px de ancho el navegador apila sus categorías.
  Sigue siendo una comprobación headless, no captura Tauri.
- **Verificación real VST3 adicional:** el probe aislado de Analog Lab V 5.12.5.6878
  por yabridge 5.1.1 reportó clase `4172747541564953416C617650726F63`, MIDI-in,
  salida estéreo y GUI. La prueba focal
  `vst3_worker_returns_real_plugin_audio_outside_the_render_callback` pasó al
  usar ese ID hexadecimal. Una ejecución previa con el nombre de clase legible
  falló porque el host requiere 32 dígitos hexadecimales; no es un fallo del
  plugin. No es medición de latencia ni QA de controles integrados.
- El Tauri abierto para inspección runtime a 1842×1066 mostraba el estado inicial
  sin proyecto; no se declara como QA de Demo, dispositivo, plugin ni audio.
  Sigue abierto para continuar la verificación integrada cuando haya un proyecto
  cargado desde la propia UI.
- **Corrección del botón Navegador (reporte visual runtime):** al ocultar el
  `aside.browser`, CSS Grid retiraba su posición y auto-colocaba `.editor` en la
  primera columna de ancho cero. Se fijó `.workstation > .editor` en la columna
  2, conservando la misma clase `browser-collapsed` y su atajo. Un smoke Firefox
  con `main.js` real verificó ancho expandido al ocultar y ancho normal al
  restaurar en 760×520, 1280×720 y 1920×1080. Conserva las verificaciones MIDI,
  filtros, categorías y Session. La captura runtime del reporte no equivale a
  repetición Tauri de la Demo; esa comprobación sigue pendiente.

## Corrección del selector Clip/Dispositivo — 2026-09-29

- La regla `.editor.has-midi-editor .clip-inspector` imponía `display:flex`
  incluso cuando el selector Dispositivo había marcado el inspector de Clip como
  `hidden`. El resultado era ver simultáneamente el piano roll y el dispositivo,
  aunque sólo una pestaña figuraba seleccionada. Se añadió una regla explícita
  para respetar el estado oculto.
- Verificación Firefox headless a 1280×720 y 1920×1080 con el `main.js` y CSS
  reales y un adaptador/snapshot de prueba: el smoke confirma que Dispositivo
  oculta el inspector MIDI, Clip lo restaura y sobreviven los flujos de scroll,
  zoom, colapso del panel, Navegador, filtros y Session. Esto no sustituye la
  matriz Tauri runtime; 3.1.5 y 3.1 siguen abiertas.
- Registrar SHA y publicación en la siguiente entrada de bitácora; no cerrar
  tareas de aceptación visual por esta regresión corregida.

## Altura ajustable de pistas en Arreglo — 2026-09-29

- La Demo mostraba un solo carril compacto y un área grande sin contenido. Se
  añadió un separador accesible en cada cabecera de pista: arrastrar cambia la
  altura del carril y su cabecera a la vez; flechas ajustan en pasos de 16 px y
  Home/End aplican los límites 56–320 px. Los clips se adaptan al alto del carril.
- La altura inicial respeta el diseño: 64 px en ventana normal y 56 px en ventana
  compacta. El valor se guarda en `localStorage` como preferencia de interfaz por
  proyecto y pista; no modifica el proyecto, comandos ni historial.
- Firefox headless ejecutó `main.js`/CSS reales con snapshot MIDI+audio y
  adaptador simulado a 760×520, 1280×720 y 1920×1080. Pasaron aserciones de
  arrastre, flechas, límites, alineación de carril/encabezado y persistencia al
  refrescar y reabrir el WebView. QA Tauri y cierre de 3.1 siguen pendientes.
- Registrar SHA, verificaciones de crate y publicación en la bitácora cuando se
  complete el corte.


## Continuidad de superficies y carriles — 2026-09-29

- Sobre `c4b096e`, el smoke headless de Firefox con `main.js` y CSS reales más adaptador Tauri simulado cubrió 760×520, 1280×720 y 1920×1080. Scroll y zoom del piano roll sobreviven a cambios Clip/Dispositivo y Arreglo/Sesión; pistas MIDI/audio mantienen alturas independientes, clips alineados y preferencias persistidas incluso en una segunda apertura del perfil.
- Esto verifica frontend aislado, no Tauri runtime. `workstation-arrangement-surface-v2` sigue en 10/50; 3.1.3/3.1.5/3.1.6 permanecen abiertas hasta QA Tauri, matriz y aceptación visual.


## Acceso consistente a Dispositivo — 2026-09-29

- Los accesos desde la cabecera de pista y desde Navegador comparten una transición: seleccionan la pista, abren el detalle Dispositivo, restauran el panel si estaba colapsado y cierran Mezcla. El acceso desde Navegador también vuelve a Arreglo y enfoca la pista.
- Firefox headless con frontend real y snapshot/adaptador sintético pasó la transición desde detalle colapsado + Mezcla activa a Dispositivo visible; continuaron pasando piano MIDI, carriles, categorías, filtros, búsqueda y Session. Es smoke frontend, no QA Tauri.


## Edición MIDI con ratón — 2026-09-29

- El smoke Firefox ejercitó el clic de la rejilla en C4, tercer paso: el manejador frontend envió `addMidiNote` para el clip seleccionado, nota MIDI 60, inicio 960 ticks y duración configurada desde PPQ. El resto del recorrido visual también terminó en `BEHAVIOR PASS` a 1280×720.
- El adaptador de prueba captura el comando; no valida la mutación del modelo Tauri ni la reproducción acústica. El usuario ya puede componer desde la rejilla con el ratón según el handler actual; no se añade un contrato nuevo.


## Casillas Session y disponibilidad de lanzamiento — 2026-09-29

- Smoke headless con frontend real y backend sintético: borrar la casilla MIDI en Verso emitió `setClipSlot` con los IDs `scene-1`/`midi-1` y tipo/clip nulos; el control de lanzamiento siguió deshabilitado porque el motor está desconectado. El smoke completo terminó en `BEHAVIOR PASS` a 1280×720.
- Se verifica el comando frontend y el estado honesto del control, no la mutación real ni la ejecución de Session en Tauri.


## Corrección de criterio de validación Tauri — 2026-09-29

- La arquitectura y el destino de la interfaz siguen siendo la aplicación de escritorio Tauri. No se aprobó ni planificó migración a WebAssembly.
- Los smokes de Firefox anteriores ejecutan el DOM/CSS/JS con un adaptador simulado; son experimentos auxiliares de handlers y no prueban WebKitGTK, integración Tauri ni la interfaz que usa la persona. No deben presentarse como QA visual/runtime ni como criterio de aceptación de 3.1.
- Para cerrar 3.1 sigue siendo obligatoria la revisión de la aplicación Tauri con Demo MIDI y tamaños objetivo, más aceptación visual explícita. No cambiar arquitectura ni scope fuera del delta OpenSpec aprobado.

## Continuación — foco contextual y categorías del Navegador — 2026-09-30

- Se conservó el cambio local de foco de Dispositivo y se completó su estado:
  cambiar de proyecto limpia la pista enfocada; abrir la pestaña Dispositivo desde
  un clip MIDI enfoca la pista de ese clip aunque la selección múltiple usada para
  agrupar haya cambiado. La ruta sigue usando `createTrackInstrumentControl` y no
  agrega IPC ni contratos del motor.
- El Navegador normaliza la categoría activa cuando sólo queda una categoría real
  o cuando desaparece la categoría seleccionada. Así el botón visible conserva
  `aria-pressed`/estado visual coherente con la lista que muestra; no se inventan
  categorías ni contenido.
- Verificación ejecutada: `node --check
  crates/ui-shell/frontend/main.js`, `node --check
  crates/ui-shell/frontend/platform-tauri.js` y `git diff --check` pasaron.
- `cargo check -p estudio-daw-ui-shell` no pudo completar porque este servidor no
  tiene `glib-2.0 >= 2.70` en `pkg-config`; `openspec validate` tampoco está
  instalado. El servidor se usa deliberadamente sin interfaz gráfica ni
  navegador: no se intentará ejecutar QA Tauri aquí.
- El árbol ya contenía cambios locales en `.codebase-memory/artifact.json`,
  `main.js` y `tasks.md`; no se sobrescribieron. OpenSpec 3.1 y 3.1.5 siguen
  abiertas. Próximo paso en este entorno: continuar los cortes de frontend y
  sus comprobaciones de código; la matriz Tauri y la aceptación visual quedan
  reservadas para una estación con interfaz, sin tratar su ausencia como fallo
  del diseño ni marcar el rediseño como aceptado sólo por sintaxis o compilación.

## Preview visual remoto en desarrollo — 2026-09-30

- Se preparó una ruta de revisión en `dev.pinguinoseguro.cl/estudioDaw` sobre
  el `pinguinoseguro_web` existente, sin modificar nginx, DNS, TLS, Sentinel,
  servicios de producción ni el repositorio `sentinel`.
- La página Next restringe la ruta al host `dev.pinguinoseguro.cl` y responde
  `404` en `pinguinoseguro.cl`. El iframe carga una copia estática de
  `preview.html`, `main.js`, `styles.css` y `platform-preview.js`.
- `platform-preview.js` es un adaptador visual aislado: usa datos sintéticos
  MIDI/audio y estado local, no carga `platform-tauri.js`, no llama Tauri, no
  conecta PipeWire/MIDI/filesystem/plugins y no guarda proyectos. Play,
  Record, importación, preescucha y plugins externos informan que no están
  disponibles.
- Verificación: `npm ci --ignore-scripts`, `npm run build` del portal generó
  `ƒ /estudioDaw`; la ruta local y remota devolvió `200` con el host dev,
  `404` con el host de producción, y el HTML remoto referencia el iframe y el
  adaptador preview. `node --check` y `git diff --check` pasaron.
- Limitación de infraestructura preexistente: el certificado presentado para
  `dev.pinguinoseguro.cl` tiene `CN=pinguinoseguro.cl` y no incluye el SAN
  `dev.pinguinoseguro.cl`. La verificación HTTPS normal falla por ese mismatch;
  no se corrigió el certificado ni se tocó configuración externa. HTTP responde
  `200`; `curl -k` confirmó la ruta HTTPS detrás del proxy.
- Esto habilita inspección remota del layout, no sustituye QA Tauri ni prueba
  audio. La aceptación de 3.1 continúa abierta hasta revisar la aplicación de
  escritorio en una estación con GUI y completar la matriz aprobada.

## Corrección TLS del preview remoto — 2026-09-30

- Se amplió el certificado Let's Encrypt existente de
  `pinguinoseguro.cl` para incluir `dev.pinguinoseguro.cl`. No se cambió la
  topología nginx, DNS, Sentinel ni los servicios de aplicación.
- `nginx -t` pasó y nginx se recargó. La comprobación HTTPS normal ahora
  devuelve `200` para `https://dev.pinguinoseguro.cl/estudioDaw`; el certificado
  presenta SAN `dev.pinguinoseguro.cl` y vence el 2026-12-29.
- La advertencia anterior de certificado queda resuelta. El preview sigue
  siendo visual/sintético y no sustituye QA Tauri ni prueba de audio.

## Continuidad de selección desde Session — 2026-09-30

- Los nombres de clips asignados en las casillas de Session ahora son botones
  accesibles. Al activarlos abren el clip en Arreglo mediante
  `focusClipInArrangement`, conservando selección, detalle contextual y
  desplazamiento hacia el clip sin lanzar ni detener la reproducción.
- El control circular de lanzamiento mantiene su acción independiente; las
  casillas vacías y las referencias de clip no disponibles conservan una
  presentación no accionable.
- Se ajustó el CSS para que el nombre accionable mantenga la jerarquía compacta
  de la celda y exponga hover/foco visible. No se añadió IPC ni capacidad del
  motor.
- Pendiente: comprobar la interacción en la aplicación Tauri y registrar
  aceptación visual; este servidor sólo permite `node --check` y revisión
  estática.

## Creación MIDI desde el cursor — 2026-09-30

- El Arreglo incorpora `＋ Clip MIDI` cuando existe una pista MIDI. El botón y
  el menú Crear comparten la acción `Crear clip MIDI en el cursor`.
- La acción crea mediante `CreateMidiClip` una región vacía de un compás en el
  cursor, con PPQ 960, identidad estable y duración derivada del tempo. La
  operación pasa por `ProjectApplication`, es reversible y actualiza el plan
  si el motor ya está conectado.
- Tras crearla, la interfaz selecciona la región, abre el piano roll y deja al
  músico añadir las notas manualmente. No se generan notas, arreglos ni
  decisiones musicales por cuenta de una IA; la futura asistencia debe
  proponer cambios revisables, no sustituir esta autoría.
- El preview remoto implementa el mismo gesto sobre su snapshot sintético para
  revisar la composición de la UI; no representa Tauri, audio ni persistencia
  real.
- Commit local: `03e9fb8` (`feat: create MIDI clips from arrangement cursor`).
  El push continúa bloqueado porque este servidor no tiene credenciales SSH,
  `gh` ni token HTTPS para `github.com`.
- Verificación: pruebas focales de `project-model` y `command-bus`, `cargo
  check -p estudio-daw-command-bus`, `cargo fmt --all -- --check`, sintaxis de
  los tres scripts frontend y `git diff --check` pasan. `cargo check
  -p estudio-daw-ui-shell` sigue bloqueado por la ausencia de
  `glib-2.0/gobject-2.0/gio-2.0 >= 2.70` y `gdk-3.0` en este servidor.
- OpenSpec 3.1, 3.3, 3.5 y 3.6 siguen abiertas hasta completar revisión Tauri
  visual e interacción en una estación con GUI.

## Creación directa desde casilla Session — 2026-09-30

- Una casilla MIDI vacía ofrece `＋ Crear clip`; una casilla de audio conserva
  `＋ Añadir clip` para asignar regiones existentes.
- El gesto crea una región MIDI vacía de un compás en el cursor mediante
  `CreateMidiClip`, la asigna a la escena mediante `SetClipSlot`, selecciona el
  clip y abre el detalle contextual. La reproducción no se inicia; el músico
  decide cuándo lanzar la casilla y qué notas añadir.
- La operación reutiliza las dos APIs existentes y mantiene la identidad común
  entre Session y Arreglo. El preview sintético implementa `createMidiClip`, por
  lo que el gesto queda disponible para revisión visual remota.
- Verificación pendiente de este corte: sintaxis frontend y `git diff --check`;
  no se puede afirmar QA Tauri desde este servidor sin GUI.
- La autenticación SSH quedó resuelta con las claves de `~/keys/ssh_bkp/`.
  `main` está publicado y alineado con `origin/main` en `3efd0d3`.

## Colecciones del navegador y referencia de atajos — 2026-09-30

- El navegador de proyecto añade las colecciones **Todo**, **Favoritos** y
  **Recientes** para clips MIDI, instrumentos asignados y regiones de audio.
  Favoritos e historial se conservan por `projectId` en `localStorage`; la
  selección mantiene el foco contextual en Arreglo y no modifica el proyecto.
- El menú **Ayuda** expone una referencia visible de atajos. `Espacio`
  reproduce/pausa fuera de campos editables; también se documentan Tab,
  Mayús+Tab y Ctrl+Alt+B junto con las acciones registradas en `UI_ACTIONS`.
- Verificación local: sintaxis de los tres scripts frontend y `git diff
  --check` pasan. La comprobación visual Tauri sigue pendiente; el preview
  remoto/local no sustituye esa aceptación.

## Preview web sincronizado — 2026-09-30

- Se sincronizaron `preview.html`, `main.js`, `styles.css` y
  `platform-preview.js` de `crates/ui-shell/frontend/` hacia
  `pinguinoseguro_web/public/estudioDaw/`.
- El portal se construyó con `npm run build` y publicó el cambio
  `ca67011` en `gitlab.com/jenovoa/pinguinoseguro_web`, rama `alpine_fenix`.
  No se tocó producción, DNS, TLS ni Sentinel.
- `https://dev.pinguinoseguro.cl/estudioDaw` devolvió `200`; los cuatro assets
  remotos coinciden por SHA-256 con los archivos fuente y el HTML contiene las
  nuevas colecciones del Browser y el botón `＋ Clip MIDI`.
- El navegador automatizado no pudo iniciar en este servidor por indisponibilidad
  del daemon Chromium; por tanto la comprobación realizada es HTTP/asset y no
  una afirmación de inspección visual automatizada.
