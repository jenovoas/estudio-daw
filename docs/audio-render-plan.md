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

## AudioBlock

`AudioBlock::new(channels, frames)` reserva una vez el buffer intercalado. El
callback sólo utiliza `samples_mut()` y llama a `RenderPlan::process_block()`;
no hay `Vec::resize`, creación de slices temporales ni cálculo de topología en
esa ruta.

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

## Reemplazo seguro del plan

`render_plan_exchange(initial)` entrega dos endpoints: `RenderPlanProcessor`
para el callback y `RenderPlanControl` para el único hilo productor/control.
El productor compila y publica un plan completo; el processor lo adopta sólo
al comenzar `process()` para un nuevo bloque. La topología activa no cambia a
mitad de bloque.

Durante Pause, el adaptador PipeWire escribe silencio sin llamar a
`RenderPlanProcessor::process`; así no avanza el estado de voces ni se consume
PCM del ring. El scheduler MIDI congela su reloj y descuenta el intervalo de
pausa al reanudar.

El intercambio usa dos slots con estados atómicos. El callback nunca destruye
el plan anterior: lo marca como retirado y el hilo de control lo libera mediante
`reap_retired()`. Hasta que se recoja ese slot, una nueva publicación se
rechaza devolviendo el plan preparado al caller. Esto evita que `Drop` de nodos
—incluido el cierre de workers— ejecute en tiempo real.

Los recursos no-RT del instrumento se asocian con `RenderPlan::retain_resource`.
El constructor SoundFont mantiene allí una referencia al worker: el plan nuevo
lo conserva mientras esté pendiente/activo y el worker anterior sólo puede
cerrarse tras reclamar su plan retirado. Las referencias externas usadas para
telemetría no deben mantener vivo indefinidamente un worker que se desea parar.

Un error al preparar el backend o compilar su plan ocurre antes de `publish`;
por tanto no cambia el slot activo y la reproducción continúa con el plan
anterior. El host PipeWire tiene variantes `*_controlled` para conectar estos
endpoints; los helpers existentes conservan su API y crean un intercambio
interno sin exponer control live.

## Prueba de regresión RT

`cargo test -p estudio-daw-audio-engine --test realtime_no_alloc` ejecuta el
procesamiento en un binario de integración con un allocator contador. El plan,
`AudioBlock`, ring SPSC y buffers de salida se construyen y calientan antes de
activar el contador; luego se repite el procesamiento 1.000 veces. El contador
es thread-local, de modo que asignaciones del harness en otros hilos no alteran
el resultado.

Esta prueba cubre el camino síncrono `RenderPlan::process_block()` y el paso por
el ring preasignado. La suite unitaria del audio-engine también prueba el
cambio de slot y la destrucción diferida; el test de integración `synth`
comprueba cero asignaciones al consumir PCM. Esto no sustituye una auditoría de
operaciones que evadan el allocator de Rust ni de procesos externos.
