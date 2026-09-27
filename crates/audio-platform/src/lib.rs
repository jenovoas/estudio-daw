//! Backends de audio del sistema operativo.
//!
//! Este crate contiene PipeWire y no debe filtrarse hacia el modelo de sesión.
//! El único objeto que atraviesa la frontera es el `RenderPlan` ya compilado.

use estudio_daw_audio_engine::{
    render_plan_exchange, RenderPlan, RenderPlanProcessor, SampleRingBuffer,
};
use pipewire as pw;
use pw::{properties::properties, spa};
use spa::pod::Pod;
use std::fs::File;
use std::io::Cursor;
use std::io::{self, Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use std::time::Instant;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipeWireStreamConfig {
    pub sample_rate: u32,
    pub channels: u32,
    pub period_frames: usize,
    pub max_buffer_frames: usize,
    /// El destino de captura es el monitor de un nodo sink, no una fuente física.
    pub capture_sink_monitor: bool,
    /// Fuerza temporalmente el quantum del grafo mientras el stream vive.
    /// Se usa para capturas de puertos monitor que de otro modo pueden
    /// renegociar el grafo a bloques de una muestra.
    pub force_graph_quantum: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PipeWireTargets {
    /// `target.object` de PipeWire espera `node.name` o `object.serial`, no el
    /// object id efímero que devuelve la enumeración.
    pub capture_node: Option<String>,
    pub playback_node: Option<String>,
}

impl Default for PipeWireStreamConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            channels: 2,
            // 256 frames is a practical low-latency desktop starting point;
            // smaller quanta can increase PipeWire scheduling errors.
            period_frames: 256,
            max_buffer_frames: 2_048,
            capture_sink_monitor: false,
            force_graph_quantum: None,
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
    #[error("el escritor WAV todavía tiene referencias activas")]
    CaptureStillInUse,
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
    pub output_last_requested_frames: u64,
    pub capture_dropped_samples: u64,
    pub output_silence_samples: u64,
    /// Microsegundos desde el inicio de la función hasta el primer callback de captura.
    pub capture_start_delay_micros: u64,
}

impl PipeWireStreamConfig {
    pub fn validate(self) -> Result<Self, PipeWireError> {
        if self.sample_rate == 0
            || self.channels == 0
            || self.period_frames == 0
            || self.max_buffer_frames < self.period_frames
            || self.force_graph_quantum == Some(0)
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
    render_plan: RenderPlan,
) -> Result<(), PipeWireError> {
    let (_control, processor) = render_plan_exchange(render_plan);
    run_pipewire_output_controlled(config, processor)
}

/// Ejecuta la salida con un procesador que admite reemplazos desde su
/// `RenderPlanControl` asociado. El plan nuevo se activa en un límite de bloque.
pub fn run_pipewire_output_controlled(
    config: PipeWireStreamConfig,
    mut processor: RenderPlanProcessor,
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
            let requested_frames = buffer.requested();
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
                let valid_bytes = output_buffer_bytes(
                    requested_frames,
                    config.period_frames,
                    config.channels as usize,
                    bytes.len(),
                );
                let (_, samples, _) = unsafe { bytes[..valid_bytes].align_to_mut::<f32>() };
                samples.fill(0.0);
                let _ = processor.process(samples);
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

/// Ejecuta una salida PipeWire hasta que el control plane solicita detenerla.
/// `ready` confirma que PipeWire aceptó el stream; el callback sólo consulta
/// buffers y el RenderPlan, nunca este canal ni el flag de parada.
pub fn run_pipewire_output_until(
    config: PipeWireStreamConfig,
    mut processor: RenderPlanProcessor,
    playback_node: Option<String>,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    ready: std::sync::mpsc::SyncSender<Result<(), String>>,
) -> Result<(), PipeWireError> {
    let config = config.validate()?;
    pw::init();

    let main_loop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&main_loop, None)?;
    let core = context.connect_rc(None)?;
    let mut stream_properties = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Playback",
        *pw::keys::MEDIA_ROLE => "Music",
        *pw::keys::AUDIO_CHANNELS => config.channels.to_string(),
        *pw::keys::NODE_LATENCY => format!("{}/{}", config.period_frames, config.sample_rate),
    };
    if let Some(node) = playback_node {
        stream_properties.insert(*pw::keys::TARGET_OBJECT, node);
    }
    let stream = pw::stream::StreamBox::new(&core, "estudio-daw-output", stream_properties)?;
    let paused_in_callback = Arc::clone(&paused);
    let _listener = stream
        .add_local_listener_with_user_data(())
        .process(move |stream, _| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let requested_frames = buffer.requested();
            let Some(data) = buffer.datas_mut().first_mut() else {
                return;
            };
            let sample_count = {
                let Some(bytes) = data.data() else {
                    return;
                };
                let valid_bytes = output_buffer_bytes(
                    requested_frames,
                    config.period_frames,
                    config.channels as usize,
                    bytes.len(),
                );
                let (_, samples, _) = unsafe { bytes[..valid_bytes].align_to_mut::<f32>() };
                // A paused transport must not advance synth envelopes or pop
                // instrument PCM. Only write silence until resume.
                process_output_block(
                    &mut processor,
                    samples,
                    paused_in_callback.load(Ordering::Acquire),
                );
                samples.len()
            };
            let chunk = data.chunk_mut();
            *chunk.offset_mut() = 0;
            *chunk.stride_mut() = (config.channels as usize * std::mem::size_of::<f32>()) as _;
            *chunk.size_mut() = (sample_count * std::mem::size_of::<f32>()) as _;
        })
        .register()?;

    let mut params = audio_params(config);
    stream.connect(
        spa::utils::Direction::Output,
        None,
        pw::stream::StreamFlags::AUTOCONNECT
            | pw::stream::StreamFlags::MAP_BUFFERS
            | pw::stream::StreamFlags::RT_PROCESS,
        &mut params,
    )?;
    let _ = ready.send(Ok(()));

    let stop_check = Arc::clone(&stop);
    let loop_to_quit = main_loop.clone();
    let timer = main_loop.loop_().add_timer(move |_| {
        if stop_check.load(Ordering::Acquire) {
            loop_to_quit.quit();
        }
    });
    timer
        .update_timer(
            Some(Duration::from_millis(2)),
            Some(Duration::from_millis(2)),
        )
        .into_result()
        .map_err(|error| PipeWireError::Timer(format!("{error:?}")))?;
    main_loop.run();
    drop(_listener);
    drop(stream);
    Ok(())
}

