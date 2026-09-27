# Proposal

## Why

`RenderPlanBuilder::connect` sugiere un grafo con ruteo, pero los nodos de audio sólo reciben en secuencia un bloque mutable compartido. Por ello, las fuentes en ramas distintas se sobrescriben entre sí y la API describe un comportamiento que el procesador no puede ofrecer.

## What Changes

- **INCOMPATIBLE** Quitar `connect` y la compilación de DAG/ciclos de la API actual del procesador.
- Definir `RenderPlan` como una cadena de efectos en el sitio que respeta el orden de inserción.
- Mantener explícita la suma de fuentes paralelas en nodos mezcladores con búferes temporales preasignados.
- Actualizar la secuencia de la CLI y la documentación de arquitectura; añadir una prueba de regresión de la semántica secuencial de la cadena.

## Capabilities

### New Capabilities

- `audio-render-chain`: contrato del orden explícito de procesamiento y la mezcla en paralelo del motor `RenderPlan` actual.

### Modified Capabilities

Ninguna. `openspec/specs/` aún no contiene especificaciones principales sincronizadas; este cambio establece un contrato canónico de capacidad.

## Impact

El cambio afecta `crates/audio-engine`, la construcción de su plan en la CLI y `docs/audio-render-plan.md`. No modifica las asignaciones de memoria de la devolución de audio ni el comportamiento de serialización del proyecto.
