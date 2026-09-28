# Diseño

## Contexto

Ver `proposal.md` para la motivación y `specs/` para los contratos observables. El proyecto persiste referencias a aplicaciones standalone, prefijos Wine y plugins; el runtime puede actuar sobre ellas al recibir Play. El callback también consume comandos de Session y mezcla entradas de canal con índices derivados del proyecto. La ruta CPAL expone `new() -> Result`, pero además implementa `Default` mediante `expect`.

## Objetivos / fuera de alcance

**Objetivos:** centralizar la aprobación local de código externo, desacoplar el entorno Wine del documento, y asegurar que la publicación/aplicación de comandos de Session respete el contrato del callback incluso al retirar buffers.

**Fuera de alcance:** sandboxing del código de plugins una vez aprobado, firma de editor/autor, rehacer la política general de acceso a medios, e incorporar los once hallazgos bajos no descritos individualmente en el resumen de auditoría.

## Decisiones

### Aprobación local ligada al destino ejecutable

La aprobación vive en configuración de usuario, nunca en `project.json`. El flujo de selección/inspección resuelve la ruta con `canonicalize`, presenta al usuario el destino real y guarda una identidad que incluya ruta canónica y huella del contenido ejecutable. Antes de `spawn` o carga del plugin se vuelve a resolver y comprobar esa identidad; un cambio de symlink, destino o contenido vuelve a requerir aprobación. La lista ofrece revocación desde configuración local. Se elige esta política por sobre aceptar cualquier ruta absoluta o confiar en el consentimiento de abrir el proyecto: la aprobación debe ser explícita y específica al código que se ejecutará.

La misma puerta cubre aplicaciones standalone y bundles VST3 referenciados por proyecto. Para un bundle, la huella representa de forma determinista sus archivos ejecutables relevantes, no el estado mutable del proyecto. El binario Wine se resuelve desde la instalación local de la aplicación; el proyecto sólo identifica el `.exe` aprobado.

### El proyecto no define `WINEPREFIX`

El prefijo efectivo procede de configuración local asociada al instrumento/ejecutable. El campo serializado `wine_prefix` se mantiene legible y escribible durante la transición por compatibilidad, pero se ignora al construir el proceso. No se copian variables de entorno arbitrarias del proyecto.

### Transferencia de comandos Session con propiedad fuera de RT

Se reemplaza el mutex consumido desde el callback por una cola acotada de comandos preparados fuera de RT. Las capacidades de buffers se reservan antes de iniciar el stream y tienen límite explícito. Al reemplazar, añadir o limpiar fuentes, el callback mueve/swap-ea buffers ya asignados y devuelve buffers retirados a una cola de reclamación inversa; un productor/controlador no-RT los recicla o destruye. Ningún `Vec` temporal, crecimiento ni destructor de colección se ejecuta en RT. Cola llena conserva una política explícita de error/reintento fuera del callback.

Los índices de canales se validan al leer/validar el proyecto y de nuevo al compilar el plan. La compilación rechaza valores inválidos con diagnóstico accionable; la construcción del nodo limita defensivamente cualquier valor que aun así llegara a una ruta interna. Así el callback opera sólo sobre canales 0–1.

### Errores del helper y de CPAL se devuelven por la frontera existente

Los locks compartidos del helper recuperan el estado envenenado con `into_inner` cuando es seguro continuar; si el estado del plugin no puede usarse, el protocolo devuelve un error de operación sin cerrar el helper por un segundo pánico. La selección de dispositivo CPAL devuelve `Result`/`Option` interpretados por el caller; se elimina el `Default` que oculta un `expect` y se mantienen errores claros cuando no hay dispositivo.

### El índice generado de codebase-memory no se versiona

Se añade `.codebase-memory/graph.db.zst` a `.gitignore` y se retira únicamente del índice Git; el archivo local no se borra. Se conserva `artifact.json` si el flujo del repositorio aún lo necesita como metadato pequeño.

## Riesgos y compensaciones

- [Primera carga externa añade una pausa de consentimiento] → el permiso se recuerda localmente por identidad y puede revocarse; no se vuelve a preguntar si el binario no cambió.
- [Hash de bundle puede ser costoso] → se calcula en el hilo de control al aprobar y se verifica antes de carga, nunca en RT; puede cachearse con metadatos de identidad y volver a calcularse si éstos cambian.
- [Cola de reclamación RT llena] → limitar el número de reemplazos pendientes y mantener el comando fuera del callback hasta que haya capacidad; no liberar buffers como fallback dentro de RT.
- [Proyectos Wine heredados dejan de controlar su prefijo] → conservar el campo en serialización y ofrecer migración/selección de prefijo local durante el flujo explícito de aprobación.

## Plan de migración

1. Añadir la política local de confianza y adaptar los flujos standalone/VST3; proyectos existentes mostrarán aprobación al primer uso y sus prefijos persistidos no se aplicarán.
2. Hacer bounded la publicación y reclamación de buffers de Session y validar canales antes de activar planes.
3. Endurecer errores del helper/CPAL y actualizar `.gitignore`; retirar el blob generado del índice conservando la copia local.
4. Ejecutar validación de proyecto/plan, pruebas deterministas del protocolo de confianza y callback, y regresiones de reproducción VST3/standalone.

## Preguntas abiertas

Ninguna decisión de seguridad queda diferida. La identificación individual de los once hallazgos bajos requiere el listado completo de la auditoría y permanece fuera de este cambio.
