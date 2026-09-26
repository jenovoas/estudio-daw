//! Bloques DSP propios de Estudio DAW.
//!
//! El ecualizador de este crate separa configuración y procesamiento: añadir
//! bandas, cambiar parámetros y recalcular coeficientes ocurre fuera del
//! callback. `process_interleaved()` sólo recorre buffers y estados ya
//! preasignados.

use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EqFilterType {
    Bell,
    LowShelf,
    HighShelf,
    LowPass,
    HighPass,
    Notch,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EqBandConfig {
    pub filter_type: EqFilterType,
    pub frequency_hz: f32,
    pub gain_db: f32,
    pub q: f32,
    pub enabled: bool,
}

impl EqBandConfig {
    pub fn bell(frequency_hz: f32, gain_db: f32, q: f32) -> Self {
        Self {
            filter_type: EqFilterType::Bell,
            frequency_hz,
            gain_db,
            q,
            enabled: true,
        }
    }

    pub fn low_pass(frequency_hz: f32, q: f32) -> Self {
        Self {
            filter_type: EqFilterType::LowPass,
            frequency_hz,
            gain_db: 0.0,
            q,
            enabled: true,
        }
    }

    pub fn high_pass(frequency_hz: f32, q: f32) -> Self {
        Self {
            filter_type: EqFilterType::HighPass,
            frequency_hz,
            gain_db: 0.0,
            q,
            enabled: true,
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DspError {
    InvalidSampleRate,
    InvalidChannelCount,
    InvalidBandIndex,
    InvalidBlockLength,
    InvalidBandParameter,
}

#[derive(Debug, Clone, Copy)]
struct BiquadCoefficients {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

#[derive(Debug, Clone, Copy, Default)]
struct BiquadState {
    z1: f32,
    z2: f32,
}

impl BiquadState {
    #[inline]
    fn process(&mut self, input: f32, coefficients: BiquadCoefficients) -> f32 {
        // Forma transpuesta II: dos estados por canal y ninguna asignación.
        let output = coefficients.b0 * input + self.z1;
        self.z1 = coefficients.b1 * input - coefficients.a1 * output + self.z2;
        self.z2 = coefficients.b2 * input - coefficients.a2 * output;
        output
    }

    fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Debug)]
struct EqBand {
    config: EqBandConfig,
    coefficients: BiquadCoefficients,
    states: Vec<BiquadState>,
}

#[derive(Debug)]
pub struct Equalizer {
    sample_rate_hz: f32,
    channels: usize,
    bands: Vec<EqBand>,
}

impl Equalizer {
    /// Crea un ecualizador. Las bandas se añaden antes de entrar al callback.
    pub fn new(sample_rate_hz: f32, channels: usize) -> Result<Self, DspError> {
        if !sample_rate_hz.is_finite() || sample_rate_hz <= 0.0 {
            return Err(DspError::InvalidSampleRate);
        }
        if channels == 0 {
            return Err(DspError::InvalidChannelCount);
        }
        Ok(Self {
            sample_rate_hz,
            channels,
            bands: Vec::new(),
        })
    }

    /// Añade una banda y devuelve su índice estable dentro de la cadena.
    pub fn add_band(&mut self, config: EqBandConfig) -> Result<usize, DspError> {
        let coefficients = calculate_coefficients(self.sample_rate_hz, config)?;
        let index = self.bands.len();
        self.bands.push(EqBand {
            config,
            coefficients,
            states: vec![BiquadState::default(); self.channels],
        });
        Ok(index)
    }

    /// Actualiza configuración y coeficientes fuera del callback RT.
    pub fn set_band(&mut self, index: usize, config: EqBandConfig) -> Result<(), DspError> {
        let coefficients = calculate_coefficients(self.sample_rate_hz, config)?;
        let band = self
            .bands
            .get_mut(index)
            .ok_or(DspError::InvalidBandIndex)?;
        band.config = config;
        band.coefficients = coefficients;
        Ok(())
    }

    pub fn band(&self, index: usize) -> Option<EqBandConfig> {
        self.bands.get(index).map(|band| band.config)
    }

    pub fn band_count(&self) -> usize {
        self.bands.len()
    }

    /// Procesa audio intercalado `[ch0, ch1, ch0, ch1, ...]`.
    ///
    /// Esta función no reserva, no cambia la cantidad de bandas y no recalcula
    /// coeficientes. Puede ejecutarse dentro del callback si el buffer fue
    /// validado y el ecualizador se preparó antes de iniciar el stream.
    pub fn process_interleaved(&mut self, buffer: &mut [f32]) -> Result<(), DspError> {
        if buffer.len() % self.channels != 0 {
            return Err(DspError::InvalidBlockLength);
        }
        for frame in buffer.chunks_exact_mut(self.channels) {
            for band in &mut self.bands {
                if !band.config.enabled {
                    continue;
                }
                for (channel, sample) in frame.iter_mut().enumerate() {
                    *sample = band.states[channel].process(*sample, band.coefficients);
                }
            }
        }
        Ok(())
    }

    pub fn reset(&mut self) {
        for band in &mut self.bands {
            for state in &mut band.states {
                state.reset();
            }
        }
    }
}

fn calculate_coefficients(
    sample_rate_hz: f32,
    config: EqBandConfig,
) -> Result<BiquadCoefficients, DspError> {
    if !config.frequency_hz.is_finite()
        || config.frequency_hz <= 0.0
        || config.frequency_hz >= sample_rate_hz * 0.5
        || !config.q.is_finite()
        || config.q <= 0.0
        || !config.gain_db.is_finite()
    {
        return Err(DspError::InvalidBandParameter);
    }
    let omega = 2.0 * PI * config.frequency_hz / sample_rate_hz;
    let cosine = omega.cos();
    let sine = omega.sin();
    let alpha = sine / (2.0 * config.q);
    let amplitude = 10.0_f32.powf(config.gain_db / 40.0);
    let sqrt_amplitude = amplitude.sqrt();

    let (b0, b1, b2, a0, a1, a2) = match config.filter_type {
        EqFilterType::Bell => (
            1.0 + alpha * amplitude,
            -2.0 * cosine,
            1.0 - alpha * amplitude,
            1.0 + alpha / amplitude,
            -2.0 * cosine,
            1.0 - alpha / amplitude,
        ),
        EqFilterType::LowPass => (
            (1.0 - cosine) / 2.0,
            1.0 - cosine,
            (1.0 - cosine) / 2.0,
            1.0 + alpha,
            -2.0 * cosine,
            1.0 - alpha,
        ),
        EqFilterType::HighPass => (
            (1.0 + cosine) / 2.0,
            -(1.0 + cosine),
            (1.0 + cosine) / 2.0,
            1.0 + alpha,
            -2.0 * cosine,
            1.0 - alpha,
        ),
        EqFilterType::Notch => (
            1.0,
            -2.0 * cosine,
            1.0,
            1.0 + alpha,
            -2.0 * cosine,
            1.0 - alpha,
        ),
        EqFilterType::LowShelf => (
            amplitude
                * ((amplitude + 1.0) - (amplitude - 1.0) * cosine + 2.0 * sqrt_amplitude * alpha),
            2.0 * amplitude * ((amplitude - 1.0) - (amplitude + 1.0) * cosine),
            amplitude
                * ((amplitude + 1.0) - (amplitude - 1.0) * cosine - 2.0 * sqrt_amplitude * alpha),
            (amplitude + 1.0) + (amplitude - 1.0) * cosine + 2.0 * sqrt_amplitude * alpha,
            -2.0 * ((amplitude - 1.0) + (amplitude + 1.0) * cosine),
            (amplitude + 1.0) + (amplitude - 1.0) * cosine - 2.0 * sqrt_amplitude * alpha,
        ),
        EqFilterType::HighShelf => (
            amplitude
                * ((amplitude + 1.0) + (amplitude - 1.0) * cosine + 2.0 * sqrt_amplitude * alpha),
            -2.0 * amplitude * ((amplitude - 1.0) + (amplitude + 1.0) * cosine),
            amplitude
                * ((amplitude + 1.0) + (amplitude - 1.0) * cosine - 2.0 * sqrt_amplitude * alpha),
            (amplitude + 1.0) - (amplitude - 1.0) * cosine + 2.0 * sqrt_amplitude * alpha,
            2.0 * ((amplitude - 1.0) - (amplitude + 1.0) * cosine),
            (amplitude + 1.0) - (amplitude - 1.0) * cosine - 2.0 * sqrt_amplitude * alpha,
        ),
    };
    if !a0.is_finite() || a0.abs() < f32::EPSILON {
        return Err(DspError::InvalidBandParameter);
    }
    Ok(BiquadCoefficients {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processes_stereo_without_allocating_during_the_call() {
        let mut equalizer = Equalizer::new(48_000.0, 2).unwrap();
        equalizer
            .add_band(EqBandConfig::bell(1_000.0, 6.0, 0.707))
            .unwrap();
        let mut buffer = vec![0.25; 128];
        equalizer.process_interleaved(&mut buffer).unwrap();
        assert!(buffer.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn disabled_band_is_transparent() {
        let mut equalizer = Equalizer::new(48_000.0, 1).unwrap();
        equalizer
            .add_band(EqBandConfig::bell(1_000.0, 24.0, 0.5).with_enabled(false))
            .unwrap();
        let mut buffer = vec![0.5; 32];
        equalizer.process_interleaved(&mut buffer).unwrap();
        assert!(buffer
            .iter()
            .all(|sample| (*sample - 0.5).abs() < f32::EPSILON));
    }

    #[test]
    fn rejects_invalid_band_and_block_parameters() {
        assert_eq!(Equalizer::new(48_000.0, 1).unwrap().band_count(), 0);
        let mut equalizer = Equalizer::new(48_000.0, 2).unwrap();
        assert_eq!(
            equalizer.add_band(EqBandConfig::bell(30_000.0, 0.0, 1.0)),
            Err(DspError::InvalidBandParameter)
        );
        assert_eq!(
            equalizer.process_interleaved(&mut [0.0; 3]),
            Err(DspError::InvalidBlockLength)
        );
    }

    #[test]
    fn reset_clears_filter_state() {
        let mut equalizer = Equalizer::new(48_000.0, 1).unwrap();
        equalizer
            .add_band(EqBandConfig::high_pass(100.0, 0.707))
            .unwrap();
        let mut first = vec![1.0; 32];
        equalizer.process_interleaved(&mut first).unwrap();
        equalizer.reset();
        let mut after_reset = vec![1.0; 32];
        equalizer.process_interleaved(&mut after_reset).unwrap();
        assert_eq!(first, after_reset);
    }
}
