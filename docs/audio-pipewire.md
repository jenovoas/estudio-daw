# Backend PipeWire

El crate `estudio-daw-audio-platform` encapsula PipeWire `0.10.1` para Linux.
El backend recibe un `RenderPlan` ya compilado y no conoce `Project`, MIDI ni
la UI.

## Flujo nativo

```text
PipeWire stream F32LE
        ↓
buffer mapeado por PipeWire
        ↓
RenderPlan::process()
        ↓
buffer de salida
```

`run_pipewire_output()` crea el main loop, contexto, core y stream antes de
entrar al loop de PipeWire. El stream solicita `AUTOCONNECT`, `MAP_BUFFERS` y
`RT_PROCESS`.

## Callback

El callback no crea `Vec`, no serializa parámetros y no resuelve la topología.
Trabaja sobre el buffer F32LE mapeado por PipeWire, lo limpia y llama al plan
DSP. La serialización SPA y la negociación de formato suceden antes de
`main_loop.run()`.

## Configuración

```rust
let config = PipeWireStreamConfig {
    sample_rate: 48_000,
    channels: 2,
};
run_pipewire_output(config, render_plan)?;
```

La configuración se valida antes de abrir el stream. El backend actual es de
salida y sirve como primera prueba nativa.

## Duplex AudioBox

`run_pipewire_duplex()` crea un stream de captura y otro de reproducción. Ambos
usan el formato F32LE y comparten un `SampleRingBuffer` SPSC preasignado:

```text
AudioBox capture callback
          ↓ push
SampleRingBuffer (lock-free SPSC)
          ↓ pop
PipeWire playback callback → RenderPlan
```

La capacidad se calcula con `channels * max_buffer_frames * 4`. El periodo DSP
objetivo es `period_frames` (32 por defecto) y se solicita a PipeWire mediante
`node.latency`. Si la captura
produce más muestras que el espacio disponible, `push()` devuelve sólo las
muestras aceptadas y no bloquea el hilo de audio. Los contadores de overflow y
la medición de xruns se añadirán en la capa de diagnóstico.

El smoke test busca automáticamente nodos cuyo nombre o descripción contiene
`AudioBox` y pasa sus ids mediante `target.object`. Si no encuentra esos nodos,
conserva la autoconexión predeterminada de PipeWire. Latencia medida y
sincronización duplex serán las siguientes extensiones.

La ruta ya incluye un `EqualizerNode` con un high-pass de 20 Hz antes de la
salida. Esto verifica la integración real del DSP modular dentro del callback,
no sólo su compilación aislada.

## Captura a WAV sin bloquear el callback

`WavCaptureRecorder` ofrece la primera frontera de grabación persistente. El
callback sólo hace `push()` a un ring SPSC preasignado; un hilo escritor genera
un WAV IEEE-float de 32 bits y actualiza el encabezado al finalizar. Si el
disco no alcanza la velocidad de captura, el diagnóstico expone las muestras
descartadas en vez de bloquear el audio.

La integración con el stream duplex usará este componente como consumidor
secundario de la captura. El comando `audio-record` ya lo conecta al stream y
guarda una toma WAV mientras el render y la reproducción siguen funcionando de
forma independiente. La ruta podrá sustituirse después por FLAC, stems o un
proxy sin cambiar el contrato del callback.

```bash
cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- \
  audio-record 10 toma.wav
```

## Smoke test

El comando de laboratorio abre el duplex durante tres segundos, imprime los
dispositivos disponibles y reporta callbacks, muestras descartadas y silencio
insertado por falta de datos:

```bash
cargo run -q -p estudio-daw-cli --bin estudio-daw-project -- audio-test
```

Si el entorno no permite acceder al servidor PipeWire, el comando informa el
error de permisos/conexión sin ocultarlo.

También reporta muestras por callback y totales de captura/salida. Esos datos
permiten detectar una negociación de quantum inesperada antes de ajustar el
ring o atribuir un problema a la AudioBox.

## Dependencia de sistema

La compilación usa las bindings oficiales `pipewire-rs` y requiere las
bibliotecas de desarrollo PipeWire disponibles en Linux. El backend está
separado para que los crates de dominio y tests sin audio sigan siendo
portables.
