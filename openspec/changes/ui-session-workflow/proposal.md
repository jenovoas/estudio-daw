# Proposal

## Why

Al iniciar el shell sin un proyecto, los controles de transporte aparecen deshabilitados y no existe una acción para crear una sesión, por lo que la interfaz no ofrece un primer paso claro. Además, su presentación actual se percibe como un panel web y no refleja la superficie compacta de una DAW que el usuario aprobó, tomando Ableton Live 12 Suite como referencia de feeling visual.

## What Changes

- Añadir creación de proyecto nuevo desde el shell, con una sesión MIDI vacía que permita guardar o abrir contenido, y una sesión Demo MIDI lista para reproducir una secuencia de prueba.
- Habilitar los controles de transporte al existir una sesión de proyecto.
- Ajustar la composición y el estilo de escritorio para una superficie de trabajo densa, oscura y orientada a música, con arreglo visible por compases y clips, jerarquía y acento cálido inspirados en Live 12.
- Conservar las acciones y capacidades actuales, sin presentar funciones de edición o grabación que aún no existen.

## Capabilities

### New Capabilities
- `desktop-project-workflow`: creación de una sesión nueva y disponibilidad del transporte desde la interfaz de escritorio.
- `desktop-workstation-ui`: presentación visual de la shell como una aplicación DAW de escritorio.

### Modified Capabilities

## Impact

Afecta el comando de aplicación Tauri, el puente de plataforma, el estado inicial de la interfaz, estilos de la shell y documentación de la experiencia de escritorio. No agrega dependencias externas ni modifica el formato serializado del proyecto.
