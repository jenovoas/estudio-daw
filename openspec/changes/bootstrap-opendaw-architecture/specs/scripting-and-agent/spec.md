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

## ADDED Requirements

### Requirement: herramientas musicales declarativas y tipadas
El scripting y el agente MUST expresar operaciones musicales mediante
comandos de dominio tipados, validados fuera del callback de audio.

#### Scenario: una orden referencia una pista inexistente
- WHEN se valida una orden de script o del agente
- THEN se devuelve un error estructurado y no se modifica la sesión.

### Requirement: propuesta de agente con control del usuario
Una propuesta generada por el profesor/productor IA MUST ser inspeccionable,
previsualizable y reversible antes de afectar el proyecto.

#### Scenario: el usuario revisa una sugerencia
- WHEN el agente propone cambios al arreglo o procesamiento
- THEN la UI presenta un changeset concreto y sólo lo aplica tras aceptación explícita.