/// Captura una fuente física hasta que se solicita detenerla. El callback sólo
/// copia F32 estéreo al ring SPSC; la conversión/ruteo ocurre en el plan.
pub fn run_pipewire_input_until(
    config: PipeWireStreamConfig,
    capture_node: String,
    ring: Arc<SampleRingBuffer>,
    stop: Arc<AtomicBool>,
    ready: std::sync::mpsc::SyncSender<Result<(), String>>,
) -> Result<(), PipeWireError> {
    let config = config.validate()?;
    pw::init();
    let main_loop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&main_loop, None)?;
    let core = context.connect_rc(None)?;
    let stream = pw::stream::StreamBox::new(
        &core,
        "estudio-daw-track-input",
        properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Music",
            *pw::keys::AUDIO_CHANNELS => config.channels.to_string(),
            *pw::keys::NODE_LATENCY => format!("{}/{}", config.period_frames, config.sample_rate),
            *pw::keys::TARGET_OBJECT => capture_node,
            "node.async" => "true",
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
            let valid_bytes =
                (data.chunk().size() as usize).min(data.data().map_or(0, |bytes| bytes.len()));
            let Some(bytes) = data.data() else {
                return;
            };
            let (_, samples, _) = unsafe { bytes[..valid_bytes].align_to::<f32>() };
            let _ = ring.push(samples);
        })
        .register()?;
    let mut params = audio_params(config);
    if let Err(error) = stream.connect(
        spa::utils::Direction::Input,
        None,
        pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
        &mut params,
    ) {
        let _ = ready.send(Err(error.to_string()));
        return Err(error.into());
    }
    let _ = ready.send(Ok(()));
    let stop_check = Arc::clone(&stop);
    let loop_to_quit = main_loop.clone();
    let timer = main_loop.loop_().add_timer(move |_| {
        if stop_check.load(Ordering::Acquire) {
            loop_to_quit.quit();
        }
    });
    timer
        .update_timer(
            Some(Duration::from_millis(2)),
            Some(Duration::from_millis(2)),
        )
        .into_result()
        .map_err(|error| PipeWireError::Timer(format!("{error:?}")))?;
    main_loop.run();
    drop(_listener);
    drop(stream);
    Ok(())
}

