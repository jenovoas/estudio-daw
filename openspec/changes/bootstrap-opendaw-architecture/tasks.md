# Tareas: arquitectura fundacional de Estudio DAW

## Identidad y reutilización

- [x] Adoptar `Estudio DAW` como nombre del proyecto tras una búsqueda inicial de
  software, repositorios y paquetes.
- [ ] Verificar marca y registrar dominio cuando el proyecto vaya a publicarse.
- [ ] Auditar openDAW existente: SDK, repositorios, licencia AGPL/comercial, formato
  DAWproject, APIs y posibilidades reales de integración.
- [ ] Crear una matriz de reutilización: referencia, adaptador, dependencia o
  implementación propia para cada componente.
- [ ] Revisar `@opendaw/studio-sdk`, `openDAW-headless`, documentación y ejemplos en
  un entorno aislado, sin incorporarlos todavía al workspace principal.
- [x] Probar import/export de un fixture DAWproject y documentar pérdidas o campos
  incompatibles con el modelo Rust.
- [x] Crear CLI mínima para importar/exportar DAWproject y `project.json`.
- [x] Integrar una toma MIDI grabada como `MidiClip` en `project.json`.
- [x] Añadir cuantización no destructiva de Note On/Off en clips MIDI.
- [x] Añadir reproducción MIDI básica hacia destinos ALSA.
- [x] Reproducir un `MidiClip` directamente desde `project.json`.
- [x] Añadir estado de transporte, pausa, parada y loop interactivo en reproducción MIDI.
- [ ] Registrar atribuciones, licencias, dependencias transitivas y estrategia de
  aislamiento antes de portar cualquier código o asset.

## Preparación

- [x] Completar Architecture Reset v2 con límites de crates, matriz tecnológica y fases.
- [x] Registrar estrategia de modular monolith, CommandBus, ChangeSet y fallbacks.
- [ ] Convertir las decisiones v2 en ADRs individuales con pruebas de aceptación.
- [ ] Confirmar distribución Linux y backend de audio objetivo.
- [ ] Registrar modelo exacto del KeyLab Essential y de la interfaz de audio.
- [ ] Decidir licencia inicial del repositorio.
- [ ] Definir política de assets, modelos y archivos de audio.
- [ ] Definir política de originales, proxies, caché y renders finales.
- [ ] Crear ADR inicial para decisiones irreversibles.

## Runtime Rust

- [x] Crear workspace Cargo y crate base de modelo/intercambio.
- [x] Implementar enumeración diagnóstica de dispositivos de audio y MIDI mediante PipeWire.
- [ ] Implementar stream de audio de prueba.
- [x] Crear módulo DSP modular de ecualización con bandas sin asignaciones en procesamiento.
- [ ] Leer Note On/Off, velocity, CC, pitch bend y sustain.
- [x] Implementar transporte sample/beat/bar mediante `TransportClock` y PPQ
  fijo de 960.
- [x] Crear reloj de reproducción determinista independiente del wall-clock.
- [x] Añadir captura de audio a WAV mediante ring SPSC y escritor fuera del
  callback RT.
- [x] Añadir captura MIDI a un modelo de evento y toma temporal.
- [x] Implementar `MediaSource`/`ProxyAsset` y política original/proxy/auto.
- [x] Crear jobs de proxy con SHA-256, perfil de audio Opus, validación ffprobe,
  publicación atómica y limpieza del temporal.
- [x] Exponer estados de caché `missing`, `building`, `ready` y `stale`, con
  exclusión de generaciones concurrentes.
- [x] Añadir administrador de caché con rutas deterministas y reutilización de
  proxies `ready`.
- [x] Persistir el manifiesto de proxies con escritura atómica y restaurarlo al
  cargar una sesión.
- [x] Integrar `MediaSource` en `Track` de audio con serialización JSON y
  validación de tipo de pista.
- [x] Conectar asociación de fuente y generación/reutilización de proxy al
  flujo CLI de proyecto sin sobrescribir el JSON de entrada.
- [x] Añadir `AudioClip` con posición musical, offset/duración en samples y
  comando CLI de inserción no destructiva.
- [x] Mantener las operaciones de audio independientes de la CLI para futura UI
  nativa y adaptador WASM.
- [x] Añadir `ChangeSet` transaccional y `ProjectHistory` con undo/redo para
  operaciones del dominio.
- [x] Exponer `ProjectSnapshot` versionado y eventos de commit/undo/redo para
  actualizar la UI sin acoplarla al modelo interno.
- [ ] Definir scheduler híbrido CPU/GPU, presupuestos de transferencia y fallback CPU.
- [ ] Crear backend de cómputo `wgpu`/Vulkan fuera del callback de audio.
- [ ] Benchmark CPU/GPU para FFT, convolución, time-stretch, render y análisis.
- [x] Definir contrato testeable de selección CPU/GPU y razones de fallback.
- [x] Crear baseline CPU reproducible para DFT de referencia y convolución.
- [x] Implementar FFT radix-2 CPU con prueba de round-trip.
- [x] Añadir sonda `wgpu` de adaptadores y límites de GPU.

