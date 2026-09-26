//! Grafo DSP compilado para el motor de audio.
//!
//! El builder puede asignar y ordenar nodos libremente porque trabaja fuera
//! del callback. `RenderPlan::process` sólo recorre una lista plana ya
//! compilada y no resuelve dependencias ni crea buffers temporales.

use estudio_daw_dsp::{DspError, EqBandConfig, Equalizer};
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};
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
    #[error("fallo DSP: {0}")]
    Dsp(#[from] DspError),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GraphError {
    #[error("el nodo {0} no existe")]
    InvalidNode(usize),
    #[error("el grafo DSP contiene un ciclo")]
    Cycle,
}

struct PendingNode {
    dependencies: Vec<usize>,
    node: Box<dyn AudioNode>,
}

/// Builder mutable para una topología de sesión.
pub struct RenderPlanBuilder {
    nodes: Vec<PendingNode>,
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

    pub fn add_node<N>(&mut self, node: N) -> usize
    where
        N: AudioNode + 'static,
    {
        let id = self.nodes.len();
        self.nodes.push(PendingNode {
            dependencies: Vec::new(),
            node: Box::new(node),
        });
        id
    }

    /// Declara que `source` debe renderizarse antes de `target`.
    pub fn connect(&mut self, source: usize, target: usize) -> Result<(), GraphError> {
        if source >= self.nodes.len() {
            return Err(GraphError::InvalidNode(source));
        }
        let Some(target_node) = self.nodes.get_mut(target) else {
            return Err(GraphError::InvalidNode(target));
        };
        target_node.dependencies.push(source);
        Ok(())
    }

    /// Compila el DAG en orden topológico fuera del hilo de audio.
    pub fn build(self) -> Result<RenderPlan, GraphError> {
        let node_count = self.nodes.len();
        let mut indegree: Vec<usize> = self
            .nodes
            .iter()
            .map(|node| node.dependencies.len())
            .collect();
        let mut ready: Vec<usize> = indegree
            .iter()
            .enumerate()
            .filter_map(|(id, degree)| (*degree == 0).then_some(id))
            .collect();
        let mut order = Vec::with_capacity(node_count);

        while let Some(source) = ready.pop() {
            order.push(source);
            for (target, node) in self.nodes.iter().enumerate() {
                if node.dependencies.contains(&source) {
                    indegree[target] -= 1;
                    if indegree[target] == 0 {
                        ready.push(target);
                    }
                }
            }
        }
        if order.len() != node_count {
            return Err(GraphError::Cycle);
        }

        let mut nodes: Vec<Option<Box<dyn AudioNode>>> =
            self.nodes.into_iter().map(|node| Some(node.node)).collect();
        let ordered_nodes = order
            .into_iter()
            .map(|id| nodes[id].take().expect("cada nodo aparece una vez"))
            .collect();
        Ok(RenderPlan {
            nodes: ordered_nodes,
        })
    }
}

/// Plan DSP inmutable en topología, mutable sólo en el estado interno de sus
/// nodos. El procesamiento no asigna memoria ni calcula dependencias.
pub struct RenderPlan {
    nodes: Vec<Box<dyn AudioNode>>,
}

impl RenderPlan {
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

    #[test]
    fn compiles_dag_in_dependency_order() {
        let mut builder = RenderPlanBuilder::new();
        let gain_a = builder.add_node(GainNode::new(2.0));
        let gain_b = builder.add_node(GainNode::new(3.0));
        builder.connect(gain_a, gain_b).unwrap();
        let mut plan = builder.build().unwrap();
        let mut block = [1.0, -1.0];
        plan.process(&mut block).unwrap();
        assert_eq!(block, [6.0, -6.0]);
    }

    #[test]
    fn rejects_cycles_before_audio_processing() {
        let mut builder = RenderPlanBuilder::new();
        let first = builder.add_node(GainNode::new(1.0));
        let second = builder.add_node(GainNode::new(1.0));
        builder.connect(first, second).unwrap();
        builder.connect(second, first).unwrap();
        assert!(matches!(builder.build(), Err(GraphError::Cycle)));
    }

    #[test]
    fn hosts_modular_equalizer_as_a_node() {
        let mut eq = EqualizerNode::new(48_000.0, 2).unwrap();
        eq.add_band(EqBandConfig::bell(1_000.0, 3.0, 1.0)).unwrap();
        let mut plan = RenderPlanBuilder::new();
        plan.add_node(eq);
        let mut plan = plan.build().unwrap();
        let mut block = [0.1; 32];
        plan.process(&mut block).unwrap();
        assert!(block.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn processes_preallocated_audio_block() {
        let mut builder = RenderPlanBuilder::new();
        builder.add_node(GainNode::new(0.5));
        let mut plan = builder.build().unwrap();
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
}