fn process_output_block(processor: &mut RenderPlanProcessor, samples: &mut [f32], paused: bool) {
    if paused || processor.process(samples).is_err() {
        samples.fill(0.0);
    }
}

/// Reproduce un `RenderPlan` por una duración finita, sin abrir una entrada
/// física ni reenviar audio capturado a la salida.
pub fn run_pipewire_output_for_targets(
    config: PipeWireStreamConfig,
    render_plan: RenderPlan,
    duration: Duration,
    playback_node: Option<String>,
) -> Result<PipeWireDuplexReport, PipeWireError> {
    let config = config.validate()?;
    let mut processor = render_plan_exchange(render_plan).1;
    pw::init();
    let main_loop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&main_loop, None)?;
    let core = context.connect_rc(None)?;
    let mut stream_properties = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Playback",
        *pw::keys::MEDIA_ROLE => "Music",
        *pw::keys::AUDIO_CHANNELS => config.channels.to_string(),
        *pw::keys::NODE_LATENCY => format!("{}/{}", config.period_frames, config.sample_rate),
    };
    if let Some(node) = playback_node {
        stream_properties.insert(*pw::keys::TARGET_OBJECT, node);
    }
    let stream = pw::stream::StreamBox::new(&core, "estudio-daw-output", stream_properties)?;
    let output_callbacks = Arc::new(AtomicU64::new(0));
    let output_total_samples = Arc::new(AtomicU64::new(0));
    let output_last_samples = Arc::new(AtomicU64::new(0));
    let output_last_requested_frames = Arc::new(AtomicU64::new(0));
    let callbacks = Arc::clone(&output_callbacks);
    let total_samples = Arc::clone(&output_total_samples);
    let last_samples = Arc::clone(&output_last_samples);
    let last_requested = Arc::clone(&output_last_requested_frames);
    let _listener = stream
        .add_local_listener_with_user_data(())
        .process(move |stream, _| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let requested_frames = buffer.requested();
            last_requested.store(requested_frames, Ordering::Relaxed);
            let Some(data) = buffer.datas_mut().first_mut() else {
                return;
            };
            let sample_count = {
                let Some(bytes) = data.data() else {
                    return;
                };
                let valid_bytes = output_buffer_bytes(
                    requested_frames,
                    config.period_frames,
                    config.channels as usize,
                    bytes.len(),
                );
                let (_, samples, _) = unsafe { bytes[..valid_bytes].align_to_mut::<f32>() };
                samples.fill(0.0);
                let _ = processor.process(samples);
                samples.len()
            };
            let chunk = data.chunk_mut();
            *chunk.offset_mut() = 0;
            *chunk.stride_mut() = (config.channels as usize * std::mem::size_of::<f32>()) as _;
            *chunk.size_mut() = (sample_count * std::mem::size_of::<f32>()) as _;
            callbacks.fetch_add(1, Ordering::Relaxed);
            total_samples.fetch_add(sample_count as u64, Ordering::Relaxed);
            last_samples.store(sample_count as u64, Ordering::Relaxed);
        })
        .register()?;

    let mut params = audio_params(config);
    stream.connect(
        spa::utils::Direction::Output,
        None,
        pw::stream::StreamFlags::AUTOCONNECT
            | pw::stream::StreamFlags::MAP_BUFFERS
            | pw::stream::StreamFlags::RT_PROCESS,
        &mut params,
    )?;
    let loop_to_quit = main_loop.clone();
    let _timer = main_loop.loop_().add_timer(move |_| loop_to_quit.quit());
    _timer
        .update_timer(Some(duration), None)
        .into_result()
        .map_err(|error| PipeWireError::Timer(format!("{error:?}")))?;
    main_loop.run();
    drop(_listener);
    drop(stream);
    Ok(PipeWireDuplexReport {
        capture_callbacks: 0,
        output_callbacks: output_callbacks.load(Ordering::Relaxed),
        capture_total_samples: 0,
        output_total_samples: output_total_samples.load(Ordering::Relaxed),
        capture_last_samples: 0,
        output_last_samples: output_last_samples.load(Ordering::Relaxed),
        capture_dropped_samples: 0,
        output_silence_samples: 0,
        output_last_requested_frames: output_last_requested_frames.load(Ordering::Relaxed),
        capture_start_delay_micros: 0,
    })
}

