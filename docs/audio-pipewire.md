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

La capacidad se calcula con `channels * max_buffer_frames * 4`. Si la captura
produce más muestras que el espacio disponible, `push()` devuelve sólo las
muestras aceptadas y no bloquea el hilo de audio. Los contadores de overflow y
la medición de xruns se añadirán en la capa de diagnóstico.

La selección explícita de nodos, latencia medida y sincronización duplex serán
las siguientes extensiones.

## Dependencia de sistema

La compilación usa las bindings oficiales `pipewire-rs` y requiere las
bibliotecas de desarrollo PipeWire disponibles en Linux. El backend está
separado para que los crates de dominio y tests sin audio sigan siendo
portables.
