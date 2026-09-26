//! Backends de audio del sistema operativo.
//!
//! Este crate contiene PipeWire y no debe filtrarse hacia el modelo de sesión.
//! El único objeto que atraviesa la frontera es el `RenderPlan` ya compilado.

use estudio_daw_audio_engine::{RenderPlan, SampleRingBuffer};
use pipewire as pw;
use pw::{properties::properties, spa};
use spa::pod::Pod;
use std::io::Cursor;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipeWireStreamConfig {
    pub sample_rate: u32,
    pub channels: u32,
    pub max_buffer_frames: usize,
}

impl Default for PipeWireStreamConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            channels: 2,
            max_buffer_frames: 2_048,
        }
    }
}

#[derive(Debug, Error)]
pub enum PipeWireError {
    #[error("configuración PipeWire inválida")]
    InvalidConfig,
    #[error("error de PipeWire: {0}")]
    PipeWire(#[from] pw::Error),
    #[error("error del timer PipeWire: {0}")]
    Timer(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipeWireDuplexReport {
    pub capture_callbacks: u64,
    pub output_callbacks: u64,
    pub capture_dropped_samples: u64,
    pub output_silence_samples: u64,
}

impl PipeWireStreamConfig {
    pub fn validate(self) -> Result<Self, PipeWireError> {
        if self.sample_rate == 0 || self.channels == 0 || self.max_buffer_frames == 0 {
            return Err(PipeWireError::InvalidConfig);
        }
        Ok(self)
    }
}

/// Ejecuta un stream de salida PipeWire y procesa sus buffers con `RenderPlan`.
///
/// La función bloquea en el main loop de PipeWire. La construcción de objetos,
/// serialización de parámetros y asignaciones ocurren antes de `run()`; el
/// callback `process` sólo obtiene el buffer mapeado, lo limpia y ejecuta el
/// plan DSP compilado.
pub fn run_pipewire_output(
    config: PipeWireStreamConfig,
    mut render_plan: RenderPlan,
) -> Result<(), PipeWireError> {
    let config = config.validate()?;
    pw::init();

    let main_loop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&main_loop, None)?;
    let core = context.connect_rc(None)?;
    let stream = pw::stream::StreamBox::new(
        &core,
        "estudio-daw-output",
        properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Playback",
            *pw::keys::MEDIA_ROLE => "Music",
            *pw::keys::AUDIO_CHANNELS => config.channels.to_string(),
        },
    )?;

    let _listener = stream
        .add_local_listener_with_user_data(())
        .process(move |stream, _| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let Some(data) = buffer.datas_mut().first_mut() else {
                return;
            };
            let sample_count = {
                let Some(bytes) = data.data() else {
                    return;
                };
                // El stream fue negociado explícitamente como F32LE. PipeWire
                // entrega el bloque mapeado y `align_to_mut` sólo separa el
                // posible prefijo/sufijo no alineado sin asignar memoria.
                let (_, samples, _) = unsafe { bytes.align_to_mut::<f32>() };
                samples.fill(0.0);
                let _ = render_plan.process(samples);
                samples.len()
            };
            let chunk = data.chunk_mut();
            *chunk.offset_mut() = 0;
            *chunk.stride_mut() = (config.channels as usize * std::mem::size_of::<f32>()) as _;
            *chunk.size_mut() = (sample_count * std::mem::size_of::<f32>()) as _;
        })
        .register()?;

    let mut audio_info = spa::param::audio::AudioInfoRaw::new();
    audio_info.set_format(spa::param::audio::AudioFormat::F32LE);
    audio_info.set_rate(config.sample_rate);
    audio_info.set_channels(config.channels);
    let object = pw::spa::pod::Object {
        type_: pw::spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: pw::spa::param::ParamType::EnumFormat.as_raw(),
        properties: audio_info.into(),
    };
    let values = pw::spa::pod::serialize::PodSerializer::serialize(
        Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(object),
    )
    .expect("la especificación F32LE es serializable")
    .0
    .into_inner();
    let mut params = [Pod::from_bytes(&values).expect("el pod serializado es válido")];

    stream.connect(
        spa::utils::Direction::Output,
        None,
        pw::stream::StreamFlags::AUTOCONNECT
            | pw::stream::StreamFlags::MAP_BUFFERS
            | pw::stream::StreamFlags::RT_PROCESS,
        &mut params,
    )?;
    main_loop.run();
    Ok(())
}

/// Ejecuta captura y reproducción PipeWire con un ring SPSC entre ambos.
///
/// El stream de entrada sólo copia muestras al ring. El stream de salida
/// consume lo disponible, rellena con silencio si falta audio y ejecuta el
/// `RenderPlan`. La capacidad se reserva antes de iniciar el main loop.
pub fn run_pipewire_duplex(
    config: PipeWireStreamConfig,
    render_plan: RenderPlan,
) -> Result<(), PipeWireError> {
    run_pipewire_duplex_internal(config, render_plan, None).map(|_| ())
}

/// Ejecuta el duplex durante una duración finita y devuelve métricas de la
/// prueba. Es el smoke test que usará la CLI para detectar problemas de
/// conexión sin dejar un main loop bloqueado indefinidamente.
pub fn run_pipewire_duplex_for(
    config: PipeWireStreamConfig,
    render_plan: RenderPlan,
    duration: Duration,
) -> Result<PipeWireDuplexReport, PipeWireError> {
    run_pipewire_duplex_internal(config, render_plan, Some(duration))
}

