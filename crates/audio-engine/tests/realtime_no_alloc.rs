//! Regresión del contrato de no asignación en el camino síncrono de audio.
//!
//! Este test vive en un binario de integración separado: el allocator global
//! no modifica el harness de pruebas unitarias del crate. El conteo es local al
//! hilo probado, para ignorar asignaciones de hilos ajenos del test runner.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

use estudio_daw_audio_engine::{
    render_plan_exchange, AudioBlock, GainNode, RenderPlanBuilder, SampleRingBuffer,
};

thread_local! {
    static TRACKING_ENABLED: Cell<bool> = const { Cell::new(false) };
    static TRACKED_ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

struct ThreadLocalTrackingAllocator;

#[global_allocator]
static TEST_ALLOCATOR: ThreadLocalTrackingAllocator = ThreadLocalTrackingAllocator;

fn record_allocation_on_this_thread() {
    // try_with evita hacer panicking si una asignación ocurre durante la
    // inicialización del propio almacenamiento thread-local.
    let _ = TRACKING_ENABLED.try_with(|enabled| {
        if enabled.get() {
            let _ = TRACKED_ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
        }
    });
}

// SAFETY: cada operación delega exactamente en el allocator global `System`;
// el contador thread-local no toma locks y sólo está activo durante el tramo
// síncrono que queremos observar.
unsafe impl GlobalAlloc for ThreadLocalTrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation_on_this_thread();
        // SAFETY: `layout` se recibe del contrato de GlobalAlloc.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record_allocation_on_this_thread();
        // SAFETY: `layout` se recibe del contrato de GlobalAlloc.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record_allocation_on_this_thread();
        // SAFETY: `ptr` y `layout` mantienen el contrato de GlobalAlloc.
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` y `layout` mantienen el contrato de GlobalAlloc.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[test]
fn processing_preallocated_audio_and_ring_does_not_allocate() {
    // Toda construcción y calentamiento ocurre antes de activar el contador:
    // el contrato medido comienza en la entrada al procesamiento del bloque.
    let mut builder = RenderPlanBuilder::new();
    builder.add_node(GainNode::new(0.75));
    let mut plan = builder.build().expect("el plan de prueba debe compilar");
    let mut block = AudioBlock::new(2, 64).expect("el bloque debe reservarse antes");
    let ring = SampleRingBuffer::new(256);
    let mut output = [0.0_f32; 128];

    // Calienta código y TLS fuera de la sección observada.
    block.samples_mut().fill(1.0);
    plan.process_block(&mut block)
        .expect("el calentamiento DSP debe funcionar");
    assert_eq!(ring.push(block.samples()), 128);
    assert_eq!(ring.pop(&mut output), 128);

    TRACKED_ALLOCATIONS.with(|count| count.set(0));
    TRACKING_ENABLED.with(|enabled| enabled.set(true));

    for _ in 0..1_000 {
        block.samples_mut().fill(1.0);
        plan.process_block(&mut block)
            .expect("el callback de prueba no debe fallar");
        let written = ring.push(block.samples());
        let _ = ring.pop(&mut output[..written]);
    }

    TRACKING_ENABLED.with(|enabled| enabled.set(false));
    let allocations = TRACKED_ALLOCATIONS.with(Cell::get);
    assert_eq!(allocations, 0, "se detectaron asignaciones en el camino RT");
    assert!(block.samples().iter().all(|sample| *sample == 0.75));
}

#[test]
fn render_plan_handoff_at_audio_boundary_does_not_allocate() {
    let mut initial = RenderPlanBuilder::new();
    initial.add_node(GainNode::new(1.0));
    let (control, mut processor) = render_plan_exchange(initial.build().unwrap());

    let mut replacement = RenderPlanBuilder::new();
    replacement.add_node(GainNode::new(0.5));
    assert!(control.publish(replacement.build().unwrap()).is_ok());
    let mut block = [1.0_f32; 64];
    processor.process(&mut block).unwrap();
    assert_eq!(block, [0.5; 64]);

    TRACKED_ALLOCATIONS.with(|count| count.set(0));
    TRACKING_ENABLED.with(|enabled| enabled.set(true));
    for _ in 0..1_000 {
        block.fill(1.0);
        processor.process(&mut block).unwrap();
    }
    TRACKING_ENABLED.with(|enabled| enabled.set(false));

    assert_eq!(TRACKED_ALLOCATIONS.with(Cell::get), 0);
    assert!(control.reap_retired());
}
