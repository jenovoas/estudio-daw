# audio-render-chain Specification

## Purpose

Define el comportamiento determinista de una cadena DSP modificada en el sitio y el contrato explícito para mezclar fuentes de instrumentos en paralelo.

## ADDED Requirements

### Requirement: los planes de renderizado definen la semántica explícita de una cadena en el sitio
El `RenderPlan` actual MUST procesar los nodos en orden de inserción sobre un mismo bloque de audio modificado en el sitio. El constructor MUST NOT exponer una API de conexiones que sugiera ramas de grafo independientes o suma automática.

#### Scenario: cadena de procesamiento ordenada
- **WHEN** multiple nodes are added and a block is processed
- **THEN** each node receives the block after the preceding node has processed it
- **AND** processing order is deterministic and matches insertion order

### Requirement: las fuentes de instrumentos en paralelo se mezclan de forma explícita
Las fuentes en paralelo MUST combinarse mediante un mezclador que ofrezca memoria temporal preasignada independiente para cada fuente y sume sus muestras antes de escribir el bloque de salida.

#### Scenario: dos fuentes de instrumento simultáneas
- **WHEN** a mixer renders two source nodes for one block
- **THEN** output contains the sum of both source signals
- **AND** the callback performs no allocation to mix them
