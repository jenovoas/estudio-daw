//! Cadena DSP in-place compilada para el motor de audio.
//!
//! El builder ordena nodos fuera del callback. `RenderPlan::process` ejecuta
//! la cadena en orden de inserción sobre un solo bloque mutable; el mixer
//! explícito gestiona varias fuentes con scratch preasignado.

pub use estudio_daw_dsp::EqBandConfig;
use estudio_daw_dsp::{DspError, Equalizer};
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::sync::Arc;
use thiserror::Error;

pub trait AudioNode: Send {
    fn process(&mut self, interleaved: &mut [f32]) -> Result<(), AudioNodeError>;
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AudioBlockError {
    #[error("la cantidad de canales debe ser mayor que cero")]
    InvalidChannelCount,
    #[error("la cantidad de frames debe ser mayor que cero")]
    InvalidFrameCount,
}

/// Buffer intercalado reservado antes de iniciar el stream de audio.
///
/// `AudioBlock::new` puede asignar porque pertenece a la preparación del
/// stream. `RenderPlan::process_block` sólo presta el slice existente a cada
/// nodo; no redimensiona ni crea buffers en el callback.
#[derive(Debug, PartialEq)]
pub struct AudioBlock {
    channels: usize,
    frames: usize,
    samples: Vec<f32>,
}

/// Ring SPSC de muestras para conectar captura y reproducción sin locks.
///
/// Sólo debe existir un productor y un consumidor. La memoria se reserva en
/// `new`; `push` y `pop` no asignan, no bloquean y funcionan con índices
/// atómicos. Es el buffer que usará el backend duplex entre callbacks.
pub struct SampleRingBuffer {
    samples: Box<[UnsafeCell<f32>]>,
    capacity: usize,
    read: AtomicUsize,
    write: AtomicUsize,
}

// Seguridad: el contrato SPSC garantiza que productor y consumidor nunca
// escriben/leen simultáneamente la misma celda. Los índices atómicos publican
// las muestras con Release/Acquire antes de que el otro hilo las observe.
unsafe impl Send for SampleRingBuffer {}
unsafe impl Sync for SampleRingBuffer {}

impl SampleRingBuffer {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "un ring de audio necesita capacidad");
        let mut samples = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            samples.push(UnsafeCell::new(0.0));
        }
        Self {
            samples: samples.into_boxed_slice(),
            capacity,
            read: AtomicUsize::new(0),
            write: AtomicUsize::new(0),
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn available(&self) -> usize {
        self.write
            .load(Ordering::Acquire)
            .saturating_sub(self.read.load(Ordering::Acquire))
            .min(self.capacity)
    }

    pub fn push(&self, input: &[f32]) -> usize {
        let write = self.write.load(Ordering::Relaxed);
        let read = self.read.load(Ordering::Acquire);
        let writable = self.capacity - write.saturating_sub(read).min(self.capacity);
        let count = input.len().min(writable);
        for (offset, sample) in input.iter().take(count).enumerate() {
            let index = (write + offset) % self.capacity;
            // SAFETY: sólo el productor escribe la posición antes de publicar
            // el nuevo índice `write` con Release.
            unsafe { *self.samples[index].get() = *sample };
        }
        self.write.store(write + count, Ordering::Release);
        count
    }

    pub fn pop(&self, output: &mut [f32]) -> usize {
        let read = self.read.load(Ordering::Relaxed);
        let write = self.write.load(Ordering::Acquire);
        let readable = write.saturating_sub(read).min(self.capacity);
        let count = output.len().min(readable);
        for (offset, sample) in output.iter_mut().take(count).enumerate() {
            let index = (read + offset) % self.capacity;
            // SAFETY: sólo el consumidor lee la posición antes de publicar
            // el nuevo índice `read` con Release.
            unsafe { *sample = *self.samples[index].get() };
        }
        self.read.store(read + count, Ordering::Release);
        count
    }
}

