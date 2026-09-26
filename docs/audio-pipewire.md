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
salida y sirve como primera prueba nativa; captura AudioBox, duplex, xruns,
latencia medida y selección de nodos serán las siguientes extensiones.

## Dependencia de sistema

La compilación usa las bindings oficiales `pipewire-rs` y requiere las
bibliotecas de desarrollo PipeWire disponibles en Linux. El backend está
separado para que los crates de dominio y tests sin audio sigan siendo
portables.

