# Audio RenderPlan

`estudio-daw-audio-engine` contiene la primera implementación del grafo DSP
propio de Estudio DAW.

## Compilación del grafo

La topología se construye fuera del procesamiento:

```text
RenderPlanBuilder
    ↓ add_node / connect
orden topológico + validación de ciclos
    ↓
RenderPlan
    ↓
lista plana de AudioNode
```

`connect(source, target)` declara una dependencia. `build()` rechaza ciclos y
produce una lista ordenada. El hilo de audio nunca resuelve dependencias ni
recorre un mapa de conexiones.

## Contrato de nodo

Un `AudioNode` recibe un bloque intercalado mutable y procesa su estado interno:

```rust
pub trait AudioNode: Send {
    fn process(&mut self, interleaved: &mut [f32]) -> Result<(), AudioNodeError>;
}
```

El nodo no debe asignar memoria ni bloquear. La configuración, creación de
nodos y recompilación del plan pertenecen al hilo de control. Actualmente el
crate incluye `GainNode` y `EqualizerNode` como pruebas de integración.

## Ejemplo de cadena

```rust
let mut eq = EqualizerNode::new(48_000.0, 2)?;
eq.add_band(EqBandConfig::bell(300.0, -3.5, 1.0))?;

let mut builder = RenderPlanBuilder::new();
let input = builder.add_node(input_node);
let equalizer = builder.add_node(eq);
let master = builder.add_node(GainNode::new(0.8));
builder.connect(input, equalizer)?;
builder.connect(equalizer, master)?;
let mut plan = builder.build()?;
```

La integración de entrada/salida PipeWire y los buffers scratch preasignados
vendrá después. El `RenderPlan` actual valida la arquitectura sin introducir
todavía una dependencia del sistema de audio.