/// Ejecuta la salida en su propio hilo de PipeWire y la captura del monitor
/// del sink en un cliente/hilo separado para aislar la planificación RT.
pub fn run_pipewire_output_for_targets_with_monitor_capture(
    config: PipeWireStreamConfig,
    render_plan: RenderPlan,
    duration: Duration,
    playback_node: Option<String>,
    capture_node: String,
    recorder: WavCaptureRecorder,
) -> Result<(PipeWireDuplexReport, WavCaptureReport), PipeWireError> {
    if !config.capture_sink_monitor {
        return Err(PipeWireError::InvalidConfig);
    }
    run_pipewire_output_for_targets_with_capture(
        config,
        render_plan,
        duration,
        playback_node,
        capture_node,
        recorder,
        true,
    )
}

/// Ejecuta reproducción en el cliente de salida y captura de una entrada física
/// en un cliente/hilo asíncrono independiente. La captura no gobierna el reloj
/// de salida ni entra en el callback de render RT.
pub fn run_pipewire_output_for_targets_with_source_capture(
    config: PipeWireStreamConfig,
    render_plan: RenderPlan,
    duration: Duration,
    playback_node: Option<String>,
    capture_node: String,
    recorder: WavCaptureRecorder,
) -> Result<(PipeWireDuplexReport, WavCaptureReport), PipeWireError> {
    if config.capture_sink_monitor {
        return Err(PipeWireError::InvalidConfig);
    }
    run_pipewire_output_for_targets_with_capture(
        config,
        render_plan,
        duration,
        playback_node,
        capture_node,
        recorder,
        false,
    )
}

fn run_pipewire_output_for_targets_with_capture(
    config: PipeWireStreamConfig,
    render_plan: RenderPlan,
    duration: Duration,
    playback_node: Option<String>,
    capture_node: String,
    recorder: WavCaptureRecorder,
    sink_monitor: bool,
) -> Result<(PipeWireDuplexReport, WavCaptureReport), PipeWireError> {
    let capture_started_at = Instant::now();
    let capture_stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let worker_stop = Arc::clone(&capture_stop);
    let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
    let capture_worker = thread::Builder::new()
        .name(if sink_monitor {
            "estudio-pipewire-monitor".into()
        } else {
            "estudio-pipewire-source".into()
        })
        .spawn(move || {
            run_pipewire_external_capture(
                config,
                capture_node,
                capture_started_at,
                worker_stop,
                recorder,
                ready_tx,
                sink_monitor,
            )
        })
        .map_err(PipeWireError::Capture)?;

    if ready_rx.recv().is_err() {
        return capture_worker
            .join()
            .map_err(|_| PipeWireError::CaptureWorkerPanic)?;
    }

    let output_result =
        run_pipewire_output_for_targets(config, render_plan, duration, playback_node);
    capture_stop.store(true, Ordering::Release);
    let (capture_metrics, capture_report) = capture_worker
        .join()
        .map_err(|_| PipeWireError::CaptureWorkerPanic)??;
    let mut output_report = output_result?;
    output_report.capture_callbacks = capture_metrics.capture_callbacks;
    output_report.capture_total_samples = capture_metrics.capture_total_samples;
    output_report.capture_last_samples = capture_metrics.capture_last_samples;
    output_report.capture_dropped_samples = capture_metrics.capture_dropped_samples;
    output_report.capture_start_delay_micros = capture_metrics.capture_start_delay_micros;
    Ok((output_report, capture_report))
}

