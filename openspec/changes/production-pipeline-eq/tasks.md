# Tasks

## 1. Project Model

- [x] 1.1 Verificar que `EqBandConfig` y `EqFilterType` tienen serialización estable; añadir derives y dependencia serde si faltan
- [x] 1.2 Añadir `eq_bands: Vec<EqBandConfig>` al struct `Track` con default vacío y compatibilidad de deserialización
- [x] 1.3 Exponer los tipos EQ desde la frontera adecuada del workspace sin acoplar la UI a detalles innecesarios
- [x] 1.4 Añadir o actualizar pruebas de persistencia de bandas en project-model

## 2. Backend — Comando Tauri

- [x] 2.1 Añadir `eqBands` al snapshot de pistas y poblarlo desde el modelo
- [x] 2.2 Implementar `set_track_eq(track_id, bands)` con validación de pista y máximo de 8 bandas
- [x] 2.3 Garantizar que una petición inválida no modifica el proyecto ni publica un plan
- [x] 2.4 Registrar el comando en Tauri y cubrirlo con prueba de contrato

## 3. Motor de Audio — RenderPlan

- [x] 3.1 Insertar `EqualizerNode` por pista después de la fuente y antes de gain/pan
- [x] 3.2 Construir el nodo fuera del callback con sample rate y canales correctos
- [x] 3.3 Publicar el plan recompilado mediante el intercambio existente sin interrumpir reproducción
- [x] 3.4 Añadir prueba de procesamiento que distinga bypass de una banda activa

## 4. Frontend — Panel EQ

- [x] 4.1 Añadir `setTrackEq` a `platform-tauri.js` y contrato equivalente a `platform-preview.js`
- [x] 4.2 Añadir caja EQ al Device Rack con filas de tipo, Hz, dB, Q, bypass y eliminar
- [x] 4.3 Enviar el array completo al backend después de cada edición y renderizar desde el snapshot devuelto
- [x] 4.4 Limitar la creación a 8 bandas y mostrar errores de validación
- [x] 4.5 Añadir estilos compactos y legibles para la caja y sus controles
- [x] 4.6 Verificar sintaxis JS y navegación del panel

## 5. Verificación

- [x] 5.1 Ejecutar pruebas unitarias y de contrato de project-model, audio-engine y ui-shell
- [x] 5.2 Smoke test: añadir una banda Bell en la UI de preview y verificar el procesamiento activo frente al bloque bypass en audio-engine
- [x] 5.3 Verificar guardar/recargar conserva las bandas
- [x] 5.4 Verificar bypass conserva configuración y elimina el efecto
- [x] 5.5 Ejecutar la suite completa del workspace y documentar el resultado: 404 pruebas pasadas, 1 ignorada
