//! Política de selección CPU/GPU.
//!
//! Este crate no inicializa una GPU todavía. Define el contrato que usarán
//! `wgpu`, Vulkan y los workers de análisis para decidir si una operación puede
//! acelerarse sin poner en riesgo la reproducción.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadKind {
    RealtimeAudio,
    Fft,
    Convolution,
    TimeStretch,
    Analysis,
    OfflineRender,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputeBackend {
    Cpu,
    Gpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackReason {
    RealtimeSafety,
    GpuUnavailable,
    GpuUnhealthy,
    GpuMemoryInsufficient,
    TransferTooExpensive,
    DeadlineRisk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComputeRequest {
    pub workload: WorkloadKind,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub estimated_cpu_micros: u64,
    pub estimated_gpu_micros: u64,
    pub estimated_transfer_micros: u64,
    pub deadline_micros: Option<u64>,
    pub gpu_available: bool,
    pub gpu_healthy: bool,
    pub gpu_free_bytes: u64,
    pub gpu_required_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComputePolicy {
    /// Porcentaje mínimo de mejora requerido para justificar GPU.
    pub minimum_gpu_speedup_percent: u64,
    /// La GPU no se usa en tiempo real hasta validar buffers persistentes y
    /// deadlines en una implementación concreta.
    pub allow_realtime_gpu: bool,
}

impl Default for ComputePolicy {
    fn default() -> Self {
        Self {
            minimum_gpu_speedup_percent: 20,
            allow_realtime_gpu: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComputeDecision {
    pub backend: ComputeBackend,
    pub fallback_reason: Option<FallbackReason>,
    pub estimated_total_micros: u64,
}

impl ComputeDecision {
    fn cpu(reason: FallbackReason, estimated_total_micros: u64) -> Self {
        Self {
            backend: ComputeBackend::Cpu,
            fallback_reason: Some(reason),
            estimated_total_micros,
        }
    }
}

/// Decide la ruta de ejecución sin tocar todavía ningún recurso de hardware.
///
/// La decisión es conservadora: cualquier duda que pueda afectar al callback
/// de audio devuelve CPU. Las razones quedan registradas para diagnóstico y
/// para comparar posteriormente la predicción con un benchmark real.
pub fn choose_backend(request: ComputeRequest, policy: ComputePolicy) -> ComputeDecision {
    if request.workload == WorkloadKind::RealtimeAudio && !policy.allow_realtime_gpu {
        return ComputeDecision::cpu(FallbackReason::RealtimeSafety, request.estimated_cpu_micros);
    }
    if !request.gpu_available {
        return ComputeDecision::cpu(FallbackReason::GpuUnavailable, request.estimated_cpu_micros);
    }
    if !request.gpu_healthy {
        return ComputeDecision::cpu(FallbackReason::GpuUnhealthy, request.estimated_cpu_micros);
    }
    if request.gpu_required_bytes > request.gpu_free_bytes {
        return ComputeDecision::cpu(
            FallbackReason::GpuMemoryInsufficient,
            request.estimated_cpu_micros,
        );
    }

    let gpu_total = request
        .estimated_gpu_micros
        .saturating_add(request.estimated_transfer_micros);
    if request
        .deadline_micros
        .is_some_and(|deadline| gpu_total > deadline)
    {
        return ComputeDecision::cpu(FallbackReason::DeadlineRisk, gpu_total);
    }

    // Evita floats y overflow: GPU sólo gana si su coste total es menor que
    // CPU después de exigir la mejora mínima configurada.
    let required_cpu_budget = u128::from(request.estimated_cpu_micros)
        * u128::from(100_u64.saturating_sub(policy.minimum_gpu_speedup_percent));
    if u128::from(gpu_total) * 100 >= required_cpu_budget {
        return ComputeDecision::cpu(FallbackReason::TransferTooExpensive, gpu_total);
    }

    ComputeDecision {
        backend: ComputeBackend::Gpu,
        fallback_reason: None,
        estimated_total_micros: gpu_total,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FftError {
    EmptyInput,
    LengthMismatch,
    LengthNotPowerOfTwo,
}

/// FFT radix-2 in-place para buffers de tamaño potencia de dos.
///
/// `inverse=true` normaliza el resultado por N. La implementación usa sólo
/// memoria que recibe el llamador, una propiedad importante para poder crear
/// después una variante sin asignaciones en el runtime de audio.
pub fn fft_in_place(
    real: &mut [f32],
    imaginary: &mut [f32],
    inverse: bool,
) -> Result<(), FftError> {
    if real.is_empty() {
        return Err(FftError::EmptyInput);
    }
    if real.len() != imaginary.len() {
        return Err(FftError::LengthMismatch);
    }
    if !real.len().is_power_of_two() {
        return Err(FftError::LengthNotPowerOfTwo);
    }

    // Reordenamiento bit-reversal: deja las muestras en el orden necesario
    // para que cada pasada combine mariposas contiguas.
    let mut reversed = 0;
    for index in 1..real.len() {
        let mut bit = real.len() >> 1;
        while reversed & bit != 0 {
            reversed ^= bit;
            bit >>= 1;
        }
        reversed ^= bit;
        if index < reversed {
            real.swap(index, reversed);
            imaginary.swap(index, reversed);
        }
    }

    let sign = if inverse { 1.0 } else { -1.0 };
    let mut width = 2;
    while width <= real.len() {
        let angle = sign * std::f32::consts::TAU / width as f32;
        let sine = angle.sin();
        let cosine = angle.cos();
        for start in (0..real.len()).step_by(width) {
            let mut twiddle_real = 1.0;
            let mut twiddle_imaginary = 0.0;
            for offset in 0..width / 2 {
                let left = start + offset;
                let right = left + width / 2;
                let product_real =
                    twiddle_real * real[right] - twiddle_imaginary * imaginary[right];
                let product_imaginary =
                    twiddle_real * imaginary[right] + twiddle_imaginary * real[right];
                real[right] = real[left] - product_real;
                imaginary[right] = imaginary[left] - product_imaginary;
                real[left] += product_real;
                imaginary[left] += product_imaginary;
                let next_real = twiddle_real * cosine - twiddle_imaginary * sine;
                twiddle_imaginary = twiddle_real * sine + twiddle_imaginary * cosine;
                twiddle_real = next_real;
            }
        }
        width *= 2;
    }

    if inverse {
        let scale = 1.0 / real.len() as f32;
        for (real, imaginary) in real.iter_mut().zip(imaginary.iter_mut()) {
            *real *= scale;
            *imaginary *= scale;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuAdapterReport {
    pub name: String,
    pub backend: String,
    pub device_type: String,
    pub vendor: u32,
    pub device: u32,
    pub driver: String,
    pub driver_info: String,
    pub max_storage_buffer_binding_size: u32,
}

/// Consulta adaptadores sin crear un device ni reservar buffers de trabajo.
/// Es seguro ejecutarlo al iniciar la aplicación, nunca desde el callback RT.
pub fn probe_gpu_adapters() -> Vec<GpuAdapterReport> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()))
        .into_iter()
        .map(|adapter| {
            let info = adapter.get_info();
            let limits = adapter.limits();
            GpuAdapterReport {
                name: info.name,
                backend: format!("{:?}", info.backend),
                device_type: format!("{:?}", info.device_type),
                vendor: info.vendor,
                device: info.device,
                driver: info.driver,
                driver_info: info.driver_info,
                max_storage_buffer_binding_size: limits.max_storage_buffer_binding_size as u32,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> ComputeRequest {
        ComputeRequest {
            workload: WorkloadKind::Fft,
            input_bytes: 1_048_576,
            output_bytes: 1_048_576,
            estimated_cpu_micros: 10_000,
            estimated_gpu_micros: 2_000,
            estimated_transfer_micros: 500,
            deadline_micros: None,
            gpu_available: true,
            gpu_healthy: true,
            gpu_free_bytes: 512 * 1024 * 1024,
            gpu_required_bytes: 8 * 1024 * 1024,
        }
    }

    #[test]
    fn uses_gpu_when_speedup_survives_transfer_cost() {
        let decision = choose_backend(request(), ComputePolicy::default());
        assert_eq!(decision.backend, ComputeBackend::Gpu);
        assert_eq!(decision.fallback_reason, None);
        assert_eq!(decision.estimated_total_micros, 2_500);
    }

    #[test]
    fn realtime_audio_defaults_to_cpu() {
        let mut request = request();
        request.workload = WorkloadKind::RealtimeAudio;
        let decision = choose_backend(request, ComputePolicy::default());
        assert_eq!(decision.backend, ComputeBackend::Cpu);
        assert_eq!(
            decision.fallback_reason,
            Some(FallbackReason::RealtimeSafety)
        );
    }

    #[test]
    fn unavailable_gpu_falls_back() {
        let mut request = request();
        request.gpu_available = false;
        assert_eq!(
            choose_backend(request, ComputePolicy::default()).fallback_reason,
            Some(FallbackReason::GpuUnavailable)
        );
    }

    #[test]
    fn unhealthy_gpu_falls_back() {
        let mut request = request();
        request.gpu_healthy = false;
        assert_eq!(
            choose_backend(request, ComputePolicy::default()).fallback_reason,
            Some(FallbackReason::GpuUnhealthy)
        );
    }

    #[test]
    fn insufficient_gpu_memory_falls_back() {
        let mut request = request();
        request.gpu_free_bytes = 1;
        assert_eq!(
            choose_backend(request, ComputePolicy::default()).fallback_reason,
            Some(FallbackReason::GpuMemoryInsufficient)
        );
    }

    #[test]
    fn transfer_cost_can_make_cpu_better() {
        let mut request = request();
        request.estimated_transfer_micros = 10_000;
        assert_eq!(
            choose_backend(request, ComputePolicy::default()).fallback_reason,
            Some(FallbackReason::TransferTooExpensive)
        );
    }

    #[test]
    fn deadline_risk_falls_back() {
        let mut request = request();
        request.deadline_micros = Some(100);
        assert_eq!(
            choose_backend(request, ComputePolicy::default()).fallback_reason,
            Some(FallbackReason::DeadlineRisk)
        );
    }

    #[test]
    fn policy_can_explicitly_allow_realtime_gpu() {
        let mut request = request();
        request.workload = WorkloadKind::RealtimeAudio;
        let policy = ComputePolicy {
            allow_realtime_gpu: true,
            ..ComputePolicy::default()
        };
        assert_eq!(choose_backend(request, policy).backend, ComputeBackend::Gpu);
    }

    #[test]
    fn fft_round_trip_recovers_signal() {
        let original = [1.0, 0.0, -1.0, 0.5, 2.0, -0.25, 0.0, 0.75];
        let mut real = original;
        let mut imaginary = [0.0; 8];
        fft_in_place(&mut real, &mut imaginary, false).unwrap();
        fft_in_place(&mut real, &mut imaginary, true).unwrap();
        for (actual, expected) in real.iter().zip(original) {
            assert!((actual - expected).abs() < 0.0001);
        }
        assert!(imaginary.iter().all(|value| value.abs() < 0.0001));
    }

    #[test]
    fn fft_rejects_non_power_of_two() {
        let mut real = [0.0; 3];
        let mut imaginary = [0.0; 3];
        assert_eq!(
            fft_in_place(&mut real, &mut imaginary, false),
            Err(FftError::LengthNotPowerOfTwo)
        );
    }
}
