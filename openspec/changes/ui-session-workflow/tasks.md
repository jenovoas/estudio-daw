# Tasks

## 1. Flujo de proyecto y transporte

- [x] 1.1 Añadir el comando Tauri para crear una sesión v2 vacía con una pista MIDI SineSynth y detener playback antes de reemplazar una sesión; verificar el contrato del snapshot y el estado del host.
- [x] 1.2 Añadir la acción Nuevo proyecto en el bridge y la cabecera, conectar la creación con el render de snapshot y explicar el comportamiento de una sesión vacía; verificar `node --check` y que la UI refleje el estado habilitado del transporte tras crearla.

## 2. Superficie visual de escritorio

- [x] 2.1 Reorganizar el tema CSS como superficie de DAW compacta con referencia visual aprobada de Live 12 Suite, sin recursos propietarios; verificar jerarquía y controles a ancho mínimo soportado.
- [x] 2.2 Actualizar documentación de la shell para describir la creación de sesión y los límites funcionales actuales; verificar que no prometa edición de clips, grabación live o audio que aún no existen.

## 3. Integración

- [x] 3.1 Ejecutar `cargo fmt --all -- --check`, pruebas del crate ui-shell, `cargo check -p estudio-daw-ui-shell`, `git diff --check` y `openspec validate ui-session-workflow --strict`; compilar Tauri y dejar abierta la instancia actualizada para revisar el flujo interactivo.
