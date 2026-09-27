# Tareas

## 1. Flujo de proyecto y transporte

- [x] 1.1 Añadir el comando Tauri para crear una sesión v2 vacía con una pista MIDI SineSynth y detener playback antes de reemplazar una sesión; verificar el contrato del snapshot y el estado del host.
- [x] 1.2 Añadir la acción Nuevo proyecto en el bridge y la cabecera, conectar la creación con el render de snapshot y explicar el comportamiento de una sesión vacía; verificar `node --check` y que la UI refleje el estado habilitado del transporte tras crearla.

## 2. Superficie visual de escritorio

- [x] 2.1 Reorganizar el tema CSS como superficie de DAW compacta con referencia visual aprobada de Live 12 Suite, sin recursos propietarios; verificar jerarquía y controles a ancho mínimo soportado.
- [x] 2.2 Actualizar documentación de la shell para describir la creación de sesión y los límites funcionales actuales; verificar que no prometa edición de clips, grabación live o audio que aún no existen.

## 3. Integración

- [x] 3.1 Ejecutar `cargo fmt --all -- --check`, pruebas del crate ui-shell, `cargo check -p estudio-daw-ui-shell`, `git diff --check` y `openspec validate ui-session-workflow --strict`; compilar Tauri y dejar abierta la instancia actualizada para revisar el flujo interactivo.


## 4. Sesión audible y arreglo DAW

- [x] 4.1 Añadir comando Demo MIDI que adjunta por el command bus una toma breve a una sesión SineSynth; verificar con test que el snapshot incluye clip/eventos y actualizar indicaciones de sonido.
- [x] 4.2 Añadir resúmenes compactos de clip al snapshot y renderizar regla de compases/lane por pista; verificar posiciones mediante fixtures con distintas PPQ y sesión vacía.
- [x] 4.3 Reorganizar la shell como superficie de arreglo ocupando la ventana, con barra compacta, lanes y preferencias colapsables; verificar jerarquía visual al tamaño mínimo de ventana.
- [x] 4.4 Actualizar documentación y correr `cargo fmt --all -- --check`, pruebas ui-shell, `cargo check -p estudio-daw-ui-shell`, `node --check`, `git diff --check` y `openspec validate ui-session-workflow --strict`; dejar Tauri ejecutándose con una ruta de prueba audible disponible.
