//! Benchmarks CPU de referencia para calibrar la política CPU/GPU.
//!
//! No intenta ser un benchmark científico todavía. Su propósito es producir
//! números reproducibles del hardware actual antes de implementar kernels
//! `wgpu`, y detectar regresiones con el mismo tamaño de trabajo.

use estudio_daw_compute_runtime::fft_in_place;
use std::{hint::black_box, time::Instant};

fn main() {
    let fft_micros = benchmark_fft(1024, 100);
    let convolution_micros = benchmark_convolution(65_536, 64, 3);
    let profile = if cfg!(debug_assertions) {
        "debug (no apto para comparar rendimiento)"
    } else {
        "release"
    };
    println!("Estudio DAW compute baseline (CPU)");
    println!("build_profile={profile}");
    println!("fft_1024_us={fft_micros}");
    println!("convolution_65536x64_us={convolution_micros}");
    println!("Estos valores calibrarán el scheduler cuando exista backend GPU.");
    if cfg!(debug_assertions) {
        println!("Para mediciones válidas: cargo run --release -p estudio-daw-compute-runtime --bin compute-benchmark");
    }
}

/// FFT radix-2 CPU que comparte implementación con el runtime.
fn benchmark_fft(size: usize, repetitions: usize) -> u128 {
    let input: Vec<f32> = (0..size).map(|i| (i as f32).sin()).collect();
    let started = Instant::now();
    for _ in 0..repetitions {
        let mut real = input.clone();
        let mut imaginary = vec![0.0_f32; size];
        fft_in_place(&mut real, &mut imaginary, false).expect("tamaño válido");
        black_box((real, imaginary));
    }
    started.elapsed().as_micros() / repetitions as u128
}

fn benchmark_convolution(samples: usize, kernel_size: usize, repetitions: usize) -> u128 {
    let input: Vec<f32> = (0..samples).map(|i| (i as f32 * 0.01).sin()).collect();
    let kernel: Vec<f32> = (0..kernel_size)
        .map(|i| (i as f32 / kernel_size as f32).cos())
        .collect();
    let started = Instant::now();
    for _ in 0..repetitions {
        let mut output = vec![0.0_f32; samples];
        for sample in kernel_size..samples {
            output[sample] = kernel
                .iter()
                .enumerate()
                .map(|(tap, coefficient)| input[sample - tap] * coefficient)
                .sum();
        }
        black_box(output);
    }
    started.elapsed().as_micros() / repetitions as u128
}
