# Especificación: scripting y agente musical

## Propósito

El usuario debe poder controlar el proyecto mediante un lenguaje musical y mediante una LLM con herramientas seguras.

## Requisitos

### Requisito: comandos declarativos
El lenguaje DEBE poder describir transporte, pistas, clips, instrumentos, MIDI y automatización básica.

#### Escenario: crear arreglo
- DADO un script válido
- CUANDO el usuario lo evalúa
- ENTONCES se crea o actualiza el arreglo correspondiente sin modificar audio fuente de forma destructiva.

### Requisito: validación antes de ejecutar
Los scripts y operaciones del agente DEBEN validarse antes de aplicarse al proyecto.

#### Escenario: operación inválida
- DADO que el agente solicita una pista o dispositivo inexistente
- CUANDO se valida la operación
- ENTONCES se rechaza con un error estructurado y no se modifica el proyecto.

### Requisito: preview y undo
Las operaciones del agente DEBEN poder previsualizarse y deshacerse.

#### Escenario: sugerencia de producción
- DADO que el usuario pide más energía en el coro
- CUANDO el agente genera una propuesta
- ENTONCES el sistema muestra las operaciones concretas, permite aceptarlas y ofrece undo agrupado.
