# Propuesta: estación de trabajo inspirada en Ableton

## Motivo

La interfaz actual es un prototipo parcial, no una estación de trabajo terminada. Una pasada anterior marcó como completas tareas de presentación sin entregar el flujo Session, la edición real ni un sistema utilizable de pistas de audio. Ableton Live 12 es la referencia principal y prioritaria de interfaz y flujo creativo. Ardour queda como consulta técnica secundaria para grabación, ruteo y señal de audio; no guía la composición visual ni la interacción. Los manuales oficiales respaldan el análisis y los requisitos verificables para Estudio DAW.

## Cambios

- Establecer un único modelo de proyecto con revisión interna del formato, operaciones reversibles y pistas ordenadas de MIDI, instrumento, audio, bus y master; incluir escenas, casillas, fuentes, listas de reproducción no destructivas, regiones y estado de mezcla.
- Entregar superficies Session y Arrangement conectadas sobre esas mismas pistas, con lanzamiento real de clips/escenas, edición de arreglo, edición contextual MIDI/audio, navegador, importación de medios y operaciones de mezcla respaldadas de extremo a extremo.
- Proveer pistas de audio con procedencia real de fuentes, canales, flujo entrada/procesamiento/panorama/ganancia/medición/salida, importación y reproducción, además de un flujo completo de grabación/monitorización antes de habilitar esos controles.
- Crear un sistema localizable de comandos, menús y atajos que cubra las familias de menús y flujos de Ableton Live 12, junto con capacidades adicionales previstas para Estudio DAW. Entregarlo por etapas sin marcar como completas las acciones diferidas ni mostrarlas activas.
- Seguir la jerarquía creativa de Live 12, su relación entre Session y Arrangement, el navegador, el lanzamiento de clips, la edición contextual y el mezclador integrado. Consultar Ardour sólo para detalles de ingeniería de audio ausentes o poco especificados en Ableton; no adoptar su jerarquía visual ni su flujo de edición. No copiar marca ni recursos protegidos.
- Mantener veraz cada capacidad: las ediciones del dominio usan comandos tipados, las operaciones reversibles tienen deshacer/rehacer, el trabajo de plataforma/medios permanece en adaptadores o procesos auxiliares y la devolución de audio sigue acotada y sin asignaciones de memoria.
- Migrar proyectos existentes de forma aditiva y demostrar que se conservan las identidades de pistas/clips, la procedencia de medios y las posiciones musicales al abrir y guardar.

## Capacidades

### Capacidades nuevas

Ninguna se separa en esta propuesta.

### Capacidades modificadas

- `desktop-workstation-ui`: ampliar el contrato de composición de DAW, controles visibles, arreglo y estados de sesión.

## Alcance técnico

**Prioridad de ejecución desde el 2026-09-29:** la siguiente intervención se
limita al rediseño de frontend de 3.1, según el delta de planificación para Luna
en `design.md` y las subtareas 3.1.1–3.1.6. No amplía el motor ni los contratos
IPC. Las demás funciones quedan pausadas hasta aceptación explícita del usuario
de la dirección visual. Esta precisión conserva el alcance general descrito
abajo; no declara la interfaz implementada ni aceptada.

Afecta `project-model`, `command-bus`, `application`, `ui-shell` y el motor de audio; requiere migraciones compatibles, comandos tipados, planificador de clips, procesos auxiliares de medios de audio y una matriz rastreable entre menús/opciones y operaciones. La interfaz queda abierta hasta que cada superficie y ruta anunciada esté implementada y verificada.