fn run_pipewire_external_capture(
    config: PipeWireStreamConfig,
    capture_node: String,
    stream_started_at: Instant,
    stop: Arc<std::sync::atomic::AtomicBool>,
    recorder: WavCaptureRecorder,
    ready: std::sync::mpsc::SyncSender<()>,
    sink_monitor: bool,
) -> Result<(PipeWireDuplexReport, WavCaptureReport), PipeWireError> {
    let config = config.validate()?;
    pw::init();
    let main_loop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&main_loop, None)?;
    let core = context.connect_rc(None)?;
    let recorder = Arc::new(recorder);
    let mut capture_properties = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Capture",
        *pw::keys::MEDIA_ROLE => "Music",
        *pw::keys::AUDIO_CHANNELS => config.channels.to_string(),
        *pw::keys::NODE_LATENCY => format!("{}/{}", config.period_frames, config.sample_rate),
        *pw::keys::TARGET_OBJECT => capture_node,
        // Keep ADC/monitor processing out of the sink driver's RT dependency
        // chain. The mapped capture callback is dispatched on this client's
        // regular loop and only copies into the preallocated recorder ring.
        "node.async" => "true",
    };
    if sink_monitor {
        capture_properties.insert("stream.capture.sink", "true");
        capture_properties.insert(
            "node.force-quantum",
            config
                .force_graph_quantum
                .unwrap_or(config.period_frames)
                .to_string(),
        );
    }
    let capture_stream = pw::stream::StreamBox::new(
        &core,
        if sink_monitor {
            "estudio-daw-monitor-capture"
        } else {
            "estudio-daw-physical-capture"
        },
        capture_properties,
    )?;
    let capture_callbacks = Arc::new(AtomicU64::new(0));
    let capture_total_samples = Arc::new(AtomicU64::new(0));
    let capture_last_samples = Arc::new(AtomicU64::new(0));
    let capture_dropped_samples = Arc::new(AtomicU64::new(0));
    let capture_start_delay_micros = Arc::new(AtomicU64::new(0));
    let callbacks = Arc::clone(&capture_callbacks);
    let total_samples = Arc::clone(&capture_total_samples);
    let last_samples = Arc::clone(&capture_last_samples);
    let dropped_samples = Arc::clone(&capture_dropped_samples);
    let start_delay = Arc::clone(&capture_start_delay_micros);
    let capture_recorder = Arc::clone(&recorder);
    let _listener = capture_stream
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
            let elapsed_micros = stream_started_at.elapsed().as_micros() as u64;
            let _ = start_delay.compare_exchange(
                0,
                elapsed_micros.saturating_add(1),
                Ordering::Relaxed,
                Ordering::Relaxed,
            );
            callbacks.fetch_add(1, Ordering::Relaxed);
            total_samples.fetch_add(samples.len() as u64, Ordering::Relaxed);
            last_samples.store(samples.len() as u64, Ordering::Relaxed);
            let pushed = capture_recorder.push(samples);
            dropped_samples.fetch_add((samples.len() - pushed) as u64, Ordering::Relaxed);
        })
        .register()?;

    let mut params = audio_params(config);
    capture_stream.connect(
        spa::utils::Direction::Input,
        None,
        pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
        &mut params,
    )?;
    let _ = ready.send(());
    let stop_check = Arc::clone(&stop);
    let loop_to_quit = main_loop.clone();
    let timer = main_loop.loop_().add_timer(move |_| {
        if stop_check.load(Ordering::Acquire) {
            loop_to_quit.quit();
        }
    });
    timer
        .update_timer(
            Some(Duration::from_millis(2)),
            Some(Duration::from_millis(2)),
        )
        .into_result()
        .map_err(|error| PipeWireError::Timer(format!("{error:?}")))?;
    main_loop.run();
    drop(_listener);
    drop(capture_stream);
    let capture_report = Arc::try_unwrap(recorder)
        .map_err(|_| PipeWireError::CaptureStillInUse)?
        .finish()?;
    Ok((
        PipeWireDuplexReport {
            capture_callbacks: capture_callbacks.load(Ordering::Relaxed),
            output_callbacks: 0,
            capture_total_samples: capture_total_samples.load(Ordering::Relaxed),
            output_total_samples: 0,
            capture_last_samples: capture_last_samples.load(Ordering::Relaxed),
            output_last_samples: 0,
            output_silence_samples: 0,
            output_last_requested_frames: 0,
            capture_dropped_samples: capture_dropped_samples.load(Ordering::Relaxed),
            capture_start_delay_micros: capture_start_delay_micros
                .load(Ordering::Relaxed)
                .saturating_sub(1),
        },
        capture_report,
    ))
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
    let (_control, processor) = render_plan_exchange(render_plan);
    run_pipewire_duplex_controlled(config, processor)
}

