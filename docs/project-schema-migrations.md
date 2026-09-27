# Migraciones de `project.json`

La API de aplicación es la única ruta de apertura usada por la UI y la CLI. Al
cargar, `estudio-daw-project-model::load_project_json` normaliza el documento
antes de construir el modelo; el guardado posterior escribe el estado actual.

## Revisiones internas del formato persistido

`schema_version` identifica la forma de los datos de `project.json`; no es
una versión del producto, no representa un release y no implica que la UI o una
capacidad estén terminadas.

| Entrada | Tratamiento |
| --- | --- |
| Sin `schema_version`, `estudio-daw.project.v0` o `0` | Migra a `estudio-daw.project.v1`; agrega `midi_clips` y `audio_clips` vacíos si faltan. |
| `1` | Normaliza el alias a `estudio-daw.project.v1`. |
| `estudio-daw.project.v1` | Migra instrumento MIDI, mixer/color y roles/canales con valores conservadores. |
| `estudio-daw.project.v2` | Migra mixer/color y roles/canales. |
| `estudio-daw.project.v3` | Migra roles/canales. |
| `estudio-daw.project.v4` | Lee la forma actual: medio/rol, conteo de canales, mixer/color, clips y procedencia disponibles en el modelo. |
| Cualquier versión futura/desconocida o tipo no textual | Rechaza la apertura; no se permite guardar accidentalmente descartando campos desconocidos. |

Las migraciones sólo actualizan el documento en memoria. La apertura nunca
sobrescribe el archivo fuente. El archivo adopta el schema actual únicamente
cuando el usuario lo guarda de forma explícita.

## Evolución segura

Cada revisión del schema debe tener una transformación determinista desde la
anterior y pruebas que verifiquen preservación de campos, defaults, rechazo de
formatos no soportados y apertura a través de `ProjectApplication`. No se deben
introducir defaults silenciosos para campos obligatorios que puedan cambiar el
significado musical del proyecto.

Comprobación:

```bash
cargo test -p estudio-daw-project-model
cargo test -p estudio-daw-application
```
