# Migraciones de `project.json`

La API de aplicación es la única ruta de apertura usada por la UI y la CLI. Al
cargar, `estudio-daw-project-model::load_project_json` normaliza el documento
antes de construir el modelo; el guardado posterior escribe el estado actual.

## Versiones admitidas

| Entrada | Tratamiento |
| --- | --- |
| Sin `schema_version`, `estudio-daw.project.v0` o `0` | Migra a `estudio-daw.project.v1`; agrega `midi_clips` y `audio_clips` vacíos si faltan. |
| `1` | Normaliza el alias a `estudio-daw.project.v1`. |
| `estudio-daw.project.v1` | Lee directamente. |
| Cualquier versión futura/desconocida o tipo no textual | Rechaza la apertura; no se permite guardar accidentalmente descartando campos desconocidos. |

Las migraciones sólo actualizan el documento en memoria. La apertura nunca
sobrescribe el archivo fuente. El archivo adopta el schema actual únicamente
cuando el usuario lo guarda de forma explícita.

## Evolución segura

Cada nuevo schema debe tener una transformación determinista desde la versión
anterior y pruebas que verifiquen preservación de campos, defaults, rechazo de
versiones no soportadas y apertura a través de `ProjectApplication`. No se deben
introducir defaults silenciosos para campos obligatorios que puedan cambiar el
significado musical del proyecto.

Comprobación:

```bash
cargo test -p estudio-daw-project-model
cargo test -p estudio-daw-application
```
