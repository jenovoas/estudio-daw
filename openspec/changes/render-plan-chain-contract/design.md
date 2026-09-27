# Diseño

## Modelo del plan

`RenderPlanBuilder` guarda los nodos en orden de inserción. `build()` transfiere la lista ordenada a un plan de topología inmutable; `process()` llama a cada nodo en orden sobre el mismo búfer intercalado. Un nodo fuente reemplaza o llena el bloque, y los nodos de inserción/master posteriores lo transforman en el sitio. Es una cadena, no un DAG.

## Fuentes en paralelo

Los instrumentos en paralelo deben conectarse mediante un mezclador explícito, como `InstrumentMixerNode`, que renderiza cada fuente en memoria temporal preasignada, suma las muestras y escribe el bloque resultante. Una futura implementación de grafo requiere contratos independientes de entrada/salida de búferes de audio y un plan de asignación de búferes compilado de antemano.

## Impacto en la API

Quitar `GraphError`, las listas de dependencias por nodo, la ordenación topológica y `connect`. `build()` pasa a ser infalible. Actualizar las llamadas del repositorio para expresar la cadena prevista únicamente mediante el orden de inserción.

## Verificación

Comprobar que dos nodos de prueba aditivos se ejecuten en orden de inserción sobre el mismo bloque; conservar las pruebas del mezclador para demostrar que la suma en paralelo es explícita y no asigna memoria. Ejecutar formato, pruebas/verificación del espacio de trabajo, validación estricta de OpenSpec y revisión de diferencias.
