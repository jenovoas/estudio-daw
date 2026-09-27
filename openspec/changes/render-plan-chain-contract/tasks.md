# Tareas

## 1. Alinear la API y la implementación

- [x] 1.1 Eliminar el almacenamiento de conexiones/dependencias del DAG y ofrecer un constructor de cadenas infalible que respete el orden de inserción.
- [x] 1.2 Actualizar la construcción del plan en la CLI para usar el orden de inserción y mantener la suma de fuentes en nodos mezcladores explícitos.
- [x] 1.3 Actualizar la documentación del plan de renderizado: retirar afirmaciones de grafo/DAG y describir los contratos de cadena/mezclador.
- [x] 1.4 Añadir pruebas de regresión del orden determinista de la cadena y de la mezcla explícita de fuentes.

## 2. Verificar y registrar

- [x] 2.1 Ejecutar `cargo fmt --all`, `cargo test --workspace -- --test-threads=1`, `cargo check --workspace`, la validación estricta de OpenSpec y `git diff --check`.
- [x] 2.2 Añadir el SHA verificado y el resultado al traspaso de AGENTS/bitácora.
