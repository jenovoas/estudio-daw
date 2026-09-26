# Fixture DAWproject mínimo

Este directorio representa el contenido desempaquetado de un `.dawproject` de
prueba. También se incluye `../minimal.dawproject`, un contenedor ZIP real.
DAWproject es un contenedor ZIP con `project.xml`, `metadata.xml` y
archivos opcionales de media/plugins.

La fixture cubre únicamente:

- tempo 92 BPM;
- compás 4/4;
- una pista MIDI con tres notas;
- una pista de audio con un evento sin archivo de media real;
- metadata mínima.

El archivo `project.json` es el modelo canónico interno de Estudio DAW para la
misma sesión. No debe confundirse con el XML de intercambio.

Objetivo del primer adaptador:

1. importar `project.xml` a `project.json`;
2. preservar tempo, compás, pistas, notas, clips y metadata disponible;
3. registrar campos no representables como warnings;
4. exportar nuevamente sin afirmar equivalencia perfecta;
5. comparar el resultado mediante un reporte de pérdidas.

La fixture valida actualmente contra `Project.xsd` y `MetaData.xsd`. Deliberadamente
no contiene audio ni estado de plugins; esos aspectos se probarán en fixtures
posteriores.
