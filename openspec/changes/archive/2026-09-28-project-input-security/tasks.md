# Tasks

## 1. Confianza local para código externo

- [x] 1.1 Añadir almacenamiento local de aprobaciones ligadas a ruta canónica e identidad del contenido; solicitar consentimiento antes del primer `spawn` standalone y cubrir aprobación, revocación, cambios de symlink/contenido y rutas inexistentes con pruebas deterministas y documentación del flujo.
- [x] 1.2 Dejar de aplicar el `wine_prefix` del proyecto y resolver el prefijo efectivo desde configuración local; conservar compatibilidad de lectura/escritura del campo heredado y verificarlo con pruebas de carga/guardado y documentación de migración.
- [x] 1.3 Aplicar el mismo control a bundles VST3 antes de cargar su código; verificar que un proyecto no aprobado no inicia el helper/plugin, que uno aprobado sí puede hacerlo y que una identidad modificada requiere consentimiento nuevo, con pruebas y documentación.

## 2. Seguridad del plan y callback de audio

- [x] 2.1 Validar los índices de entrada al validar el proyecto y compilar el plan, añadiendo defensa de rango al construir el nodo; cubrir índices válidos, fuera de rango y la garantía de que el callback no puede indexar fuera del buffer.
- [x] 2.2 Sustituir la consulta mutex de comandos Session por publicación acotada y buffers preasignados con reclamación fuera de RT; garantizar que replace/append/clear no asignan, liberan memoria ni esperan dentro del callback, y cubrir ciclo de vida, cola llena y reclamación con pruebas deterministas y documentación de ownership.

## 3. Errores recuperables de helpers y backend

- [x] 3.1 Recuperar o reportar como error estructurado el estado VST3 protegido por mutex envenenado; verificar que los comandos posteriores no provocan un segundo pánico y documentar la semántica de recuperación.
- [x] 3.2 Quitar el `Default` CPAL que contiene `expect` y hacer explícitos los errores de selección/ausencia de dispositivo; cubrir `None`/error sin dispositivo con pruebas y documentar el contrato de construcción.
- [x] 3.3 Confirmar que todo fallo de backend/helper durante Play devuelve diagnóstico, detiene o conserva detenido el transporte y no publica estado conectado; cubrir integración con pruebas de ciclo de vida.

## 4. Higiene del repositorio y verificación integrada

- [x] 4.1 Ignorar `.codebase-memory/graph.db.zst` y retirarlo del índice Git sin borrar la copia local; verificar `git check-ignore`, que el archivo local permanece y que Git deja de seguirlo.
- [x] 4.2 Ejecutar pruebas focales de confianza, callback, helper y arranque de audio; ejecutar formato, `cargo check --workspace`, `cargo test --workspace -- --test-threads=1`, validación OpenSpec estricta y `git diff --check`, registrando resultados y limitaciones de QA manual en la bitácora.
