# Primer instrumento MIDI nativo

`estudio-daw-synth` implementa la primera fuente de sonido propia de Estudio DAW.
Su alcance intencional es reducido: una voz sinusoidal polifónica permite
verificar eventos MIDI, renderizado de bloques y salida PipeWire antes de abordar
samplers, SoundFonts o plugins.

## Ruta de señal

```text
KeyLab (ALSA MIDI)
  → hilo de entrada fuera de RT
  → ring SPSC preasignado (256 eventos)
  → SineSynthNode dentro del RenderPlan
  → EQ de inserción
  → PipeWire playback (AudioBox si está disponible)
```

El ring tiene un productor y un consumidor no clonables. Si se llena, el envío
devuelve `false` y el productor incrementa un contador; el callback nunca espera
ni reserva memoria. El nodo limita su polifonía a 16 voces, calcula las
frecuencias antes de iniciar audio y aplica ataque/release lineales simples.
Note On con velocidad cero se interpreta como Note Off.

El primer nodo limpia el bloque y genera la fuente: este comando no monitoriza la
entrada física de AudioBox. En este corte no hay presets, control de volumen por
CC, sustain semántico, tabla de ondas ni muestras. El motor y el contrato MIDI se
mantienen listos para reemplazar esta fuente por instrumentos de mayor calidad.

## Probar desde CLI

Grabar una toma tocando y oír simultáneamente el sinte:

```bash
cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- \
  midi-synth-live 12 mi-toma.json "KeyLab Essential 49 MID"
```

Reproducir después la toma por el mismo instrumento nativo:

```bash
cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- \
  midi-synth-play mi-toma.json
```

El runtime busca el sink AudioBox para la salida y usa la autoconexión de
PipeWire si no encuentra uno. La validación automatizada cubre generación,
liberación y ausencia de asignaciones; la prueba auditiva con el hardware sigue
siendo una verificación manual, no se infiere del test unitario.
