# audio-render-chain Specification

## Purpose

Define el comportamiento determinista de una cadena DSP modificada en el sitio y el contrato explícito para mezclar fuentes de instrumentos en paralelo.

## Requirements

### Requirement: los planes de renderizado definen la semántica explícita de una cadena en el sitio
El `RenderPlan` actual MUST procesar los nodos en orden de inserción sobre un mismo bloque de audio modificado en el sitio. El constructor MUST NOT exponer una API de conexiones que sugiera ramas de grafo independientes o suma automática.

#### Scenario: cadena de procesamiento ordenada
- **WHEN** se añaden varios nodos y se procesa un bloque
- **THEN** cada nodo recibe el bloque que procesó el nodo anterior
- **AND** el orden de procesamiento es determinista y coincide con el orden de inserción

### Requirement: las fuentes de instrumentos en paralelo se mezclan de forma explícita
Las fuentes en paralelo MUST combinarse mediante un mezclador que ofrezca memoria temporal preasignada independiente para cada fuente y sume sus muestras antes de escribir el bloque de salida. La llamada de retorno MUST NOT asignar ni liberar memoria dinámica, adquirir locks (incluidos los intentos no bloqueantes), realizar I/O o ejecutar operaciones cuyo tiempo no esté acotado. Los datos derivados del proyecto MUST validarse antes de activar el plan; un índice inválido MUST producir un diagnóstico antes de la llamada de retorno, y ningún acceso del callback puede indexar fuera de un buffer.

#### Scenario: dos fuentes de instrumento simultáneas
- **WHEN** un mezclador procesa dos nodos de origen en un bloque
- **THEN** la salida contiene la suma de ambas señales de origen
- **AND** la llamada de retorno no reserva ni libera memoria para mezclarlas

#### Scenario: comando de Session cambia las fuentes activas
- **WHEN** la llamada de retorno recibe un comando preparado para reemplazar, añadir o retirar fuentes de Session
- **THEN** aplica el cambio con buffers acotados y memoria ya reservada
- **AND** conserva la propiedad de buffers retirados fuera del callback para que no se asignen ni liberen allí

#### Scenario: configuración contiene un índice de canal inválido
- **WHEN** se compila un plan con un canal fuera del rango admitido por la entrada estéreo
- **THEN** la compilación devuelve un diagnóstico o normaliza el valor antes de activar el plan
- **AND** el callback nunca indexa el buffer de entrada con ese valor inválido
