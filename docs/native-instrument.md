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

Al terminar `midi-synth-live` o `midi-synth-play`, el CLI informa la ocupación
actual y el máximo observado del ring PCM, su capacidad, la duración equivalente
del pico (`frames / sample_rate`) y el periodo PipeWire solicitado. La ocupación
se toma antes de consumir cada bloque; por eso el máximo describe cuánto audio
ya renderizado puede quedar por delante de un evento MIDI. No equivale a la
latencia completa desde una tecla hasta la salida acústica: esa medición requiere
sincronizar la entrada MIDI con un loopback físico de AudioBox y considerar el
hardware y PipeWire. En la prueba local de reproducción de una toma KeyLab, el
ring alcanzó 2048/2048 frames (42.67 ms a 48 kHz), con 288 frames al cierre,
cero underruns y cero errores del worker; el periodo PipeWire solicitado fue
32 frames (0.67 ms). Una apertura live de 3 s, sin tocar deliberadamente teclas,
observó el mismo pico, cero eventos MIDI descartados y cero underruns. Son
mediciones del puente PCM en este equipo, no una afirmación de latencia total.

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
  --soundfont /usr/share/soundfonts/FluidR3_GM.sf2 --bank 0 --program 0 \
  --capture loopback.wav

cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- \
  midi-synth-play mi-toma.json \
  --soundfont /usr/share/soundfonts/FluidR3_GM.sf2 --bank 0 --program 0 \
  --capture loopback.wav
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

`midi-synth-live --capture salida.wav` graba la entrada física de AudioBox al
mismo tiempo que la toma MIDI y el render SoundFont. `midi-synth-play --capture
salida.wav` captura la reproducción de una toma guardada. Ambos informes dan el
tiempo del primer callback de captura respecto del origen monotónico de los
eventos MIDI.

Para una medición acústica, el micrófono puede conectarse a Input 1 y colocarse
frente a los parlantes alimentados por AudioBox. La captura registra entonces
DAC, amplificación, altavoz, propagación por el aire, micrófono y ADC; distancia,
ganancia y sala afectan el resultado.

La AudioBox USB 96 expone un source monitor virtual de PipeWire para el sink de
salida (`alsa_output.…analog-stereo.monitor`). Es un puerto digital separado de
las entradas físicas: Input 1 conserva el micrófono e Input 2 la guitarra. El
CLI `--capture` todavía captura la entrada física, no el monitor. Un intento de
capturar el monitor como stream nativo llevó el callback PipeWire a procesar
casi muestra por muestra y causó underruns de FluidSynth; esa ruta experimental
se retiró. Hace falta desacoplar esa captura o corregir la configuración de
latencia antes de medirla. El monitor representa la señal digital del sink y no
incluye DAC, amplificación, parlantes, aire ni ADC.

## Pruebas y límites

Los tests verifican render FluidSynth, paso de notas por el worker y ausencia de
asignaciones en el nodo PCM y el handoff de RenderPlan bajo un allocator de
conteo. La prueba de ocupación verifica que el nodo registra los frames
disponibles antes de consumirlos. Si
`/usr/share/soundfonts/FluidR3_GM.sf2` no existe, los tests del banco local se
omiten. Todavía no hay control semántico de sustain/CC ni editor de presets. La
API hot-swap está disponible para un host que conserve el endpoint de control,
pero la CLI y UI aún no ofrecen un selector interactivo durante reproducción.
