# Proposal

## Why

El instrumento nativo actual demuestra la captura MIDI y la ruta de renderizado en tiempo real, pero su tono sinusoidal sólo sirve para diagnóstico y no permite practicar ni producir de manera significativa. Un instrumento local SoundFont ofrecería sonidos conocidos de piano, bajo, guitarra y orquesta sin depender de la nube ni cobrar por reproducción, y permitiría reutilizar un motor de síntesis maduro y de código abierto en vez de recrear la reproducción de muestras.

## What Changes

- Añadir un motor de instrumento SoundFont opcional basado en la biblioteca de sistema `libfluidsynth`, con archivo y preajuste local seleccionados por el usuario.
- Mantener el instrumento sinusoidal disponible como alternativa pequeña sin dependencias y como fuente de pruebas.
- Aislar de la devolución PipeWire la carga SoundFont, el envío MIDI y el renderizado FluidSynth; intercambiar comandos MIDI acotados y bloques PCM preasignados con el grafo de renderizado.
- Informar claramente si falta la biblioteca/archivo, el preajuste no es válido, falla el proceso de renderizado o se interrumpe el flujo PCM; nunca sustituir silenciosamente un instrumento seleccionado.
- Guardar la configuración del instrumento como referencia a medios propiedad del usuario; no incluir ni redistribuir archivos SoundFont.

## Capabilities

### New Capabilities

- `soundfont-instrument`: configuración de instrumento SoundFont local, ciclo de vida, respuesta MIDI, entrega de audio y diagnósticos.

### Modified Capabilities

- Ninguna. Se conserva el contrato actual de la devolución de ejecución; este cambio añade una capacidad de instrumento que respeta ese contrato.

## Impact

- Crates posiblemente afectados: `synth`, `audio-engine`, `audio-platform`, `application` y `cli`; el artefacto de diseño determina el límite exacto.
- Añade una dependencia opcional del sistema en tiempo de ejecución para FluidSynth, enlazada dinámicamente para que el usuario pueda instalarla o actualizarla por separado. La biblioteca usa LGPL; se incluirán los avisos requeridos y se auditará el enlace Rust elegido antes de adoptarlo.
- El estado del proyecto referenciará el SoundFont del usuario, pero no lo copiará. Las licencias SoundFont son responsabilidad del usuario y no quedan cubiertas por la licencia de la biblioteca.
- PipeWire sigue siendo el motor de salida; MIDI, planificación y el `SineSynthNode` actual siguen disponibles si FluidSynth no está instalado.
