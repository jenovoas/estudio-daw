//! Instrumento MIDI nativo inicial para validar la ruta de audio en tiempo real.
//!
//! Este crate no pretende sustituir un sampler ni un SoundFont: produce una
//! onda sinusoidal polifónica para probar captura MIDI, voz, transporte y salida
//! PipeWire antes de integrar instrumentos con muestras o plugins.

use estudio_daw_audio_engine::{AudioNode, AudioNodeError};
use std::{
    cell::UnsafeCell,
    f32::consts::TAU,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use thiserror::Error;

pub const MIDI_EVENT_CAPACITY: usize = 256;
const POLYPHONY: usize = 16;
const ATTACK_SECONDS: f32 = 0.005;
const RELEASE_SECONDS: f32 = 0.12;
const VOICE_GAIN: f32 = 0.20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SynthMidiEvent {
    NoteOn { channel: u8, note: u8, velocity: u8 },
    NoteOff { channel: u8, note: u8 },
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SynthConfigError {
    #[error("la frecuencia de muestreo debe ser mayor que cero")]
    InvalidSampleRate,
    #[error("la cantidad de canales debe ser mayor que cero")]
    InvalidChannelCount,
}

struct EventRing {
    slots: Box<[UnsafeCell<SynthMidiEvent>]>,
    read: AtomicUsize,
    write: AtomicUsize,
    dropped: AtomicUsize,
}

// SAFETY: los extremos de la cola respetan el contrato SPSC: exactamente un
// productor escribe slots y exactamente un consumidor los lee. Los índices
// Release/Acquire publican el contenido antes de que el otro extremo lo use.
unsafe impl Send for EventRing {}
unsafe impl Sync for EventRing {}

/// Extremo productor. Se mueve al hilo de entrada MIDI y no se clona, para
/// mantener verificable el contrato de un solo productor.
pub struct SynthEventSender(Arc<EventRing>);

/// Extremo consumidor, propiedad del instrumento y usado sólo en el callback.
pub struct SynthEventReceiver(Arc<EventRing>);

/// Reserva la cola antes de iniciar audio y entrega extremos separados por rol.
pub fn midi_event_queue() -> (SynthEventSender, SynthEventReceiver) {
    let ring = Arc::new(EventRing {
        slots: (0..MIDI_EVENT_CAPACITY)
            .map(|_| {
                UnsafeCell::new(SynthMidiEvent::NoteOff {
                    channel: 0,
                    note: 0,
                })
            })
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        read: AtomicUsize::new(0),
        write: AtomicUsize::new(0),
        dropped: AtomicUsize::new(0),
    });
    (
        SynthEventSender(Arc::clone(&ring)),
        SynthEventReceiver(ring),
    )
}

impl SynthEventSender {
    /// Publica una nota sin asignar ni bloquear. Devuelve `false` y cuenta el
    /// evento descartado si la cola ya está llena.
    pub fn try_send(&mut self, event: SynthMidiEvent) -> bool {
        let write = self.0.write.load(Ordering::Relaxed);
        let read = self.0.read.load(Ordering::Acquire);
        if write.wrapping_sub(read) >= MIDI_EVENT_CAPACITY {
            self.0.dropped.fetch_add(1, Ordering::Relaxed);
            return false;
        }
        let slot = write % MIDI_EVENT_CAPACITY;
        // SAFETY: sólo este sender escribe el índice `write`; el consumidor no
        // puede reutilizarlo hasta publicar el nuevo read index.
        unsafe { *self.0.slots[slot].get() = event };
        self.0.write.store(write.wrapping_add(1), Ordering::Release);
        true
    }

    pub fn dropped_events(&self) -> usize {
        self.0.dropped.load(Ordering::Relaxed)
    }
}

impl SynthEventReceiver {
    fn try_receive(&mut self) -> Option<SynthMidiEvent> {
        let read = self.0.read.load(Ordering::Relaxed);
        let write = self.0.write.load(Ordering::Acquire);
        if read == write {
            return None;
        }
        let slot = read % MIDI_EVENT_CAPACITY;
        // SAFETY: el productor publicó `write` con Release y no vuelve a tocar
        // este slot hasta que el consumidor publique el nuevo read index.
        let event = unsafe { *self.0.slots[slot].get() };
        self.0.read.store(read.wrapping_add(1), Ordering::Release);
        Some(event)
    }
}

#[derive(Clone, Copy, Default)]
struct Voice {
    active: bool,
    releasing: bool,
    channel: u8,
    note: u8,
    velocity: f32,
    phase: f32,
    envelope: f32,
}

/// Fuente de instrumento polifónica mínima. Cada bloque limpia su buffer y
/// renderiza las voces activas; así una entrada capturada no se monitoriza por
/// accidente mientras este instrumento ocupa el plan de prueba.
pub struct SineSynthNode {
    sample_rate: f32,
    channels: usize,
    attack_step: f32,
    release_step: f32,
    frequencies: [f32; 128],
    receiver: SynthEventReceiver,
    voices: [Voice; POLYPHONY],
    next_voice: usize,
}

impl SineSynthNode {
    pub fn new(
        sample_rate: u32,
        channels: usize,
        receiver: SynthEventReceiver,
    ) -> Result<Self, SynthConfigError> {
        if sample_rate == 0 {
            return Err(SynthConfigError::InvalidSampleRate);
        }
        if channels == 0 {
            return Err(SynthConfigError::InvalidChannelCount);
        }

        let sample_rate = sample_rate as f32;
        // Precalcular las 128 frecuencias evita `powf` al recibir una nota en
        // tiempo real. La afinación de referencia es A4=440 Hz.
        let frequencies =
            std::array::from_fn(|note| 440.0 * 2.0_f32.powf((note as f32 - 69.0) / 12.0));

        Ok(Self {
            sample_rate,
            channels,
            attack_step: 1.0 / (sample_rate * ATTACK_SECONDS).max(1.0),
            release_step: 1.0 / (sample_rate * RELEASE_SECONDS).max(1.0),
            frequencies,
            receiver,
            voices: [Voice::default(); POLYPHONY],
            next_voice: 0,
        })
    }

    fn apply_event(&mut self, event: SynthMidiEvent) {
        match event {
            SynthMidiEvent::NoteOn {
                channel,
                note,
                velocity: 0,
            }
            | SynthMidiEvent::NoteOff { channel, note } => {
                for voice in &mut self.voices {
                    if voice.active && voice.channel == channel && voice.note == note {
                        voice.releasing = true;
                    }
                }
            }
            SynthMidiEvent::NoteOn {
                channel,
                note,
                velocity,
            } => {
                let index = self
                    .voices
                    .iter()
                    .position(|voice| !voice.active)
                    .unwrap_or_else(|| {
                        let index = self.next_voice;
                        self.next_voice = (self.next_voice + 1) % POLYPHONY;
                        index
                    });
                self.voices[index] = Voice {
                    active: true,
                    releasing: false,
                    channel,
                    note,
                    velocity: velocity as f32 / 127.0,
                    phase: 0.0,
                    envelope: 0.0,
                };
            }
        }
    }
}

impl AudioNode for SineSynthNode {
    fn process(&mut self, interleaved: &mut [f32]) -> Result<(), AudioNodeError> {
        if interleaved.len() % self.channels != 0 {
            return Err(AudioNodeError::InvalidBlockLength);
        }

        while let Some(event) = self.receiver.try_receive() {
            self.apply_event(event);
        }

        interleaved.fill(0.0);
        for frame in interleaved.chunks_exact_mut(self.channels) {
            let mut mixed = 0.0_f32;
            for voice in &mut self.voices {
                if !voice.active {
                    continue;
                }
                if voice.releasing {
                    voice.envelope -= self.release_step;
                    if voice.envelope <= 0.0 {
                        *voice = Voice::default();
                        continue;
                    }
                } else {
                    voice.envelope = (voice.envelope + self.attack_step).min(1.0);
                }

                let increment = TAU * self.frequencies[voice.note as usize] / self.sample_rate;
                mixed += voice.phase.sin() * voice.envelope * voice.velocity * VOICE_GAIN;
                voice.phase += increment;
                if voice.phase >= TAU {
                    voice.phase -= TAU;
                }
            }
            for sample in frame {
                *sample = mixed;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use estudio_daw_audio_engine::AudioNode;

    #[test]
    fn note_events_render_a_finite_stereo_signal_and_release() {
        let (mut sender, receiver) = midi_event_queue();
        let mut synth = SineSynthNode::new(48_000, 2, receiver).unwrap();
        let mut block = [0.0; 512];
        assert!(sender.try_send(SynthMidiEvent::NoteOn {
            channel: 0,
            note: 69,
            velocity: 100,
        }));
        synth.process(&mut block).unwrap();
        assert!(block.iter().any(|sample| sample.abs() > 0.001));
        assert!(block.iter().all(|sample| sample.is_finite()));
        assert!(block.chunks_exact(2).all(|frame| frame[0] == frame[1]));

        assert!(sender.try_send(SynthMidiEvent::NoteOff {
            channel: 0,
            note: 69,
        }));
        let mut release = vec![0.0; 48_000 * 2 / 4];
        synth.process(&mut release).unwrap();
        // El cuerpo del release conserva sonido; comprobamos silencio sólo al
        // final del bloque, tras superar ampliamente sus 120 ms configurados.
        assert!(release[release.len() - 1_024..]
            .iter()
            .all(|sample| sample.abs() < 1.0e-6));
    }

    #[test]
    fn midi_note_69_uses_the_a4_reference_pitch() {
        let (mut sender, receiver) = midi_event_queue();
        let mut synth = SineSynthNode::new(48_000, 2, receiver).unwrap();
        assert!(sender.try_send(SynthMidiEvent::NoteOn {
            channel: 0,
            note: 69,
            velocity: 127,
        }));
        let mut block = vec![0.0; 4_800 * 2];
        synth.process(&mut block).unwrap();

        // Diez centésimas de segundo a 440 Hz contienen 44 ciclos; se tolera
        // un ciclo por la fase inicial, que siempre parte de cero.
        let mut frames = block.chunks_exact(2).map(|frame| frame[0]);
        let mut previous = frames.next().unwrap();
        let mut positive_crossings = 0;
        for current in frames {
            if previous <= 0.0 && current > 0.0 {
                positive_crossings += 1;
            }
            previous = current;
        }
        assert!((43..=45).contains(&positive_crossings));
    }

    #[test]
    fn queue_is_bounded_and_reports_overflow() {
        let (mut sender, mut receiver) = midi_event_queue();
        for _ in 0..MIDI_EVENT_CAPACITY {
            assert!(sender.try_send(SynthMidiEvent::NoteOn {
                channel: 0,
                note: 60,
                velocity: 1,
            }));
        }
        assert!(!sender.try_send(SynthMidiEvent::NoteOff {
            channel: 0,
            note: 60,
        }));
        assert_eq!(sender.dropped_events(), 1);
        assert!(matches!(
            receiver.try_receive(),
            Some(SynthMidiEvent::NoteOn { .. })
        ));
        assert!(sender.try_send(SynthMidiEvent::NoteOff {
            channel: 0,
            note: 60,
        }));
    }

    #[test]
    fn spsc_queue_preserves_order_across_threads() {
        const EVENT_COUNT: usize = 20_000;
        let (mut sender, mut receiver) = midi_event_queue();
        let consumer = std::thread::spawn(move || {
            for index in 0..EVENT_COUNT {
                loop {
                    if let Some(event) = receiver.try_receive() {
                        assert_eq!(
                            event,
                            SynthMidiEvent::NoteOn {
                                channel: 0,
                                note: (index % 128) as u8,
                                velocity: 100,
                            }
                        );
                        break;
                    }
                    std::thread::yield_now();
                }
            }
        });

        for index in 0..EVENT_COUNT {
            let event = SynthMidiEvent::NoteOn {
                channel: 0,
                note: (index % 128) as u8,
                velocity: 100,
            };
            while !sender.try_send(event) {
                std::thread::yield_now();
            }
        }
        consumer.join().unwrap();
    }

    #[test]
    fn rejects_partial_interleaved_frames() {
        let (_, receiver) = midi_event_queue();
        let mut synth = SineSynthNode::new(48_000, 2, receiver).unwrap();
        assert!(matches!(
            synth.process(&mut [0.0]),
            Err(AudioNodeError::InvalidBlockLength)
        ));
    }
}
