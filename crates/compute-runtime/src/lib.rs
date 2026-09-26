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
}
