//! Sonda de adaptadores disponibles para el futuro backend de compute.

fn main() {
    let adapters = estudio_daw_compute_runtime::probe_gpu_adapters();
    if adapters.is_empty() {
        println!("No se detectaron adaptadores GPU mediante wgpu.");
        println!("El fallback CPU permanece disponible.");
        return;
    }
    for (index, adapter) in adapters.iter().enumerate() {
        println!("GPU {index}: {}", adapter.name);
        println!("  backend: {}", adapter.backend);
        println!("  tipo: {}", adapter.device_type);
        println!(
            "  vendor/device: {:04x}:{:04x}",
            adapter.vendor, adapter.device
        );
        println!("  driver: {} ({})", adapter.driver, adapter.driver_info);
        println!(
            "  max storage buffer: {} bytes",
            adapter.max_storage_buffer_binding_size
        );
    }
}
