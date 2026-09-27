# Tareas

## 1. Corrección de reproducción y seguridad en tiempo real

- [x] 1.1 Congelar el procesamiento de `RenderPlan` y el consumo de PCM en la devolución PipeWire mientras esté pausada.
- [x] 1.2 Limitar los eventos MIDI planificados según la duración de cada clip, conservando los desplazamientos iniciales y los eventos `NoteOff` en el límite final.
- [x] 1.3 Limitar el tono de activación de nota de `SineSynth` y sustituir el `expect` de la devolución por silencio seguro ante fallos.
- [x] 1.4 Calcular los metadatos de microsegundos del MIDI de demostración a partir del PPQ y tempo declarados.
- [x] 1.5 Añadir pruebas de regresión para límites de clips, notas fuera de rango y ranuras vacías del plan de renderizado.
- [x] 1.6 Mantener coherentes el transporte del dominio y el del dispositivo si falla el inicio de la reproducción.
- [x] 1.7 Congelar el renderizado del proceso SoundFont durante la pausa y comprobar que su cola PCM deje de avanzar.

## 2. Presentación de la estación de trabajo de escritorio

- [x] 2.1 Rediseñar la ventana Tauri para que el espacio de arreglo inspirado en Live sea el foco visual.
- [x] 2.2 Mantener las etiquetas de controles y avisos de capacidades fieles al comportamiento implementado.
- [x] 2.3 Actualizar README y la documentación pertinente de audio/interfaz según el comportamiento y los límites de reproducción conectada.

## 3. Verificación y trazabilidad

- [x] 3.1 Ejecutar `cargo fmt --all`, `cargo test --workspace -- --test-threads=1`, `cargo check --workspace`, la validación estricta de OpenSpec y `git diff --check`.
- [x] 3.2 Actualizar el traspaso del proyecto y añadir a la bitácora un registro factual con SHA, archivos, pruebas, resultado y asuntos de auditoría pendientes.
