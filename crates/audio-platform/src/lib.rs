//! Backends de audio del sistema operativo.
//!
//! Este crate contiene PipeWire y no debe filtrarse hacia el modelo de sesión.
//! El único objeto que atraviesa la frontera es el `RenderPlan` ya compilado.

use estudio_daw_audio_engine::RenderPlan;
use pipewire as pw;
use pw::{properties::properties, spa};
use spa::pod::Pod;
use std::io::Cursor;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipeWireStreamConfig {
    pub sample_rate: u32,
    pub channels: u32,
}

impl Default for PipeWireStreamConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            channels: 2,
        }
    }
}

#[derive(Debug, Error)]
pub enum PipeWireError {
    #[error("configuración PipeWire inválida")]
    InvalidConfig,
    #[error("error de PipeWire: {0}")]
    PipeWire(#[from] pw::Error),
}

impl PipeWireStreamConfig {
    pub fn validate(self) -> Result<Self, PipeWireError> {
        if self.sample_rate == 0 || self.channels == 0 {
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
            }
            .validate()
            .unwrap_err()
            .to_string(),
            "configuración PipeWire inválida"
        );
    }
}
