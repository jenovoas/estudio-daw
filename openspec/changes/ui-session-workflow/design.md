# Diseño

## Context

La shell Tauri ya separa UI y core mediante comandos IPC y renderiza snapshots compactos del proyecto. El transporte necesita una sesión `ProjectApplication`; sin ella las acciones de proyecto y transporte se desactivan. La primera iteración ya añadió sesión nueva y compactó colores/controles, pero conserva una pila de paneles tipo dashboard y no ofrece material MIDI audible al empezar.

## Goals / Non-Goals

**Goals:**
- Crear una sesión nueva válida dentro de `ProjectApplication` sin requerir archivo previo.
- Parar el host de audio antes de sustituir una sesión.
- Hacer evidente el flujo Nuevo/Abrir/Guardar/Demo MIDI y que transporte depende de una sesión.
- Presentar una regla de compases y lanes de pista que posicionan visualmente los clips MIDI recibidos en el snapshot.
- Dar a la ventana una composición compacta de estación de trabajo oscura, usando el feeling de Live 12 como referencia cromática y de densidad.

**Non-Goals:**
- Crear una vista de arreglo editable ni herramientas para insertar o editar notas y clips.
- Cambiar el modelo de proyecto o agregar grabación live, monitorización o reproducción de clips de audio.
- Copiar logos, iconos, tipografía propietaria u otros activos de Ableton.

## Decisions

- El comando `new_project` crea un modelo v2 sin ruta, tempo 120 BPM, compás 4/4 y una pista MIDI vacía con SineSynth. La primera acción Guardar como le asigna ruta usando el flujo ya existente.
- El cambio de sesión toma locks en el mismo orden que el resto del adaptador (aplicación y luego host de audio), detiene reproducción y publica un snapshot de la nueva sesión.
- El frontend reutiliza `renderSnapshot` para activar capacidades y actualizar arreglo/controles; el comando Demo MIDI construye una sesión con toma mediante `ProjectCommand::AttachMidiTake`.
- La demo usa SineSynth para que reproduzca sin SoundFont externo y muestra una melodía breve; es una prueba de la ruta de salida, no una preescucha del piano licenciado de Analog Lab.
- El área central ocupa el espacio disponible de ventana: cabeceras fijas al lado de lanes con regla de 16 compases; preferencias de audio pasan a un panel desplegable.
- El rediseño permanece en HTML/CSS nativo del bundle Tauri: barra superior compacta, transporte agrupado, tipografía y superficies más densas, colores neutros oscuros y acento naranja cálido. No se introduce framework visual ni recursos externos.

## Risks / Trade-offs

- Una sesión nueva no contiene clips y Play produce silencio: la UI debe indicarlo con claridad y no sugerir contenido sonoro.
- El prototipo no incluye editor de arreglo; para una prueba audible se debe abrir un proyecto que ya tenga clips MIDI.
- Los cambios de estilo pueden reducir espacio en ventanas estrechas → conservar el ancho mínimo existente y ajustar la fila de preferencias con breakpoints.
