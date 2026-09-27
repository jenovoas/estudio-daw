# Design

## Runtime behavior

- In the PipeWire callback, a paused stream fills the device buffer with silence and does not call `RenderPlanProcessor::process`; this freezes node state and avoids popping SoundFont PCM. The MIDI scheduler already offsets event deadlines by paused duration and remains the transport clock for this playback adapter.
- Clip events are eligible when their local tick is at or before `duration_ticks`, preserving a NoteOff exactly on the final boundary. Their absolute position is `start_tick + event.tick`.
- The render-plan exchange must return silence on an absent active plan instead of unwinding in the audio callback. SineSynth clamps NoteOn pitch to 0–127 before indexing its frequency table.
- The UI demo take derives microseconds from its own PPQ and tempo constants.

## UI direction

Keep the existing Tauri command bridge and data model. Restyle the shell around a compact DAW transport, strong arrangement grid, readable track headers, colored MIDI regions with note previews, and a restrained dark palette with warm orange accents. Controls remain tied only to existing commands; do not imply editing or recording capabilities that are not implemented.

## Scope boundary

This change does not implement an audio DAG or alter crate dependency boundaries. Those are independent architectural changes and must receive their own explicit capability/task plan. Existing `RenderPlanBuilder::connect` semantics remain a known audit item until that follow-up is specified.

## Verification

Use deterministic unit tests for clip bounds, MIDI timing conversion, pitch safety, and render-plan fail-safe behavior. Run formatting, the complete single-threaded workspace suite, workspace check, strict OpenSpec validation, and diff checks before publishing.
