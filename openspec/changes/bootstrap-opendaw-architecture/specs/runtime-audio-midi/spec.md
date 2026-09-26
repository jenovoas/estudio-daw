# Runtime de audio y MIDI

## Propósito

Definir un motor de audio/MIDI determinista y seguro para tiempo real, capaz de representar pistas, buses, envíos, retornos, sidechain, plugins, instrumentos y entradas físicas sin resolver dependencias ni asignar memoria dentro del callback. La primera plataforma objetivo es Linux/Arch Linux, con PipeWire y JACK.

## Grafo DSP y renderizado

- La sesión se modela como un grafo dirigido de nodos DSP y conexiones tipadas de audio, MIDI y control.
- El grafo admite pistas, clips, buses, sends/returns, sidechain, plugins, instrumentos, entradas físicas y master.
- El motor compila el grafo fuera del hilo de audio a un `RenderPlan` inmutable: valida conexiones, detecta ciclos inválidos, calcula orden topológico, asigna buffers, prepara sidechain y calcula compensación de latencia.
- El hilo de audio ejecuta una lista plana precomputada de operaciones. No resuelve dependencias, busca nodos ni recompila durante el render.
- Los cambios de sesión publican un nuevo plan en un límite seguro de bloque usando doble buffer, RCU o un mecanismo equivalente sin bloqueo.
- Los feedback loops sólo existen mediante nodos explícitos con estado y límites; un ciclo arbitrario se rechaza antes de activar el plan.

## Arena de buffers sin asignaciones

- Al iniciar la sesión se reserva una arena de scratch para el máximo de canales y `MAX_BLOCK_SIZE`.
- El callback no puede ejecutar `Vec::new`, operaciones que puedan crecer un `Vec`, reasignaciones, `Box`, locks bloqueantes, syscalls, I/O, logging no preasignado, red ni llamadas a Python.
- Los buffers se identifican mediante índices/handles y se reutilizan desde la arena preasignada.
- Las colas de audio, MIDI y control son bounded y lock-free o wait-free según el caso; overflow y pérdida de eventos se contabilizan.
- Tamaño de bloque, sample rate y máximo de canales se fijan antes de iniciar el stream; cambiarlos requiere reconstruir el plan.

## Backends Linux y latencia

- PipeWire nativo (`pipewire-rs`) es el backend primario para escritorio y routing moderno.
- JACK (`jack-rs`) es el backend primario de compatibilidad profesional.
- ALSA se usa como base para MIDI cuando corresponda.
- `cpal` se mantiene como abstracción genérica/fallback, no como definición de las capacidades profesionales de routing multipista.
- Con AudioBox USB 96 se medirá y optimizará una latencia round-trip inferior a 5 ms bajo una configuración documentada; no es una garantía independiente del kernel, driver, sample rate y tamaño de bloque.

## Sistema de proxies y media pesada

Estudio DAW debe separar la fuente maestra de los medios usados para edición y preview, siguiendo el patrón de proxies de los editores de vídeo.

- Cada archivo de audio puede tener un `MediaSource` original y uno o más `ProxyAsset` derivados: mono/stereo, sample rate, bit depth, codec, canales y calidad configurables.
- El proxy conserva la misma duración, timebase, alineación de canales y referencia temporal que el original. No se usa como fuente maestra de exportación.
- La política de sesión permite `original`, `proxy` o `auto`: edición/preview puede usar proxy y render final cambia automáticamente a original cuando está disponible.
- La creación, regeneración, validación y transcodificación de proxies ocurre fuera del hilo de audio mediante jobs. El callback sólo abre el asset ya preparado.
- Los proxies se identifican por hash de fuente, parámetros de generación y versión del encoder. Si cambia la fuente o el perfil, se marcan obsoletos y se regeneran.
- El proyecto guarda sólo referencias y estado de proxy, no copia innecesariamente el audio pesado dentro de `project.json`.
- Si falta un original, Estudio DAW puede seguir editando con proxy y muestra un estado explícito que impide presentar ese render como master final.
- El caché de proxies tiene límites de espacio, limpieza segura y opción de ubicarlo en otro disco. Nunca se elimina una fuente maestra durante la limpieza.
- Para análisis y render distribuido, un proxy puede exponerse como `ArtifactRef` mmapeable, pero el sistema conserva la relación con el original.

## MIDI, dispositivos y grabación

- El runtime enumera dispositivos de audio/MIDI, capacidades y estado de conexión.
- Los eventos MIDI llevan timestamp de bloque y llegan por colas preasignadas.
- KeyLab Essential 49 se soporta mediante MIDI genérico y un perfil configurable; éste puede mapear transporte, loop, stop, play, record, faders, paneo, knobs, pads y channel strip activo.
- MCU/HUI sólo se activa cuando el dispositivo y su configuración lo validan; no se asume que todas las variantes del KeyLab exponen el mismo protocolo.
- AudioBox USB 96 dispone de un perfil con entradas nombrables, ruteo, monitorización directa y presets. El usuario puede asignar una entrada a voz y otra a guitarra/instrumento sin imponer un modo físico único.
- Gate, de-esser, simulación de caja y amp modeling son inserts no destructivos del template de entrada y nunca sustituyen la toma original.
- La grabación conserva la toma, timestamps, dispositivo, sample rate, ganancia declarada y procesamiento de monitoreo.

## Requisitos verificables

- **Dado** un proyecto con pistas, bus, send y sidechain, **cuando** se activa, **entonces** se genera un `RenderPlan` topológico sin dependencias en callback.
- **Dado** un ciclo no permitido, **cuando** se compila, **entonces** se rechaza y el plan anterior continúa ejecutándose.
- **Dado** un stream activo, **cuando** se procesan bloques, **entonces** el callback no asigna memoria, toma locks bloqueantes ni accede a disco o workers.
- **Dado** un original grande con proxy disponible, **cuando** la sesión está en modo proxy, **entonces** preview y edición usan el proxy sin alterar la referencia al original.
- **Dado** un render final, **cuando** el original está disponible, **entonces** se usa el original y se informa cualquier proxy obsoleto o fuente ausente.
- **Dado** un KeyLab y una AudioBox conectados, **cuando** se seleccionan sus perfiles, **entonces** los controles y entradas disponibles pueden mapearse, monitorizarse y probarse desde diagnóstico.
- El benchmark registra tamaño de bloque, sample rate, xruns, latencia I/O y round-trip, con objetivo inicial <5 ms bajo una configuración documentada.
