# Proposal

## Why

The current native instrument proves MIDI capture and the real-time render path, but its sine tone is only a diagnostic sound and cannot support meaningful practice or production. A local SoundFont instrument would provide familiar piano, bass, guitar, and orchestral presets without cloud use or per-play charges, while reusing a mature open-source synthesis engine instead of recreating sample playback.

## What Changes

- Add an optional SoundFont instrument backend based on the system `libfluidsynth` library, with a user-selected local SoundFont file and preset.
- Keep the sine instrument available as a small, dependency-free fallback and test source.
- Isolate SoundFont loading, MIDI dispatch, and FluidSynth rendering from the PipeWire callback; exchange bounded MIDI commands and preallocated PCM blocks with the render graph.
- Report missing library/font, invalid preset, render-worker failure, and PCM underruns clearly; never silently replace a selected instrument.
- Persist instrument configuration as a reference to user-owned media; do not bundle or redistribute SoundFont files.

## Capabilities

### New Capabilities

- `soundfont-instrument`: local SoundFont instrument configuration, lifecycle, MIDI response, audio delivery, and diagnostics.

### Modified Capabilities

- None. The existing runtime callback contract is preserved; this change adds an instrument capability behind that contract.

## Impact

- Likely affected crates: `synth`, `audio-engine`, `audio-platform`, `application`, and `cli`; the exact boundary is settled in the design artifact.
- Adds an optional system runtime dependency on FluidSynth, dynamically linked so users can install or update it independently. The library is LGPL-licensed; include required notices and audit the selected Rust binding before adopting it.
- Project state will reference, but will not copy, the user's SoundFont. SoundFont licenses remain the user's responsibility and are not implied by the library license.
- PipeWire remains the output backend; MIDI, scheduling, and the current SineSynthNode remain usable when FluidSynth is unavailable.
