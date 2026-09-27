# Diagnóstico de la interfaz Tauri

Esta guía permite distinguir un problema de frontend, de IPC/modelo, de proyecto
guardado y de salida de audio. Los resultados deben anotarse con el SHA y el
comando exacto; no inferir funcionamiento de una etiqueta o de un render visual.

## Lanzar y observar errores

Desde la raíz del repositorio:

```sh
cargo run -p estudio-daw-ui-shell
```

La salida de esa terminal contiene errores Rust, fallos de inicialización de
PipeWire y mensajes del proceso Tauri. Los errores de frontend/IPC se muestran
en la consola de desarrollo del webview y en el aviso inferior de la ventana.
El texto del aviso incluye la cadena de error devuelta por el comando Tauri.

Para comprobar persistencia, guarda como JSON y abre ese mismo archivo en un
editor de texto. El formato es portable; buscar `"tracks"`, `"mixer"` y el
identificador de pista permite confirmar el estado escrito sin depender del
render de la ventana.

## Comprobaciones deterministas por área

```sh
cargo test -p estudio-daw-project-model -- --test-threads=1
cargo test -p estudio-daw-command-bus -- --test-threads=1
cargo test -p estudio-daw-application -- --test-threads=1
cargo test -p estudio-daw-ui-shell -- --test-threads=1
```

Las pruebas de `project-model` cubren migración y persistencia. `command-bus`
cubre validación, historial y eventos de dominio. `application` cubre ciclo de
apertura/guardado y undo. `ui-shell` cubre snapshots, resúmenes y scheduler MIDI.
Para verificar todo el repositorio:

```sh
cargo test --workspace -- --test-threads=1
cargo check --workspace
cargo fmt --all -- --check
git diff --check
```

## Qué comprobar en la ventana

1. Crear proyecto nuevo: aparece una pista MIDI vacía y se habilitan Guardar
   como, transporte e historial según el snapshot.
2. Pulsar `＋M` y `＋A`: el contador/lista cambia; la pista nueva participa en
   undo/redo y sobrevive a guardar, cerrar y abrir el JSON.
3. Abrir Demo MIDI: el snapshot reporta clips MIDI y Play intenta iniciar el
   instrumento elegido en la salida configurada. AudioBox/PipeWire se comprueba
   separadamente; el indicador “conectado” no mide latencia tecla→parlante.
4. Si una acción falla, copiar el texto completo del aviso, el comando de la
   terminal y `git rev-parse HEAD`; así se puede ubicar la frontera que falló.

## Límites actuales que no son fallos de UI

Crear una pista de audio crea y persiste una pista vacía; todavía no importa,
reproduce ni graba audio. El estado de mixer se persiste en el proyecto y se
incluye en snapshots, pero aún no controla el gain/pan real del motor ni tiene
controles interactivos en la vista. Session, Browser de medios, waveform,
routing, arm/monitor y medidores de pista siguen pendientes en
`openspec/changes/workstation-arrangement-surface-v2/tasks.md`.

## Plantilla para reportar un fallo

```text
SHA:
Acción exacta:
Resultado esperado:
Resultado observado:
Texto completo del aviso:
Salida pertinente de la terminal:
¿El JSON guardado conserva el cambio?:
```