fn run_pipewire_duplex_internal(
    config: PipeWireStreamConfig,
    mut render_plan: RenderPlan,
    duration: Option<Duration>,
) -> Result<PipeWireDuplexReport, PipeWireError> {
    let config = config.validate()?;
    pw::init();
    let main_loop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&main_loop, None)?;
    let core = context.connect_rc(None)?;
    let ring = Arc::new(SampleRingBuffer::new(
        config.channels as usize * config.max_buffer_frames * 4,
    ));
    let capture_callbacks = Arc::new(AtomicU64::new(0));
    let capture_dropped_samples = Arc::new(AtomicU64::new(0));
    let output_callbacks = Arc::new(AtomicU64::new(0));
    let output_silence_samples = Arc::new(AtomicU64::new(0));

    let capture_stream = pw::stream::StreamBox::new(
        &core,
        "estudio-daw-input",
        properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Music",
            *pw::keys::AUDIO_CHANNELS => config.channels.to_string(),
        },
    )?;
    let capture_ring = Arc::clone(&ring);
    let capture_callbacks_counter = Arc::clone(&capture_callbacks);
    let capture_dropped_counter = Arc::clone(&capture_dropped_samples);
    let _capture_listener = capture_stream
        .add_local_listener_with_user_data(())
        .process(move |stream, _| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let Some(data) = buffer.datas_mut().first_mut() else {
                return;
            };
            let valid_bytes = data.chunk().size() as usize;
            let Some(bytes) = data.data() else {
                return;
            };
            let valid_bytes = valid_bytes.min(bytes.len());
            let (_, samples, _) = unsafe { bytes[..valid_bytes].align_to::<f32>() };
            capture_callbacks_counter.fetch_add(1, Ordering::Relaxed);
            let pushed = capture_ring.push(samples);
            capture_dropped_counter.fetch_add((samples.len() - pushed) as u64, Ordering::Relaxed);
        })
        .register()?;

    let output_stream = pw::stream::StreamBox::new(
        &core,
        "estudio-daw-output",
        properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Playback",
            *pw::keys::MEDIA_ROLE => "Music",
            *pw::keys::AUDIO_CHANNELS => config.channels.to_string(),
        },
    )?;
    let output_ring = Arc::clone(&ring);
    let output_callbacks_counter = Arc::clone(&output_callbacks);
    let output_silence_counter = Arc::clone(&output_silence_samples);
    let _output_listener = output_stream
        .add_local_listener_with_user_data(())
        .process(move |stream, _| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let Some(data) = buffer.datas_mut().first_mut() else {
                return;
            };
            let valid_bytes = data.chunk().size() as usize;
            let Some(bytes) = data.data() else {
                return;
            };
            let valid_bytes = valid_bytes.min(bytes.len());
            let (_, samples, _) = unsafe { bytes[..valid_bytes].align_to_mut::<f32>() };
            output_callbacks_counter.fetch_add(1, Ordering::Relaxed);
            let copied = output_ring.pop(samples);
            samples[copied..].fill(0.0);
            output_silence_counter.fetch_add((samples.len() - copied) as u64, Ordering::Relaxed);
            let _ = render_plan.process(samples);
        })
        .register()?;

    let mut capture_params = audio_params(config);
    capture_stream.connect(
        spa::utils::Direction::Input,
        None,
        pw::stream::StreamFlags::AUTOCONNECT
            | pw::stream::StreamFlags::MAP_BUFFERS
            | pw::stream::StreamFlags::RT_PROCESS,
        &mut capture_params,
    )?;
    let mut output_params = audio_params(config);
    output_stream.connect(
        spa::utils::Direction::Output,
        None,
        pw::stream::StreamFlags::AUTOCONNECT
            | pw::stream::StreamFlags::MAP_BUFFERS
            | pw::stream::StreamFlags::RT_PROCESS,
        &mut output_params,
    )?;
    let _timer = if let Some(duration) = duration {
        let loop_to_quit = main_loop.clone();
        let timer = main_loop.loop_().add_timer(move |_| loop_to_quit.quit());
        timer
            .update_timer(Some(duration), None)
            .into_result()
            .map_err(|error| PipeWireError::Timer(format!("{error:?}")))?;
        Some(timer)
    } else {
        None
    };
    main_loop.run();
    Ok(PipeWireDuplexReport {
        capture_callbacks: capture_callbacks.load(Ordering::Relaxed),
        output_callbacks: output_callbacks.load(Ordering::Relaxed),
        capture_dropped_samples: capture_dropped_samples.load(Ordering::Relaxed),
        output_silence_samples: output_silence_samples.load(Ordering::Relaxed),
    })
}

fn audio_params(config: PipeWireStreamConfig) -> [&'static Pod; 1] {
    let mut audio_info = spa::param::audio::AudioInfoRaw::new();
    audio_info.set_format(spa::param::audio::AudioFormat::F32LE);
    audio_info.set_rate(config.sample_rate);
    audio_info.set_channels(config.channels);
    let object = pw::spa::pod::Object {
        type_: pw::spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: pw::spa::param::ParamType::EnumFormat.as_raw(),
        properties: audio_info.into(),
    };
    let values = pw::spa::pod::serialize::PodSerializer::serialize(
        Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(object),
    )
    .expect("la especificación F32LE es serializable")
    .0
    .into_inner();
    let bytes: &'static [u8] = Box::leak(values.into_boxed_slice());
    [Pod::from_bytes(bytes).expect("pod válido")]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_pipewire_stream_configuration() {
        assert!(PipeWireStreamConfig::default().validate().is_ok());
        assert_eq!(
            PipeWireStreamConfig {
                sample_rate: 0,
                channels: 2,
                max_buffer_frames: 2_048,
            }
            .validate()
            .unwrap_err()
            .to_string(),
            "configuración PipeWire inválida"
        );
    }
}
