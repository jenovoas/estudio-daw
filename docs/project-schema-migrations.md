# Migraciones de `project.json`

La API de aplicación es la única ruta de apertura usada por la interfaz y la CLI. Al
cargar, `estudio-daw-project-model::load_project_json` normaliza el documento
antes de construir el modelo; el guardado posterior escribe el estado actual.

## Revisiones internas del formato persistido

`schema_version` identifica la forma de los datos de `project.json`; no es
una versión del producto, no representa una publicación inicial y no implica que la interfaz o una
capacidad estén terminadas.

| Entrada | Tratamiento |
| --- | --- |
| Sin `schema_version`, `estudio-daw.project.v0` o `0` | Migra a `estudio-daw.project.v1`; agrega `midi_clips` y `audio_clips` vacíos si faltan. |
| `1` | Normaliza el alias a `estudio-daw.project.v1`. |
| `estudio-daw.project.v1` | Migra instrumento MIDI, mezclador/color y roles/canales con valores conservadores. |
| `estudio-daw.project.v2` | Migra mezclador/color y roles/canales. |
| `estudio-daw.project.v3` | Migra roles/canales. |
| `estudio-daw.project.v4` | Migra fuentes incrustadas en la pista a `audio_sources`, añade `source_id` a las regiones que tienen fuente, crea una lista ordenada por pista de audio y añade colecciones vacías de escenas/casillas. Conserva desplazamientos, posición, duración, ganancia y desvanecimientos. |
| `estudio-daw.project.v5` | Lee la forma actual: fuente de medio separada, listas/regiones, escenas y casillas mediante referencias. |
| Cualquier versión futura/desconocida o tipo no textual | Rechaza la apertura; no se permite guardar accidentalmente descartando campos desconocidos. |

Las migraciones sólo actualizan el documento en memoria. La apertura nunca
sobrescribe el archivo fuente. El archivo adopta la forma actual únicamente
cuando el usuario lo guarda de forma explícita.

## Evolución segura

Cada revisión interna del formato debe tener una transformación determinista
desde la anterior y pruebas que verifiquen preservación de campos, valores predeterminados,
rechazo de formatos no admitidos y apertura a través de `ProjectApplication`. No se deben
introducir valores predeterminados silenciosos para campos obligatorios que puedan cambiar el
significado musical del proyecto.

Comprobación:

```bash
cargo test -p estudio-daw-project-model
cargo test -p estudio-daw-application
```