impl AudioBlock {
    pub fn new(channels: usize, frames: usize) -> Result<Self, AudioBlockError> {
        if channels == 0 {
            return Err(AudioBlockError::InvalidChannelCount);
        }
        if frames == 0 {
            return Err(AudioBlockError::InvalidFrameCount);
        }
        Ok(Self {
            channels,
            frames,
            samples: vec![0.0; channels * frames],
        })
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn frames(&self) -> usize {
        self.frames
    }

    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    pub fn samples_mut(&mut self) -> &mut [f32] {
        &mut self.samples
    }

    pub fn clear(&mut self) {
        self.samples.fill(0.0);
    }
}

#[derive(Debug, Error)]
pub enum AudioNodeError {
    #[error("el bloque de audio tiene una longitud inválida")]
    InvalidBlockLength,
    #[error("falló un worker externo de audio")]
    WorkerFailed,
    #[error("fallo DSP: {0}")]
    Dsp(#[from] DspError),
}

/// Builder para una cadena DSP in-place en orden de inserción.
pub struct RenderPlanBuilder {
    nodes: Vec<Box<dyn AudioNode>>,
}

impl Default for RenderPlanBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderPlanBuilder {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add_node<N>(&mut self, node: N)
    where
        N: AudioNode + 'static,
    {
        self.nodes.push(Box::new(node));
    }

