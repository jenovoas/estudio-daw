# Diagnóstico de la interfaz Tauri

Esta guía permite distinguir un problema de interfaz web, de IPC/modelo, de proyecto
guardado y de salida de audio. Los resultados deben anotarse con el SHA y el
comando exacto; no inferir funcionamiento de una etiqueta o de una imagen estática.

## Lanzar y observar errores

Desde la raíz del repositorio:

```sh
cargo build -p vst3-host --bins
cargo run -p estudio-daw-ui-shell
```

Los ejecutables `vst3-host-helper` y `vst3-host-probe` deben distribuirse junto
al ejecutable de la aplicación. Si falta alguno, la pista VST3 informa el error
de carga/inspección y no sustituye el instrumento.

Para abrir los controles nativos de un VST3, el transporte debe estar en Play o
pausa: el helper sólo vive mientras el motor está conectado. El botón CONTROLES
de la pista envía `set_vst3_editor`; la primera apertura puede tardar y el audio
de esa pista puede interrumpirse unos segundos mientras el plugin crea su
ventana. Cerrar la ventana del plugin o pulsar OCULTAR usa `CloseGui`. Esto no
sustituye una prueba de Analog Lab en la ventana Tauri.

La aplicación Tauri corre en la sesión Wayland (Hyprland). Analog Lab y los VST
de Wine/yabridge dibujan su editor por XWayland (`DISPLAY`, hoy `:0` con
`xwayland:enabled`). El helper se conecta a ese puente; el contrato VST3 Linux
sigue siendo `X11EmbedWindowID`. Si `DISPLAY` está vacío, el aviso pide activar
XWayland en Hyprland.

La salida de esa terminal contiene errores Rust, fallos de inicialización de
PipeWire y mensajes del proceso Tauri. Los errores de interfaz web/IPC se muestran
en la consola de desarrollo del visor web y en el aviso inferior de la ventana.
El texto del aviso incluye la cadena de error devuelta por el comando Tauri.

Para comprobar persistencia, guarda como JSON y abre ese mismo archivo en un
editor de texto. El valor `estudio-daw.project.v5` identifica la revisión
interna del formato JSON; no es una versión del producto ni una publicación. Busca
`"tracks"`, `"audio_sources"`, `"audio_playlists"`, `"audio_clips"`,
`"scenes"` y `"clip_slots"`. Las regiones/slots guardan identificadores hacia
los clips; no deben contener una segunda copia de sus notas o fuente.

## Comprobaciones deterministas por área

```sh
cargo test -p estudio-daw-project-model -- --test-threads=1
cargo test -p estudio-daw-command-bus -- --test-threads=1
cargo test -p estudio-daw-application -- --test-threads=1
cargo test -p estudio-daw-ui-shell -- --test-threads=1
```

Las pruebas de `project-model` cubren migración v1–v4, valores predeterminados,
roles, fuentes/listas/regiones, referencias de casillas, identidad/orden y persistencia. `command-bus` cubre validación, historial y eventos de dominio. `application` cubre el ciclo de
apertura/guardado y deshacer. `ui-shell` cubre instantáneas, resúmenes y planificador MIDI.
Para comprobar todo el espacio de trabajo:

```sh
cargo test --workspace -- --test-threads=1
cargo check --workspace
cargo fmt --all -- --check
git diff --check
```

## Qué comprobar en la ventana

1. Crear proyecto nuevo: aparece una pista MIDI vacía y se habilitan Guardar
   como, transporte e historial según la instantánea.
2. Pulsar `＋M` y `＋A`: el contador/lista cambia; la pista nueva participa en
   deshacer/rehacer y sobrevive a guardar, cerrar y abrir el JSON.
3. Abrir la demostración MIDI: la instantánea reporta clips MIDI y reproducir intenta iniciar el
   instrumento elegido en la salida configurada. AudioBox/PipeWire se comprueba
   separadamente; el indicador “conectado” no mide latencia tecla→parlante.
4. Si una acción falla, copiar el texto completo del aviso, el comando de la
   terminal y `git rev-parse HEAD`; así se puede ubicar la frontera que falló.

## Límites actuales que no son fallos de la interfaz

Este documento se conserva como guía de depuración de una etapa anterior. El
estado actual ya importa y reproduce regiones de audio; permite asignar entradas
PipeWire, armar pistas y grabar WAV como regiones reversibles. Para los límites
vigentes, consultar `docs/audio-import.md` y las tareas abiertas en
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