## Modelo de proyecto

- [ ] Separar `Portable Domain` de `Platform Runtime` mediante crates y traits explícitos.
- [ ] Definir `Session`, `TransportSnapshot`, `CommandBus` y `ChangeSet`.
- [x] Crear mapa modular de control MIDI para transporte, escenas y volumen
  master, con lector ALSA reutilizable y resolución sin asignaciones.
- [x] Aplicar comandos MIDI a un estado de sesión live con transporte,
  grabación, loop, escena activa y volumen master.
- [x] Conectar play/pause, stop y loop MIDI con el reproductor de takes.
- [x] Añadir perfil del puerto DAW del KeyLab para Play (94), Stop (93) y
  Record (95).
- [x] Exponer reproducción de clips de proyecto con control MIDI live mediante
  `project-play-live`.
- [x] Permitir iniciar y detener una grabación MIDI desde el botón Record del
  puerto DAW con `midi-record-live`.
- [x] Crear crate de sesión con `TransportSnapshot`, `SessionCommand` y
  `CommandBus` bounded, documentado y probado sin hardware.
- [x] Traducir `MidiControlCommand` a `SessionCommand` sin acoplar el dominio a
  MIDI.
- [x] Ejecutar el flujo MIDI → `CommandBus` → `Session` desde
  `midi-control-monitor`.
- [x] Crear `audio-engine` con DAG compilado, detección de ciclos y adaptador
  del ecualizador modular.
- [x] Añadir `AudioBlock` preasignado y `RenderPlan::process_block()` sin
  redimensionar buffers durante el procesamiento.
- [x] Crear adaptador PipeWire F32LE con stream autoconectado y flag
  `RT_PROCESS`, aislado en `audio-platform`.
- [x] Añadir ring SPSC preasignado y streams PipeWire de captura/reproducción
  para el primer flujo duplex.
- [x] Crear smoke test duplex finito con métricas de callbacks, muestras
  descartadas y silencio de salida.
- [x] Seleccionar automáticamente los nodos PipeWire de AudioBox por nombre e
  id de objeto.
- [x] Diagnosticar y limitar el quantum PipeWire del callback mediante
  `period_frames` y `node.latency`.
- [x] Definir schema inicial de `project.json` para la fixture de interoperabilidad.
- [ ] Definir entidades Track, Clip, Take, Device, Automation y AnalysisArtifact.
- [ ] Definir relación original-proxy, timebase, canales y estado de disponibilidad.
- [ ] Implementar comandos y eventos.
- [ ] Implementar undo/redo agrupado.
- [ ] Añadir migración de schema.

## Worker Python

- [ ] Crear entorno Python reproducible.
- [ ] Definir protocolo de jobs y eventos.
- [ ] Implementar `ArtifactRef` mmap/memmap para originales, stems y proxies.
- [ ] Implementar worker heartbeat y cancelación.
- [ ] Crear job de prueba que inspeccione duración y formato.
- [ ] Integrar separación de stems.
- [ ] Integrar transcripción bilingüe.
- [ ] Integrar melodía vocal y acordes.
- [ ] Guardar resultados como artefactos versionados.
- [ ] Invalidar análisis cuando cambia la fuente o el proxy de procedencia.

## Scripting

- [ ] Definir gramática mínima de sesión.
- [ ] Implementar parser con errores de línea/columna.
- [ ] Traducir AST a comandos del proyecto.
- [ ] Añadir evaluación en modo preview.
- [ ] Añadir evaluación en vivo en un punto de sincronización.

## Agente y profesor

- [ ] Definir catálogo de herramientas tipadas.
- [ ] Implementar autorización y confirmación de mutaciones.
- [ ] Crear adaptador de LLM desacoplado del proveedor.
- [ ] Construir contexto pedagógico desde análisis y progreso.
- [ ] Añadir explicación de teoría basada en datos de la sesión.
- [ ] Añadir ejercicios de oído y registro de resultados.

## Verificación

- [ ] Test de no asignaciones en el callback de audio.
- [ ] Test de estabilidad de transporte.
- [ ] Test de desconexión/reconexión MIDI.
- [ ] Test de worker caído durante reproducción.
- [ ] Test de serialización y migración.
- [ ] Test de proxy offline, proxy obsoleto, fuente ausente y render final desde original.
- [ ] Test de undo de operación del agente.
- [ ] Prueba vertical: KeyLab → synth → grabación MIDI → reproducción.

## Web y WASM

- [ ] Definir la frontera portable entre Native Core, Portable Domain y Web Adapter.
- [ ] Diseñar un adaptador experimental para evaluar el SDK web existente sólo
  después de cerrar la auditoría de licencia.
