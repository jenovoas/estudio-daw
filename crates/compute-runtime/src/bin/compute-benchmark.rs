//! Benchmarks CPU de referencia para calibrar la política CPU/GPU.
//!
//! No intenta ser un benchmark científico todavía. Su propósito es producir
//! números reproducibles del hardware actual antes de implementar kernels
//! `wgpu`, y detectar regresiones con el mismo tamaño de trabajo.

use std::{hint::black_box, time::Instant};

fn main() {
    let dft_micros = benchmark_dft(1024, 3);
    let convolution_micros = benchmark_convolution(65_536, 64, 3);
    println!("Estudio DAW compute baseline (CPU)");
    println!("dft_1024_us={dft_micros}");
    println!("convolution_65536x64_us={convolution_micros}");
    println!("Estos valores calibrarán el scheduler cuando exista backend GPU.");
}

/// DFT pequeña, deliberadamente simple, para tener una referencia estable.
/// No debe confundirse con una FFT: su coste es O(N²). La FFT optimizada
/// llegará en la siguiente fase junto con la comparación CPU/GPU.
fn benchmark_dft(size: usize, repetitions: usize) -> u128 {
    let input: Vec<f32> = (0..size).map(|i| (i as f32).sin()).collect();
    let started = Instant::now();
    for _ in 0..repetitions {
        let mut output = vec![0.0_f32; size];
        for (frequency, value) in output.iter_mut().enumerate() {
            let mut sum = 0.0;
            for (sample, input) in input.iter().enumerate() {
                let phase = (frequency * sample) as f32 / size as f32;
                sum += input * (phase * std::f32::consts::TAU).cos();
            }
            *value = sum;
        }
        black_box(output);
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