    /// Termina la cadena fuera del callback; los nodos conservan su orden.
    pub fn build(self) -> RenderPlan {
        RenderPlan {
            nodes: self.nodes,
            retained_resources: Vec::new(),
        }
    }
}

/// Plan DSP inmutable en topología, mutable sólo en el estado interno de sus
/// nodos. El procesamiento no asigna memoria ni calcula dependencias.
pub struct RenderPlan {
    nodes: Vec<Box<dyn AudioNode>>,
    // Workers and non-RT resources whose lifetime must cover this plan. They
    // are never touched by process and are destroyed by control when retired.
    retained_resources: Vec<Box<dyn Send>>,
}

const SLOT_FREE: u8 = 0;
const SLOT_WRITING: u8 = 1;
const SLOT_PENDING: u8 = 2;
const SLOT_ACTIVE: u8 = 3;
const SLOT_RETIRED: u8 = 4;
const SLOT_RECLAIMING: u8 = 5;
const NO_SLOT: usize = usize::MAX;

struct RenderPlanSlot {
    state: AtomicU8,
    plan: UnsafeCell<Option<RenderPlan>>,
}

// Safety: access to `plan` is exclusive by the state machine. The control
// thread writes FREE→WRITING and reclaims RETIRED; the audio thread alone
// reads/writes ACTIVE and publishes it as RETIRED only after finishing a block.
unsafe impl Sync for RenderPlanSlot {}

struct RenderPlanExchangeInner {
    slots: [RenderPlanSlot; 2],
    pending: AtomicUsize,
}

/// Control-plane endpoint for replacing a compiled render plan.
///
/// One control thread may publish plans. The old plan must be reclaimed with
/// `reap_retired` on that thread; it is never destroyed by the audio callback.
pub struct RenderPlanControl {
    inner: Arc<RenderPlanExchangeInner>,
    // A single control producer keeps publication and slot reservation SPSC.
    _single_producer: std::marker::PhantomData<std::cell::Cell<()>>,
}

/// Audio-thread endpoint for a plan exchange. `process` checks for a new plan
/// exactly at a block boundary and performs no allocation, lock, or drop.
pub struct RenderPlanProcessor {
    inner: Arc<RenderPlanExchangeInner>,
    active_slot: usize,
}

impl RenderPlanControl {
    /// Publishes a fully prepared plan. `Err(plan)` means a prior replacement
    /// is still pending or its retired plan has not yet been reclaimed.
    pub fn publish(&self, plan: RenderPlan) -> Result<(), RenderPlan> {
        if self.inner.pending.load(Ordering::Acquire) != NO_SLOT {
            return Err(plan);
        }

        let Some((slot_index, slot)) = self.inner.slots.iter().enumerate().find(|(_, slot)| {
            slot.state
                .compare_exchange(
                    SLOT_FREE,
                    SLOT_WRITING,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .is_ok()
        }) else {
            return Err(plan);
        };

        // SAFETY: WRITING grants exclusive control-thread access until the
        // plan is fully initialized and published with Release ordering.
        unsafe { *slot.plan.get() = Some(plan) };
        slot.state.store(SLOT_PENDING, Ordering::Release);
        self.inner.pending.store(slot_index, Ordering::Release);
        Ok(())
    }

    /// Destroys the previous plan outside real-time, freeing its slot for the
    /// next replacement. Returns whether a retired plan was reclaimed.
    pub fn reap_retired(&self) -> bool {
        for slot in &self.inner.slots {
            if slot
                .state
                .compare_exchange(
                    SLOT_RETIRED,
                    SLOT_RECLAIMING,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                // SAFETY: RECLAIMING excludes both the audio thread and any
                // publisher until the value is dropped and the slot is FREE.
                let old_plan = unsafe { (&mut *slot.plan.get()).take() };
                drop(old_plan);
                slot.state.store(SLOT_FREE, Ordering::Release);
                return true;
            }
        }
        false
    }
}

impl RenderPlanProcessor {
    /// Called by the host once per output block. Plan ownership changes only
    /// here; destruction is deferred until control calls `reap_retired`.
    pub fn process(&mut self, interleaved: &mut [f32]) -> Result<(), AudioNodeError> {
        let next = self.inner.pending.swap(NO_SLOT, Ordering::AcqRel);
        if next != NO_SLOT {
            let old = self.active_slot;
            self.active_slot = next;
            self.inner.slots[next]
                .state
                .store(SLOT_ACTIVE, Ordering::Release);
            self.inner.slots[old]
                .state
                .store(SLOT_RETIRED, Ordering::Release);
        }

        let slot = &self.inner.slots[self.active_slot];
        // SAFETY: this processor is the sole owner of the active slot until it
        // marks that slot RETIRED at a later block boundary.
        // A missing slot is an invariant violation, but the audio callback
        // must fail closed with silence instead of unwinding across FFI.
        let Some(plan) = (unsafe { (&mut *slot.plan.get()).as_mut() }) else {
            interleaved.fill(0.0);
            return Ok(());
        };
        plan.process(interleaved)
    }
}

/// Creates a render endpoint and a control endpoint around the initial plan.
/// The returned control handle is intended for the non-real-time/UI thread.
pub fn render_plan_exchange(initial: RenderPlan) -> (RenderPlanControl, RenderPlanProcessor) {
    let mut initial = Some(initial);
    let inner = Arc::new(RenderPlanExchangeInner {
        slots: std::array::from_fn(|index| RenderPlanSlot {
            state: AtomicU8::new(if index == 0 { SLOT_ACTIVE } else { SLOT_FREE }),
            plan: UnsafeCell::new(if index == 0 { initial.take() } else { None }),
        }),
        pending: AtomicUsize::new(NO_SLOT),
    });
    (
        RenderPlanControl {
            inner: Arc::clone(&inner),
            _single_producer: std::marker::PhantomData,
        },
        RenderPlanProcessor {
            inner,
            active_slot: 0,
        },
    )
}

impl RenderPlan {
    /// Keeps a resource (for example, an instrument worker feeding a node)
    /// alive for the full plan lifetime. Allocation and destruction are non-RT.
    pub fn retain_resource<T: Send + 'static>(&mut self, resource: T) {
        self.retained_resources.push(Box::new(resource));
    }

    pub fn process(&mut self, interleaved: &mut [f32]) -> Result<(), AudioNodeError> {
        for node in &mut self.nodes {
            node.process(interleaved)?;
        }
        Ok(())
    }

