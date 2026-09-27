# audio-render-chain Specification

## Purpose

Define el comportamiento determinista de una cadena DSP modificada en el sitio y el contrato explícito para mezclar fuentes de instrumentos en paralelo.

## ADDED Requirements

### Requirement: los planes de renderizado definen la semántica explícita de una cadena en el sitio
El `RenderPlan` actual MUST procesar los nodos en orden de inserción sobre un mismo bloque de audio modificado en el sitio. El constructor MUST NOT exponer una API de conexiones que sugiera ramas de grafo independientes o suma automática.

#### Scenario: cadena de procesamiento ordenada
- **WHEN** se añaden varios nodos y se procesa un bloque
- **THEN** cada nodo recibe el bloque que procesó el nodo anterior
- **AND** el orden de procesamiento es determinista y coincide con el orden de inserción

### Requirement: las fuentes de instrumentos en paralelo se mezclan de forma explícita
Las fuentes en paralelo MUST combinarse mediante un mezclador que ofrezca memoria temporal preasignada independiente para cada fuente y sume sus muestras antes de escribir el bloque de salida.

#### Scenario: dos fuentes de instrumento simultáneas
- **WHEN** un mezclador procesa dos nodos de origen en un bloque
- **THEN** la salida contiene la suma de ambas señales de origen
- **AND** la llamada de retorno no reserva memoria para mezclarlas
