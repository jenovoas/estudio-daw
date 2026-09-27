# Tasks

## 1. Playback correctness and realtime safety

- [x] 1.1 Freeze RenderPlan processing and PCM consumption in the paused PipeWire callback.
- [x] 1.2 Bound scheduled MIDI events by each clip's duration while preserving start offsets and final-boundary NoteOff events.
- [x] 1.3 Clamp SineSynth note-on pitch and replace callback `expect` with fail-safe silence.
- [x] 1.4 Derive demo MIDI microsecond metadata from its declared PPQ and tempo.
- [x] 1.5 Add regression tests for clip boundaries, out-of-range notes, and empty render-plan slots.
- [x] 1.6 Keep domain and device transport state consistent when starting playback fails.
- [x] 1.7 Freeze SoundFont worker rendering during Pause and verify its PCM queue stops advancing.

## 2. Desktop workstation presentation

- [x] 2.1 Restyle the Tauri shell to make the Live-inspired arrangement workspace the visual focus.
- [x] 2.2 Keep control labels and capability notices accurate to implemented behavior.
- [x] 2.3 Update README and relevant audio/UI docs to match the connected playback behavior and limits.

## 3. Verification and traceability

- [x] 3.1 Run `cargo fmt --all`, `cargo test --workspace -- --test-threads=1`, `cargo check --workspace`, strict OpenSpec validation, and `git diff --check`.
- [x] 3.2 Update the project handoff and append a factual vault log entry with SHA, files, tests, outcome, and remaining audit items.
