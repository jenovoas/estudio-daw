# Especificación: modelo de proyecto

## Propósito

El proyecto debe conservar una sesión musical reproducible, editable por UI, script o agente.

## Frontera entre formato interno e intercambio

El formato interno canónico será versionado y orientado a reproducción, undo/redo,
proxies, análisis y notas scale-aware. DAWproject será un adaptador de intercambio:
es un contenedor ZIP con XML y puede representar audio, notas, automatización,
routing, escenas y estado de plugins, pero no debe convertirse en la estructura
interna de tiempo real.

La importación/exportación DEBE producir un reporte de pérdidas con campos
convertidos, omitidos, aproximados o no soportados. Los datos propios de Estudio
DAW —manifiesto simbólico, procedencia de análisis, aprendizaje, proxies, escala
enriquecida y operaciones del agente— se conservan en extensiones internas y no se
suponen portables a otras DAW.

La fixture inicial se encuentra en `tests/fixtures/dawproject/` y contiene un
`.dawproject` real, sus XML desempaquetados y el `project.json` canónico equivalente.
Los XML validan contra `Project.xsd` y `MetaData.xsd` de la especificación oficial.

## Requisitos

### Requisito: fuentes y proxies de media

El modelo DEBE distinguir el archivo original de cualquier proxy derivado. Un
proxy debe conservar duración, timebase, canales y alineación de la fuente, y debe
ser regenerable mediante sus parámetros y hash de procedencia.

#### Escenario: edición con proxy

- DADO un original pesado y un proxy vigente
- CUANDO la sesión está en modo `proxy` o `auto`
- ENTONCES la edición y el preview pueden leer el proxy
- Y el render final conserva la referencia al original y lo usa cuando está disponible.

#### Escenario: proxy obsoleto

- DADO que cambió el hash del original o el perfil de generación
- CUANDO se abre la sesión
- ENTONCES el proxy se marca `stale` y puede regenerarse sin borrar el original.

### Requisito: estado serializable
El proyecto DEBE guardar transporte, pistas, clips, conexiones, instrumentos, parámetros, automatizaciones y referencias a artefactos.

#### Escenario: cerrar y reabrir
- DADO un proyecto guardado
- CUANDO se abre en otra sesión del programa
- ENTONCES se reconstruye el estado musical sin depender de una sesión viva anterior.

### Requisito: operaciones reversibles
Cada mutación DEBE representarse como comando y evento con undo/redo.

#### Escenario: cambio generado por IA
- DADO que un agente propone duplicar y procesar una guitarra
- CUANDO el usuario acepta el cambio
- ENTONCES queda registrado como una operación atribuible al agente
- Y puede deshacerse como una unidad.

### Requisito: versionado de schema
El formato DEBE incluir versión y migraciones explícitas.

#### Escenario: proyecto antiguo
- DADO un proyecto con una versión anterior
- CUANDO se abre
- ENTONCES el programa informa la migración aplicada o rechaza el formato con un diagnóstico accionable.
