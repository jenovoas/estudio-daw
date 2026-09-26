//! Backends de audio del sistema operativo.
//!
//! Este crate contiene PipeWire y no debe filtrarse hacia el modelo de sesión.
//! El único objeto que atraviesa la frontera es el `RenderPlan` ya compilado.

use estudio_daw_audio_engine::{RenderPlan, SampleRingBuffer};
use pipewire as pw;
use pw::{properties::properties, spa};
use spa::pod::Pod;
use std::io::Cursor;
use std::fs::File;
use std::io::{self, Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipeWireStreamConfig {
    pub sample_rate: u32,
    pub channels: u32,
    pub period_frames: usize,
    pub max_buffer_frames: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PipeWireTargets {
    pub capture_node: Option<u64>,
    pub playback_node: Option<u64>,
}

impl Default for PipeWireStreamConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            channels: 2,
            period_frames: 32,
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
    #[error("error de captura WAV: {0}")]
    Capture(#[from] io::Error),
    #[error("el hilo de captura WAV terminó inesperadamente")]
    CaptureWorkerPanic,
}

/// Resultado de una captura escrita por `WavCaptureRecorder`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WavCaptureReport {
    pub captured_samples: u64,
    pub dropped_samples: u64,
}

/// Escritor WAV desacoplado del callback de audio.
///
/// `push()` es la única operación permitida desde el hilo RT: escribe en un
/// ring SPSC preasignado y no reserva memoria ni toca el sistema de archivos.
/// Un hilo dedicado consume el ring y genera un WAV IEEE-float de 32 bits.
pub struct WavCaptureRecorder {
    ring: Arc<SampleRingBuffer>,
    stop: Arc<std::sync::atomic::AtomicBool>,
    captured_samples: Arc<AtomicU64>,
    dropped_samples: Arc<AtomicU64>,
    worker: Option<JoinHandle<io::Result<()>>>,
}

impl WavCaptureRecorder {
    pub fn new(
        path: impl Into<std::path::PathBuf>,
        sample_rate: u32,
        channels: u16,
        capacity_frames: usize,
    ) -> Result<Self, PipeWireError> {
        if sample_rate == 0 || channels == 0 || capacity_frames == 0 {
            return Err(PipeWireError::InvalidConfig);
        }
        let path = path.into();
        let ring = Arc::new(SampleRingBuffer::new(
            capacity_frames.saturating_mul(channels as usize),
        ));
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let captured_samples = Arc::new(AtomicU64::new(0));
        let dropped_samples = Arc::new(AtomicU64::new(0));
        let worker_ring = Arc::clone(&ring);
        let worker_stop = Arc::clone(&stop);
        let worker = thread::Builder::new()
            .name("estudio-wav-writer".into())
            .spawn(move || write_wav(worker_ring, worker_stop, path, sample_rate, channels))
            .map_err(PipeWireError::Capture)?;

        Ok(Self {
            ring,
            stop,
            captured_samples,
            dropped_samples,
            worker: Some(worker),
        })
    }

    /// Copia muestras al ring sin bloquear ni asignar memoria.
    pub fn push(&self, samples: &[f32]) -> usize {
        let pushed = self.ring.push(samples);
        self.captured_samples
            .fetch_add(pushed as u64, Ordering::Relaxed);
        self.dropped_samples
            .fetch_add((samples.len() - pushed) as u64, Ordering::Relaxed);
        pushed
    }

    pub fn finish(mut self) -> Result<WavCaptureReport, PipeWireError> {
        self.stop.store(true, Ordering::Release);
        let worker_result = self
            .worker
            .take()
            .expect("el escritor WAV debe existir")
            .join()
            .map_err(|_| PipeWireError::CaptureWorkerPanic)?;
        worker_result.map_err(PipeWireError::Capture)?;
        Ok(WavCaptureReport {
            captured_samples: self.captured_samples.load(Ordering::Relaxed),
            dropped_samples: self.dropped_samples.load(Ordering::Relaxed),
        })
    }
}

impl Drop for WavCaptureRecorder {
    fn drop(&mut self) {
        if self.worker.is_some() {
            self.stop.store(true, Ordering::Release);
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }
}

fn write_wav(
    ring: Arc<SampleRingBuffer>,
    stop: Arc<std::sync::atomic::AtomicBool>,
    path: std::path::PathBuf,
    sample_rate: u32,
    channels: u16,
) -> io::Result<()> {
    let mut file = File::create(path)?;
    // Reservamos el encabezado y actualizamos sus tamaños al finalizar.
    file.write_all(&[0; 44])?;
    let mut block = vec![0.0_f32; 16_384];
    let mut data_bytes = 0_u32;
    loop {
        let count = ring.pop(&mut block);
        if count > 0 {
            let bytes = samples_as_le_bytes(&block[..count]);
            file.write_all(&bytes)?;
            data_bytes = data_bytes.saturating_add(bytes.len() as u32);
        } else if stop.load(Ordering::Acquire) {
            break;
        } else {
            thread::yield_now();
        }
    }

    file.seek(SeekFrom::Start(0))?;
    write_wav_header(&mut file, sample_rate, channels, data_bytes)?;
    file.flush()
}

fn samples_as_le_bytes(samples: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(samples.len() * 4);
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

fn write_wav_header(
    file: &mut File,
    sample_rate: u32,
    channels: u16,
    data_bytes: u32,
) -> io::Result<()> {
    let riff_size = 36_u32.saturating_add(data_bytes);
    let byte_rate = sample_rate
        .saturating_mul(u32::from(channels))
        .saturating_mul(4);
    let block_align = channels.saturating_mul(4);
    file.write_all(b"RIFF")?;
    file.write_all(&riff_size.to_le_bytes())?;
    file.write_all(b"WAVEfmt ")?;
    file.write_all(&16_u32.to_le_bytes())?;
    file.write_all(&3_u16.to_le_bytes())?; // IEEE float
    file.write_all(&channels.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&32_u16.to_le_bytes())?;
    file.write_all(b"data")?;
    file.write_all(&data_bytes.to_le_bytes())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipeWireDuplexReport {
    pub capture_callbacks: u64,
    pub output_callbacks: u64,
    pub capture_total_samples: u64,
    pub output_total_samples: u64,
    pub capture_last_samples: u64,
    pub output_last_samples: u64,
    pub capture_dropped_samples: u64,
    pub output_silence_samples: u64,
}

impl PipeWireStreamConfig {
    pub fn validate(self) -> Result<Self, PipeWireError> {
        if self.sample_rate == 0
            || self.channels == 0
            || self.period_frames == 0
            || self.max_buffer_frames < self.period_frames
        {
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
            *pw::keys::NODE_LATENCY => format!("{}/{}", config.period_frames, config.sample_rate),
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
    run_pipewire_duplex_internal(config, render_plan, None, PipeWireTargets::default()).map(|_| ())
}

/// Ejecuta el duplex durante una duración finita y devuelve métricas de la
/// prueba. Es el smoke test que usará la CLI para detectar problemas de
/// conexión sin dejar un main loop bloqueado indefinidamente.
pub fn run_pipewire_duplex_for(
    config: PipeWireStreamConfig,
    render_plan: RenderPlan,
    duration: Duration,
) -> Result<PipeWireDuplexReport, PipeWireError> {
    run_pipewire_duplex_for_targets(config, render_plan, duration, PipeWireTargets::default())
}

pub fn run_pipewire_duplex_for_targets(
    config: PipeWireStreamConfig,
    render_plan: RenderPlan,
    duration: Duration,
    targets: PipeWireTargets,
) -> Result<PipeWireDuplexReport, PipeWireError> {
    run_pipewire_duplex_internal(config, render_plan, Some(duration), targets)
}

fn run_pipewire_duplex_internal(
    config: PipeWireStreamConfig,
    mut render_plan: RenderPlan,
    duration: Option<Duration>,
    targets: PipeWireTargets,
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
    let capture_total_samples = Arc::new(AtomicU64::new(0));
    let capture_last_samples = Arc::new(AtomicU64::new(0));
    let capture_dropped_samples = Arc::new(AtomicU64::new(0));
    let output_callbacks = Arc::new(AtomicU64::new(0));
    let output_total_samples = Arc::new(AtomicU64::new(0));
    let output_last_samples = Arc::new(AtomicU64::new(0));
    let output_silence_samples = Arc::new(AtomicU64::new(0));

    let mut capture_properties = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Music",
            *pw::keys::AUDIO_CHANNELS => config.channels.to_string(),
            *pw::keys::NODE_LATENCY => format!("{}/{}", config.period_frames, config.sample_rate),
    };
    if let Some(node) = targets.capture_node {
        capture_properties.insert(*pw::keys::TARGET_OBJECT, node.to_string());
    }
    let capture_stream =
        pw::stream::StreamBox::new(&core, "estudio-daw-input", capture_properties)?;
    let capture_ring = Arc::clone(&ring);
    let capture_callbacks_counter = Arc::clone(&capture_callbacks);
    let capture_total_counter = Arc::clone(&capture_total_samples);
    let capture_last_counter = Arc::clone(&capture_last_samples);
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
            capture_total_counter.fetch_add(samples.len() as u64, Ordering::Relaxed);
            capture_last_counter.store(samples.len() as u64, Ordering::Relaxed);
            let pushed = capture_ring.push(samples);
            capture_dropped_counter.fetch_add((samples.len() - pushed) as u64, Ordering::Relaxed);
        })
        .register()?;

    let mut output_properties = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Playback",
        *pw::keys::MEDIA_ROLE => "Music",
        *pw::keys::AUDIO_CHANNELS => config.channels.to_string(),
    };
    if let Some(node) = targets.playback_node {
        output_properties.insert(*pw::keys::TARGET_OBJECT, node.to_string());
    }
    let output_stream = pw::stream::StreamBox::new(&core, "estudio-daw-output", output_properties)?;
    let output_ring = Arc::clone(&ring);
    let output_callbacks_counter = Arc::clone(&output_callbacks);
    let output_total_counter = Arc::clone(&output_total_samples);
    let output_last_counter = Arc::clone(&output_last_samples);
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
            let Some(bytes) = data.data() else {
                return;
            };
            // En un buffer de salida el chunk puede venir con size=0 porque
            // todavía no existe contenido producido. La capacidad mapeada es
            // maxsize; la acotamos al bloque configurado para no procesar
            // memoria de más si PipeWire entrega un pool sobredimensionado.
            let valid_bytes = (config.period_frames * config.channels as usize)
                .saturating_mul(std::mem::size_of::<f32>())
                .min(bytes.len());
            let (_, samples, _) = unsafe { bytes[..valid_bytes].align_to_mut::<f32>() };
            output_callbacks_counter.fetch_add(1, Ordering::Relaxed);
            output_total_counter.fetch_add(samples.len() as u64, Ordering::Relaxed);
            output_last_counter.store(samples.len() as u64, Ordering::Relaxed);
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
        capture_total_samples: capture_total_samples.load(Ordering::Relaxed),
        output_total_samples: output_total_samples.load(Ordering::Relaxed),
        capture_last_samples: capture_last_samples.load(Ordering::Relaxed),
        output_last_samples: output_last_samples.load(Ordering::Relaxed),
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
                period_frames: 32,
                max_buffer_frames: 2_048,
            }
            .validate()
            .unwrap_err()
            .to_string(),
            "configuración PipeWire inválida"
        );
    }

    #[test]
    fn wav_recorder_writes_float_header_and_samples_off_rt_thread() {
        let path = std::env::temp_dir().join(format!(
            "estudio-daw-test-{}-{}.wav",
            std::process::id(),
            std::thread::current().name().unwrap_or("audio")
        ));
        let recorder = WavCaptureRecorder::new(&path, 48_000, 2, 64).unwrap();
        let source = [0.0_f32, 0.25, -0.5, 1.0];
        assert_eq!(recorder.push(&source), source.len());
        let report = recorder.finish().unwrap();
        assert_eq!(report.captured_samples, 4);
        assert_eq!(report.dropped_samples, 0);

        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[36..40], b"data");
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 16);
        assert_eq!(&bytes[44..48], &0.0_f32.to_le_bytes());
        assert_eq!(&bytes[48..52], &0.25_f32.to_le_bytes());
        std::fs::remove_file(path).unwrap();
    }
}
