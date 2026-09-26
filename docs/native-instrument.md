# Instrumentos MIDI nativos

`estudio-daw-synth` ofrece dos fuentes: el sinte sinusoidal polifónico (fallback
sin dependencias) y un adaptador opcional a FluidSynth para reproducir
SoundFonts locales. FluidSynth se carga dinámicamente; compilar Estudio DAW no
requiere tener instalada su biblioteca.

## Ruta de señal

```text
KeyLab (ALSA MIDI)
  → hilo de entrada fuera de RT
  → cola acotada de eventos MIDI
  → worker dedicado (FluidSynth, al elegir SoundFont)
  → ring SPSC PCM preasignado
  → nodo fuente del RenderPlan
  → EQ de inserción → PipeWire (AudioBox si está disponible)
```

El callback PipeWire no llama a FluidSynth, no espera locks y no reserva
memoria: consume muestras del ring. Si está vacío, escribe silencio y aumenta
el contador de underruns. La cola de eventos también es acotada y cuenta los
descartes. El SoundFont se carga y el preset se valida antes de iniciar el
stream; si falla, el comando muestra el diagnóstico y no modifica ningún
proyecto. El sinte sinusoidal se conserva como fallback explícito.

## SineSynth

El instrumento de prueba tiene 16 voces, frecuencias precalculadas, ataque y
release lineales; Note On con velocidad cero equivale a Note Off. No monitoriza
la entrada física de AudioBox.

## SoundFont local

Los comandos aceptan un SoundFont y preset opcionales. Banco y programa siguen
la numeración MIDI desde cero. Para descubrir los presets disponibles en un
banco local:

```bash
cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- \
  soundfont-presets /usr/share/soundfonts/FluidR3_GM.sf2
```

```bash
cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- \
  midi-synth-live 12 mi-toma.json "KeyLab Essential 49 MID" \
  --soundfont /usr/share/soundfonts/FluidR3_GM.sf2 --bank 0 --program 0

cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- \
  midi-synth-play mi-toma.json \
  --soundfont /usr/share/soundfonts/FluidR3_GM.sf2 --bank 0 --program 0
```

En Linux se buscan `libfluidsynth.so.3`, `.so.2` y `.so`. Instala la biblioteca
de runtime y un banco SF2 de forma independiente; la ruta del banco depende de
la distribución. Estudio DAW no descarga ni redistribuye SoundFonts. Si se
omite `--soundfont`, se usa SineSynth. Una ruta ilegible, preset inexistente o
runtime ausente se informa antes de iniciar el audio.

El modelo de proyecto ya puede guardar una referencia portable y un hash
opcional, sin copiar el banco. El motor y PipeWire exponen intercambio de
RenderPlan en límite de bloque; la selección interactiva y persistida desde la
UI queda pendiente.

## Probar SineSynth

```bash
cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- \
  midi-synth-live 12 mi-toma.json "KeyLab Essential 49 MID"
cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- \
  midi-synth-play mi-toma.json
```

El runtime selecciona AudioBox para salida cuando está disponible y conserva
la autoconexión PipeWire como fallback. La prueba de hardware previa capturó
una toma desde KeyLab y finalizó la reproducción PipeWire sin descartar eventos;
eso no es una evaluación subjetiva del timbre.

## Pruebas y límites

Los tests verifican render FluidSynth, paso de notas por el worker y ausencia de
asignaciones en el nodo PCM y en el handoff de RenderPlan bajo un allocator de conteo. Si
`/usr/share/soundfonts/FluidR3_GM.sf2` no existe, los tests del banco local se
omiten. Todavía no hay control semántico de sustain/CC ni editor de presets. La
API hot-swap está disponible para un host que conserve el endpoint de control,
pero la CLI y UI aún no ofrecen un selector interactivo durante reproducción.
