//! Comprueba que el instrumento consume MIDI y renderiza bloques sin heap.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

use estudio_daw_audio_engine::{AudioBlock, RenderPlanBuilder};
use estudio_daw_synth::{
    midi_event_queue, SineSynthNode, SoundFontInstrumentWorker, SynthMidiEvent,
};

thread_local! {
    static TRACKING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

struct CountingAllocator;

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn record_allocation() {
    let _ = TRACKING.try_with(|enabled| {
        if enabled.get() {
            let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
        }
    });
}

// SAFETY: todas las operaciones se delegan al allocator del sistema. El
// contador thread-local sólo observa asignaciones del hilo RT simulado.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record_allocation();
        unsafe { System.realloc(pointer, layout, size) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[test]
fn synth_processes_queued_notes_without_allocating() {
    let (mut sender, receiver) = midi_event_queue();
    let synth = SineSynthNode::new(48_000, 2, receiver).unwrap();
    let mut builder = RenderPlanBuilder::new();
    builder.add_node(synth);
    let mut plan = builder.build().unwrap();
    let mut block = AudioBlock::new(2, 64).unwrap();
    assert!(sender.try_send(SynthMidiEvent::NoteOn {
        channel: 0,
        note: 60,
        velocity: 96,
    }));
    plan.process_block(&mut block).unwrap();
    assert!(sender.try_send(SynthMidiEvent::NoteOff {
        channel: 0,
        note: 60,
    }));

    ALLOCATIONS.with(|count| count.set(0));
    TRACKING.with(|enabled| enabled.set(true));
    for _ in 0..1_000 {
        block.clear();
        plan.process_block(&mut block).unwrap();
    }
    TRACKING.with(|enabled| enabled.set(false));

    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
}

#[test]
fn soundfont_pcm_callback_source_does_not_allocate() {
    let path = "/usr/share/soundfonts/FluidR3_GM.sf2";
    if !std::path::Path::new(path).is_file() {
        return;
    }
    let (_worker, source) = SoundFontInstrumentWorker::start(path, 48_000, 0, 0).unwrap();
    let mut builder = RenderPlanBuilder::new();
    builder.add_node(source);
    let mut plan = builder.build().unwrap();
    let mut block = AudioBlock::new(2, 64).unwrap();
    plan.process_block(&mut block).unwrap();

    ALLOCATIONS.with(|count| count.set(0));
    TRACKING.with(|enabled| enabled.set(true));
    for _ in 0..1_000 {
        block.clear();
        plan.process_block(&mut block).unwrap();
    }
    TRACKING.with(|enabled| enabled.set(false));

    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
}