/// Variante con hot-swap habilitado. El caller conserva el handle de control
/// en un hilo no-RT y recoge los planes retirados después de cada publicación.
pub fn run_pipewire_duplex_controlled(
    config: PipeWireStreamConfig,
    processor: RenderPlanProcessor,
) -> Result<(), PipeWireError> {
    run_pipewire_duplex_internal(config, processor, None, None, PipeWireTargets::default())
        .map(|_| ())
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
    let (_control, processor) = render_plan_exchange(render_plan);
    run_pipewire_duplex_for_targets_controlled(config, processor, duration, targets)
}

/// Prueba finita del duplex con reemplazo de plan habilitado para el callback.
pub fn run_pipewire_duplex_for_targets_controlled(
    config: PipeWireStreamConfig,
    processor: RenderPlanProcessor,
    duration: Duration,
    targets: PipeWireTargets,
) -> Result<PipeWireDuplexReport, PipeWireError> {
    run_pipewire_duplex_internal(config, processor, Some(duration), None, targets)
        .map(|(report, _)| report)
}

/// Ejecuta duplex y guarda la captura en un WAV sin hacer I/O en el callback.
///
/// El `WavCaptureRecorder` se consume aquí para garantizar que el hilo escritor
/// termine y que el encabezado del archivo quede actualizado antes de retornar.
pub fn run_pipewire_duplex_for_targets_with_capture(
    config: PipeWireStreamConfig,
    render_plan: RenderPlan,
    duration: Duration,
    targets: PipeWireTargets,
    recorder: WavCaptureRecorder,
) -> Result<(PipeWireDuplexReport, WavCaptureReport), PipeWireError> {
    let (_control, processor) = render_plan_exchange(render_plan);
    let (report, capture_report) =
        run_pipewire_duplex_internal(config, processor, Some(duration), Some(recorder), targets)?;
    capture_report
        .ok_or(PipeWireError::CaptureStillInUse)
        .map(|capture| (report, capture))
}

