# Proposal

## Why

Las instrucciones de entrada de este repositorio se diseñaron para un entorno anterior y no garantizan que cada agente consulte la memoria compartida ni identifique correctamente las herramientas disponibles hoy. Actualizarlas reduce divergencias entre agentes sin debilitar las reglas de arquitectura, seguridad, audio en tiempo real y trazabilidad propias de Estudio DAW.

## What Changes

- Actualizar `AGENTS.md` con el orden vigente de consulta del vault, los MCP actuales y la obligación de distinguir preparación local de operaciones realmente realizadas.
- Conservar intactos los límites arquitectónicos, los flujos de OpenSpec, la bitácora append-only y las reglas de evidencia específicas del proyecto.
- Retirar dependencias de herramientas heredadas que no estén disponibles en el entorno actual, únicamente tras comprobarlas.

## Capabilities

### New Capabilities

Ninguna.

### Modified Capabilities

Ninguna. Este cambio modifica instrucciones de trabajo, no requisitos de comportamiento del producto; se marca `skip_specs: true`.

## Impact

Afecta únicamente `AGENTS.md` y los metadatos del cambio OpenSpec. No cambia código, APIs, dependencias ni el estado operativo de la aplicación.