    pub fn process_block(&mut self, block: &mut AudioBlock) -> Result<(), AudioNodeError> {
        self.process(block.samples_mut())
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

pub struct GainNode {
    gain: f32,
}

impl GainNode {
    pub fn new(gain: f32) -> Self {
        Self { gain }
    }
}

impl AudioNode for GainNode {
    fn process(&mut self, interleaved: &mut [f32]) -> Result<(), AudioNodeError> {
        for sample in interleaved {
            *sample *= self.gain;
        }
        Ok(())
    }
}

/// Adaptador del ecualizador modular para insertarlo en una pista, bus o
/// master sin duplicar el DSP.
pub struct EqualizerNode {
    equalizer: Equalizer,
}

impl EqualizerNode {
    pub fn new(sample_rate_hz: f32, channels: usize) -> Result<Self, DspError> {
        Ok(Self {
            equalizer: Equalizer::new(sample_rate_hz, channels)?,
        })
    }

    pub fn add_band(&mut self, band: EqBandConfig) -> Result<usize, DspError> {
        self.equalizer.add_band(band)
    }
}

impl AudioNode for EqualizerNode {
    fn process(&mut self, interleaved: &mut [f32]) -> Result<(), AudioNodeError> {
        self.equalizer.process_interleaved(interleaved)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    struct DropCounter(Arc<AtomicUsize>);

    struct OffsetNode(f32);

    impl AudioNode for OffsetNode {
        fn process(&mut self, block: &mut [f32]) -> Result<(), AudioNodeError> {
            block.iter_mut().for_each(|sample| *sample += self.0);
            Ok(())
        }
    }

    struct ScaleNode(f32);

    impl AudioNode for ScaleNode {
        fn process(&mut self, block: &mut [f32]) -> Result<(), AudioNodeError> {
            block.iter_mut().for_each(|sample| *sample *= self.0);
            Ok(())
        }
    }

    impl AudioNode for DropCounter {
        fn process(&mut self, _interleaved: &mut [f32]) -> Result<(), AudioNodeError> {
            Ok(())
        }
    }

    impl Drop for DropCounter {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn processes_in_insertion_order_as_an_in_place_chain() {
        let mut builder = RenderPlanBuilder::new();
        builder.add_node(OffsetNode(3.0));
        builder.add_node(ScaleNode(2.0));
        let mut plan = builder.build();
        let mut block = [1.0, -1.0];
        plan.process(&mut block).unwrap();
        assert_eq!(block, [8.0, 4.0]);
    }

    #[test]
    fn hosts_modular_equalizer_as_a_node() {
        let mut eq = EqualizerNode::new(48_000.0, 2).unwrap();
        eq.add_band(EqBandConfig::bell(1_000.0, 3.0, 1.0)).unwrap();
        let mut plan = RenderPlanBuilder::new();
        plan.add_node(eq);
        let mut plan = plan.build();
        let mut block = [0.1; 32];
        plan.process(&mut block).unwrap();
        assert!(block.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn equalizer_bypass_preserves_signal_while_enabled_band_changes_it() {
        let mut bypassed = EqualizerNode::new(48_000.0, 2).unwrap();
        bypassed
            .add_band(EqBandConfig::bell(1_000.0, 12.0, 1.0).with_enabled(false))
            .unwrap();
        let mut active = EqualizerNode::new(48_000.0, 2).unwrap();
        active
            .add_band(EqBandConfig::bell(1_000.0, 12.0, 1.0))
            .unwrap();
        let input = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let mut bypassed_block = input;
        let mut active_block = input;
        bypassed.process(&mut bypassed_block).unwrap();
        active.process(&mut active_block).unwrap();
        assert_eq!(bypassed_block, input);
        assert!(active_block
            .iter()
            .zip(input)
            .any(|(actual, expected)| (actual - expected).abs() > 1e-6));
    }

    #[test]
    fn processes_preallocated_audio_block() {
        let mut builder = RenderPlanBuilder::new();
        builder.add_node(GainNode::new(0.5));
        let mut plan = builder.build();
        let mut block = AudioBlock::new(2, 4).unwrap();
        block.samples_mut().fill(1.0);
        plan.process_block(&mut block).unwrap();
        assert_eq!(block.channels(), 2);
        assert_eq!(block.frames(), 4);
        assert!(block.samples().iter().all(|sample| *sample == 0.5));
    }

    #[test]
    fn rejects_empty_audio_blocks_before_the_callback() {
        assert_eq!(
            AudioBlock::new(0, 64),
            Err(AudioBlockError::InvalidChannelCount)
        );
        assert_eq!(
            AudioBlock::new(2, 0),
            Err(AudioBlockError::InvalidFrameCount)
        );
    }

    #[test]
    fn ring_transfers_samples_without_allocating_after_construction() {
        let ring = SampleRingBuffer::new(4);
        assert_eq!(ring.push(&[1.0, 2.0, 3.0]), 3);
        let mut output = [0.0; 2];
        assert_eq!(ring.pop(&mut output), 2);
        assert_eq!(output, [1.0, 2.0]);
        assert_eq!(ring.push(&[4.0, 5.0, 6.0]), 3);
        let mut rest = [0.0; 4];
        assert_eq!(ring.pop(&mut rest), 4);
        assert_eq!(rest, [3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn ring_drops_new_samples_when_full_instead_of_blocking() {
        let ring = SampleRingBuffer::new(2);
        assert_eq!(ring.push(&[1.0, 2.0, 3.0]), 2);
        assert_eq!(ring.available(), 2);
        assert_eq!(ring.push(&[4.0]), 0);
    }

    #[test]
    fn render_plan_swap_occurs_at_block_boundary_and_retires_off_rt() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut initial = RenderPlanBuilder::new();
        initial.add_node(DropCounter(Arc::clone(&drops)));
        let mut initial = initial.build();
        initial.retain_resource(DropCounter(Arc::clone(&drops)));

        let mut replacement = RenderPlanBuilder::new();
        replacement.add_node(GainNode::new(2.0));
        let replacement = replacement.build();
        let (control, mut processor) = render_plan_exchange(initial);

        // El plan viejo sigue siendo el activo hasta que comienza el siguiente
        // bloque después de que el productor publica el plan completo.
        assert!(control.publish(replacement).is_ok());
        let mut block = [0.25, -0.25];
        processor.process(&mut block).unwrap();
        assert_eq!(block, [0.5, -0.5]);

        // Cambiar de plan no destruye nodos ni ejecuta Drop en tiempo real.
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        assert!(control.reap_retired());
        assert_eq!(drops.load(Ordering::Relaxed), 2);
        assert!(!control.reap_retired());
    }

    #[test]
    fn exchange_rejects_another_plan_until_retired_slot_is_reclaimed() {
        let mut builder = RenderPlanBuilder::new();
        builder.add_node(GainNode::new(1.0));
        let (control, mut processor) = render_plan_exchange(builder.build());

        let mut replacement = RenderPlanBuilder::new();
        replacement.add_node(GainNode::new(3.0));
        assert!(control.publish(replacement.build()).is_ok());
        let mut block = [1.0];
        processor.process(&mut block).unwrap();

        let mut next = RenderPlanBuilder::new();
        next.add_node(GainNode::new(4.0));
        let next = next.build();
        assert!(control.publish(next).is_err());
        assert!(control.reap_retired());
        let mut next = RenderPlanBuilder::new();
        next.add_node(GainNode::new(4.0));
        assert!(control.publish(next.build()).is_ok());
    }

    #[test]
    fn empty_active_slot_fails_closed_with_silence_without_panicking() {
        let mut builder = RenderPlanBuilder::new();
        builder.add_node(GainNode::new(2.0));
        let (_control, mut processor) = render_plan_exchange(builder.build());
        // Simulate slot corruption/invariant failure without crossing an FFI
        // boundary; the callback contract is to return silence, never unwind.
        unsafe { *processor.inner.slots[0].plan.get() = None };
        let mut block = [0.5, -0.5];
        assert!(processor.process(&mut block).is_ok());
        assert_eq!(block, [0.0, 0.0]);
    }
}