fn run_pipewire_duplex_internal(
    config: PipeWireStreamConfig,
    mut processor: RenderPlanProcessor,
    duration: Option<Duration>,
    recorder: Option<WavCaptureRecorder>,
    targets: PipeWireTargets,
) -> Result<(PipeWireDuplexReport, Option<WavCaptureReport>), PipeWireError> {
    let config = config.validate()?;
    let stream_started_at = Instant::now();
    let recorder = recorder.map(Arc::new);
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
    let capture_start_delay_micros = Arc::new(AtomicU64::new(0));
    let output_callbacks = Arc::new(AtomicU64::new(0));
    let output_total_samples = Arc::new(AtomicU64::new(0));
    let output_last_samples = Arc::new(AtomicU64::new(0));
    let output_last_requested_frames = Arc::new(AtomicU64::new(0));
    let output_silence_samples = Arc::new(AtomicU64::new(0));

    let mut capture_properties = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Music",
            *pw::keys::AUDIO_CHANNELS => config.channels.to_string(),
            *pw::keys::NODE_LATENCY => format!("{}/{}", config.period_frames, config.sample_rate),
    };
    if let Some(node) = targets.capture_node {
        capture_properties.insert(*pw::keys::TARGET_OBJECT, node);
    }
    if config.capture_sink_monitor {
        // WirePlumber otherwise treats an Audio/Sink target as incompatible
        // with a capture stream and falls back to the default physical source.
        capture_properties.insert("stream.capture.sink", "true");
        // The monitor link points back from the playback sink. Async graph
        // scheduling prevents that dependency from perturbing the sink driver.
        capture_properties.insert("node.async", "true");
    }
    if let Some(quantum) = config.force_graph_quantum {
        capture_properties.insert("node.force-quantum", quantum.to_string());
    }
    let capture_stream =
        pw::stream::StreamBox::new(&core, "estudio-daw-input", capture_properties)?;
    let capture_ring = Arc::clone(&ring);
    let capture_callbacks_counter = Arc::clone(&capture_callbacks);
    let capture_total_counter = Arc::clone(&capture_total_samples);
    let capture_last_counter = Arc::clone(&capture_last_samples);
    let capture_dropped_counter = Arc::clone(&capture_dropped_samples);
    let capture_start_delay_counter = Arc::clone(&capture_start_delay_micros);
    let capture_recorder = recorder.as_ref().map(Arc::clone);
    let capture_sink_monitor = config.capture_sink_monitor;
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
            let elapsed_micros = stream_started_at.elapsed().as_micros() as u64;
            let _ = capture_start_delay_counter.compare_exchange(
                0,
                elapsed_micros.saturating_add(1),
                Ordering::Relaxed,
                Ordering::Relaxed,
            );
            capture_callbacks_counter.fetch_add(1, Ordering::Relaxed);
            capture_total_counter.fetch_add(samples.len() as u64, Ordering::Relaxed);
            capture_last_counter.store(samples.len() as u64, Ordering::Relaxed);
            let pushed = if capture_sink_monitor {
                // A sink monitor already contains the playback stream. Feeding
                // it back through the duplex ring would create a feedback loop.
                samples.len()
            } else {
                capture_ring.push(samples)
            };
            capture_dropped_counter.fetch_add((samples.len() - pushed) as u64, Ordering::Relaxed);
            if let Some(recorder) = capture_recorder.as_ref() {
                recorder.push(samples);
            }
        })
        .register()?;

    let mut output_properties = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Playback",
        *pw::keys::MEDIA_ROLE => "Music",
        *pw::keys::AUDIO_CHANNELS => config.channels.to_string(),
        *pw::keys::NODE_LATENCY => format!("{}/{}", config.period_frames, config.sample_rate),
    };
    if let Some(node) = targets.playback_node {
        output_properties.insert(*pw::keys::TARGET_OBJECT, node);
    }
    let output_stream = pw::stream::StreamBox::new(&core, "estudio-daw-output", output_properties)?;
    let output_ring = Arc::clone(&ring);
    let output_callbacks_counter = Arc::clone(&output_callbacks);
    let output_total_counter = Arc::clone(&output_total_samples);
    let output_last_counter = Arc::clone(&output_last_samples);
    let output_requested_counter = Arc::clone(&output_last_requested_frames);
    let output_silence_counter = Arc::clone(&output_silence_samples);
    let capture_sink_monitor = config.capture_sink_monitor;
    let _output_listener = output_stream
        .add_local_listener_with_user_data(())
        .process(move |stream, _| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let requested_frames = buffer.requested();
            output_requested_counter.store(requested_frames, Ordering::Relaxed);
            let Some(data) = buffer.datas_mut().first_mut() else {
                return;
            };
            let Some(bytes) = data.data() else {
                return;
            };
            // PipeWire puede solicitar menos frames que el quantum del grafo
            // después del remuestreo; producir el quantum completo aceleraría
            // el render y vaciaría el ring PCM.
            let valid_bytes = output_buffer_bytes(
                requested_frames,
                config.period_frames,
                config.channels as usize,
                bytes.len(),
            );
            let (_, samples, _) = unsafe { bytes[..valid_bytes].align_to_mut::<f32>() };
            output_callbacks_counter.fetch_add(1, Ordering::Relaxed);
            output_total_counter.fetch_add(samples.len() as u64, Ordering::Relaxed);
            output_last_counter.store(samples.len() as u64, Ordering::Relaxed);
            let copied = if capture_sink_monitor {
                0
            } else {
                output_ring.pop(samples)
            };
            samples[copied..].fill(0.0);
            output_silence_counter.fetch_add((samples.len() - copied) as u64, Ordering::Relaxed);
            let _ = processor.process(samples);
        })
        .register()?;

    let mut capture_params = audio_params(config);
    let mut capture_flags =
        pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS;
    // Without RT_PROCESS PipeWire dispatches the monitor callback asynchronously,
    // so it cannot control playback's realtime schedule.
    if !config.capture_sink_monitor {
        capture_flags |= pw::stream::StreamFlags::RT_PROCESS;
    }
    capture_stream.connect(
        spa::utils::Direction::Input,
        None,
        capture_flags,
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
    // Liberamos explícitamente los listeners antes de recuperar el recorder
    // único y cerrar su hilo escritor de forma determinista.
    drop(_capture_listener);
    drop(capture_stream);
    drop(_output_listener);
    drop(output_stream);
    let capture_report = recorder
        .map(|recorder| {
            Arc::try_unwrap(recorder)
                .map_err(|_| PipeWireError::CaptureStillInUse)
                .and_then(WavCaptureRecorder::finish)
        })
        .transpose()?;
    Ok((
        PipeWireDuplexReport {
            capture_callbacks: capture_callbacks.load(Ordering::Relaxed),
            output_callbacks: output_callbacks.load(Ordering::Relaxed),
            capture_total_samples: capture_total_samples.load(Ordering::Relaxed),
            output_total_samples: output_total_samples.load(Ordering::Relaxed),
            capture_last_samples: capture_last_samples.load(Ordering::Relaxed),
            output_last_samples: output_last_samples.load(Ordering::Relaxed),
            output_last_requested_frames: output_last_requested_frames.load(Ordering::Relaxed),
            capture_dropped_samples: capture_dropped_samples.load(Ordering::Relaxed),
            output_silence_samples: output_silence_samples.load(Ordering::Relaxed),
            capture_start_delay_micros: capture_start_delay_micros
                .load(Ordering::Relaxed)
                .saturating_sub(1),
        },
        capture_report,
    ))
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

fn output_buffer_bytes(
    requested_frames: u64,
    fallback_frames: usize,
    channels: usize,
    buffer_bytes: usize,
) -> usize {
    let frames = if requested_frames > 0 {
        usize::try_from(requested_frames).unwrap_or(usize::MAX)
    } else {
        fallback_frames
    };
    frames
        .saturating_mul(channels)
        .saturating_mul(std::mem::size_of::<f32>())
        .min(buffer_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use estudio_daw_audio_engine::{AudioNode, AudioNodeError, RenderPlanBuilder};
    use std::sync::atomic::AtomicUsize;

    struct ProcessCounter(Arc<AtomicUsize>);

    impl AudioNode for ProcessCounter {
        fn process(&mut self, samples: &mut [f32]) -> Result<(), AudioNodeError> {
            self.0.fetch_add(1, Ordering::Relaxed);
            samples.fill(1.0);
            Ok(())
        }
    }

    #[test]
    fn pause_outputs_silence_without_advancing_render_plan() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut builder = RenderPlanBuilder::new();
        builder.add_node(ProcessCounter(Arc::clone(&calls)));
        let (_control, mut processor) = render_plan_exchange(builder.build());

        let mut block = [0.25; 8];
        process_output_block(&mut processor, &mut block, true);
        assert_eq!(block, [0.0; 8]);
        assert_eq!(calls.load(Ordering::Relaxed), 0);

        process_output_block(&mut processor, &mut block, false);
        assert_eq!(block, [1.0; 8]);
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn validates_pipewire_stream_configuration() {
        assert!(PipeWireStreamConfig::default().validate().is_ok());
        assert_eq!(
            PipeWireStreamConfig {
                sample_rate: 0,
                channels: 2,
                period_frames: 32,
                max_buffer_frames: 2_048,
                capture_sink_monitor: false,
                force_graph_quantum: None,
            }
            .validate()
            .unwrap_err()
            .to_string(),
            "configuración PipeWire inválida"
        );
    }

    #[test]
    fn output_stream_respects_pipewire_requested_frames() {
        assert_eq!(output_buffer_bytes(8, 128, 2, 4_096), 64);
        assert_eq!(output_buffer_bytes(0, 128, 2, 4_096), 1_024);
        assert_eq!(output_buffer_bytes(256, 128, 2, 512), 512);
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
