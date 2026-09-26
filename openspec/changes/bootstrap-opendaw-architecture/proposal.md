# Propuesta: arquitectura fundacional de Estudio DAW

> `openDAW` es un proyecto existente y no debe usarse como nombre del producto.
> El nombre adoptado para este proyecto es **Estudio DAW**.

## Qué se propone

Definir la arquitectura inicial de Estudio DAW como un estudio de producción musical programable, con núcleo de audio/MIDI en Rust, workers de análisis en Python y una capa de herramientas para un profesor/productor virtual.

La propuesta cubre el contrato entre componentes, los límites del tiempo real, el modelo de proyecto, la ejecución de scripts musicales y la integración futura de análisis e IA. No implementa todavía el motor ni la interfaz.

## Por qué

El objetivo no es crear únicamente un editor de audio ni un clon visual de Ableton Live. Estudio DAW debe permitir producir música con guitarra, voz, micrófono, interfaz de audio y teclado MIDI, pero también modificar una sesión mediante código o instrucciones naturales.

El núcleo debe ser suficientemente sólido para soportar grabación y reproducción reales, y suficientemente abierto para añadir análisis musical, live coding, instrumentos, plugins y enseñanza personalizada durante años.

## Objetivos

- Definir una arquitectura Linux-first.
- Separar estrictamente el tiempo real de análisis, red y LLM.
- Establecer un modelo de proyecto versionable y reversible.
- Permitir que la UI, el lenguaje musical y el agente modifiquen el mismo estado.
- Soportar audio, MIDI, clips, automatización y transporte musical.
- Preparar contratos para Python sin acoplar el motor a un modelo específico.
- Diseñar una base que permita incorporar un profesor virtual bilingüe.

## Fuera de alcance inicial

- Compatibilidad total con sesiones de Ableton, Logic o Pro Tools.
- Recrear toda la suite de instrumentos de Arturia.
- Publicación/distribución de música de terceros.
- Entrenamiento de modelos de IA propios.
- Aplicación móvil.
- Colaboración remota multiusuario.
- Compatibilidad garantizada con todos los plugins existentes.

## Supuestos

- El primer entorno objetivo es Linux x86_64.
- La interfaz de audio se conecta mediante PipeWire/ALSA/JACK según el backend disponible.
- El teclado Arturia se usa como dispositivo MIDI USB.
- El audio original permanece local; los workers pueden usar modelos descargados localmente.
- La LLM puede ser remota por API, pero recibe operaciones y resultados estructurados, no control irrestricto del sistema.

## Criterios de éxito de esta fase

- Existe un modelo claro de componentes y responsabilidades.
- Se puede implementar un vertical slice sin decisiones arquitectónicas pendientes.
- Las operaciones de audio/MIDI y las operaciones de IA tienen contratos versionados.
- Las futuras decisiones de UX, almacenamiento y plugins no rompen el motor de tiempo real.

## Resultado de la auditoría de openDAW existente

El proyecto existente reduce el riesgo del futuro modo navegador, pero no elimina
la necesidad del núcleo nativo. Estudio DAW adoptará como objetivos de
interoperabilidad el formato DAWproject, el concepto de bundles/media assets y una
frontera de dominio portable. El motor Linux, la memoria en tiempo real,
PipeWire/JACK, los proxies locales y el protocolo Python/IA se mantienen como
diseño propio.

No se copiará código ni se hará un fork hasta verificar AGPL/comercial, avisos de
copyright, dependencias y consecuencias para distribución.
