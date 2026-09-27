# Diseño

## Context

Consultar `proposal.md` para conocer la motivación y `specs/soundfont-instrument/spec.md` para el comportamiento observable. Actualmente `SineSynthNode` recibe MIDI dentro del plan de renderizado existente. La devolución de PipeWire debe seguir sin asignaciones de memoria ni bloqueos. La documentación oficial de renderizado de FluidSynth indica que las llamadas de renderizado bloquean y pertenecen a su hilo de síntesis; la carga SoundFont también realiza tareas inadecuadas para la devolución ([API de renderizado](https://www.fluidsynth.org/api/group__audio__rendering.html), [contexto de síntesis](https://www.fluidsynth.org/api/synth-context.html), [carga de SoundFont](https://www.fluidsynth.org/api/LoadingSoundfonts.html)).

## Goals / Non-Goals

**Objetivos:**

- Renderizar por la salida PipeWire y el grafo DSP existentes un preajuste SF2 local elegido por el usuario.
- Mantener la devolución PipeWire independiente de llamadas FluidSynth, acceso a archivos y bloqueos.
- Conservar el nodo sinusoidal y el formato actual de tomas MIDI.
- Hacer visibles la ausencia del entorno/archivo y la falta de datos de audio.

**Exclusiones:**

- Distribuir un SoundFont, afirmar que carece de restricciones de licencia o empaquetar una biblioteca de piano específica.
- Reimplementar un sintetizador basado en muestras o crear un editor de instrumentos en este cambio.
- Afirmar que no se añade latencia; la cola PCM entre proceso y devolución es observable, pero la latencia de extremo a extremo requiere una medición física de retorno MIDI/audio.
- Sustituir el flujo de destinos MIDI ALSA externos existente.

## Decisions

### Usar el motor FluidSynth oficial mediante un adaptador opcional acotado

El primer motor de plataforma usa la API C oficial de `libfluidsynth`, cargada como biblioteca compartida del sistema. La interfaz FFI debe permanecer aislada en el límite entre síntesis y plataforma; los tipos de FluidSynth no deben exponerse desde el modelo portable del proyecto. El crate antiguo de Rust `fluidsynth` encontrado durante la investigación tiene versión 0.0.1 y data de 2018; no debe usarse sin una auditoría independiente de mantenimiento y seguridad. `fluidlite` parece ser una alternativa de envoltorio seguro con mantenimiento, pero apunta a otro motor mínimo, FluidLite; sigue siendo una alternativa y no un reemplazo implícito. FluidSynth usa la licencia LGPL-2.1; se prefiere enlazar dinámicamente la biblioteca del sistema e incluir los avisos requeridos, sujeto a una revisión de licencias antes de distribuir ([licencia oficial](https://github.com/FluidSynth/fluidsynth/blob/master/LICENSE), [preguntas frecuentes oficiales sobre licencias](https://www.fluidsynth.org/wiki/LicensingFAQ/)).

Alternativas consideradas:

- **Escribir un motor SoundFont en Rust:** se descarta para este corte porque duplicaría análisis maduro de formato, reproducción de muestras, moduladores y envolventes.
- **Llamar directamente a FluidSynth desde `RenderPlan::process_block`:** se descarta porque su API de renderizado bloquea y requiere propiedad del contexto de síntesis, mientras que el contrato de la devolución prohíbe bloquear.
- **Ejecutar sólo el programa independiente `fluidsynth`:** se conserva como opción de interoperabilidad existente, pero se descarta como motor principal porque su flujo de audio y sus preajustes no aparecerían como fuente de instrumento dentro de nuestro grafo de renderizado.

### Asignar la instancia FluidSynth a un único proceso de trabajo

Crear y destruir el sintetizador, cargar/descargar el SoundFont, seleccionar preajustes, enviar MIDI y renderizar muestras en un único proceso de trabajo dedicado. Esto sigue el modelo de hilos de síntesis de FluidSynth y evita llamadas concurrentes sobre el estado mutable del sintetizador. Los comandos de control usan una cola acotada; el proceso publica bloques de audio flotante preasignados mediante una cola acotada de productor/consumidor único. La capacidad de las colas, el tamaño de bloque, el precargado inicial y la frecuencia de muestreo se configuran fuera de la devolución de audio.

El adaptador del plan de renderizado sólo extrae los cuadros PCM disponibles y los copia al `AudioBlock` preasignado. Nunca llama a la API C ni espera al proceso de trabajo. Si la cola está vacía, escribe silencio para los cuadros faltantes e incrementa el contador de interrupciones. Una carga de instrumento exitosa se intercambia en un límite seguro del plan; el instrumento anterior permanece activo hasta entonces. Una sustitución fallida no invalida el plan activo.

La sustitución del plan de renderizado usa dos ranuras de propiedad preasignadas
compartidas por un único productor de control y el procesador de audio. El
productor construye por completo el reemplazo antes de publicar su ranura con
orden de liberación. En el siguiente límite de
`RenderPlanProcessor::process`, la devolución adopta esa ranura y marca la
anterior como retirada. Nunca destruye un plan ni nodo; `RenderPlanControl`
recupera los planes retirados desde el hilo de control. No se acepta una segunda
publicación hasta completar esa recuperación. Cada `RenderPlan` conserva sus
procesos de instrumentos, así que el proceso SoundFont sigue activo mientras el
plan esté pendiente o en uso; recuperar el plan retirado libera esa propiedad
fuera del hilo de tiempo real. Así se mantienen los destructores de nodos y
procesos, y su espera de cierre, fuera de la devolución de audio. PipeWire ofrece
puntos de entrada controlados de salida y dúplex para que un anfitrión conserve
el extremo de control mientras transmite.

### Persistir referencias, no el contenido del banco de muestras

El estado portable del proyecto registra el motor, la referencia local al
SoundFont, el hash de contenido esperado opcional y la selección de
banco/programa. Nunca inserta ni copia el SoundFont. La resolución admite un
archivo relativo al proyecto cuando el usuario lo ha colocado deliberadamente
entre sus medios; de lo contrario usa una ruta local externa. Las referencias
ausentes producen un diagnóstico recuperable. Este cambio no incluye empaquetar
ni compartir proyectos con SoundFont de terceros.

### Conservar un comportamiento alternativo explícito

La fuente sinusoidal permanece disponible para pruebas y sistemas sin FluidSynth. Elegir el motor SoundFont nunca cambia el instrumento de manera silenciosa. Si el motor elegido no puede iniciarse, se informa el motivo concreto y se permite seleccionar explícitamente la fuente sinusoidal o volver a intentarlo.

## Risks / Trade-offs

- **[La planificación del proceso puede añadir latencia o causar interrupciones]** → mostrar la profundidad actual/máxima de la cola PCM y las interrupciones; documentar por separado la duración equivalente del búfer de cola y la latencia de extremo a extremo. Medir la latencia entre tecla y audio con retorno MIDI/audio físico en el hardware objetivo. No bloquear la devolución para ocultar la falta de datos.
- **[Incompatibilidad de ABI nativa o versión de biblioteca]** → validar los símbolos requeridos y la versión mínima del entorno antes de crear el instrumento; devolver un error de compatibilidad que indique cómo actuar.
- **[Los formatos y preajustes SoundFont varían]** → comenzar con SF2, enumerar los preajustes del archivo elegido y probar con un archivo proporcionado por el usuario, sin añadir bancos con derechos de autor a Git.
- **[La licencia SoundFont es independiente de FluidSynth]** → no almacenar bancos incluidos; mostrar la ruta externa y documentar la responsabilidad sobre la licencia del archivo.
- **[Hilos FluidSynth adicionales podrían competir con PipeWire]** → configurar inicialmente un único propietario de síntesis y valores internos conservadores; medir antes de habilitar núcleos FluidSynth en paralelo.

## Migration Plan

No se elimina ningún esquema existente de proyecto o toma. Se añade una
configuración opcional de instrumento con selector explícito de motor y un valor
de migración que conserva los proyectos existentes como instrumento sinusoidal
de prueba actual. La CLI conserva sus comandos actuales y añade selección
explícita de SoundFont. Si falta el entorno opcional, siguen funcionando la
fuente sinusoidal y las rutas MIDI externas. Para revertirlo, se selecciona el
instrumento sinusoidal; el MIDI del proyecto permanece intacto.

## Open Questions

- ¿Qué SoundFont proporcionado por el usuario se utilizará en la primera prueba de escucha manual? Esto no bloquea la implementación; las pruebas no deben depender de redistribuirlo.
