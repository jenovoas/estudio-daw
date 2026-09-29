# Handoff de continuación — 2026-09-29

## Prioridad para la próxima sesión

La prioridad absoluta es la UI y el frontend. La persona usuaria busca para Linux
una experiencia muy cercana al flujo creativo y la distribución de Ableton Live;
considera que la versión actual todavía no se acerca y que el último cambio sólo
mejoró un poco la organización. No presentar el corte como aceptado ni pedirle
que valide esta versión como condición para volver a funciones: seguir trabajando
en una transformación visual sustancial hasta que la persona usuaria diga que se
acerca a lo que busca. No retomar tareas de funciones durante ese trabajo.

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

- Rama `main`, HEAD `4c532ce`. Los cambios permanecen locales y sin commit; el
  intento de crear un commit de resguardo fue interrumpido. Verificar
  `git status --short --branch` y `git log -3 --oneline` antes de continuar.
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

1. Leer este handoff, `AGENTS.md`, `README.md`,
   `openspec/changes/workstation-arrangement-surface-v2/{proposal.md,design.md,
   tasks.md}` y `docs/ui-architecture.md`; verificar rama, árbol e historial.
2. Abrir el Demo MIDI guardado por la persona usuaria en Tauri y revisar primero
   la composición de la ventana a tamaño estándar. Revisar también tamaño
   estrecho y las vistas Arreglo/Session y detalle/Mezclador.
3. Hacer un análisis y rediseño de interfaz de mayor alcance guiado por la
   jerarquía real de Live: Browser categorizado y usable, transporte/control
   global, Arrangement como canvas multipista amplio, Session como superficie
   alternativa equivalente, panel inferior contextual y mezclador integrado.
   Distinguir los contenidos que existen de los que faltan; no rellenar el
   navegador con categorías vacías. Evitar más retoques incrementales de CSS que
   no cambien el flujo perceptible.
4. Mantener la implementación dentro de OpenSpec y preservar el límite de no
   copiar código, marca ni recursos de Ableton. No dar 3.1 por completa por
   inspección de código/build: se necesita feedback explícito de la persona
   usuaria después de un cambio sustancial.
5. Sólo después de que la persona usuaria acepte la dirección visual, continuar
   otras tareas OpenSpec. 2.2, 2.3, 3.4, 3.5, 3.6, 4.x y 5.x siguen con QA o
   trabajo pendiente; no inferir que se completaron por compartir los cambios.

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
