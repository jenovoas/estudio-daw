# Proposal

## Why

The desktop transport can drift from the rendered audio when paused, and clip playback does not consistently respect clip bounds. The current desktop surface also fails to communicate a usable DAW arrangement workflow and falls short of the previously approved Ableton Live 12 visual direction.

## What Changes

- Freeze audio DSP/PCM consumption while paused and keep MIDI scheduling aligned across pause/resume.
- Schedule only MIDI events within each clip's start and duration; derive demo take timing from its declared PPQ and tempo.
- Make real-time playback fail safely without panicking and constrain SineSynth pitches to MIDI's valid range.
- Rework the Tauri frontend into a dense, dark arrangement workspace with clear transport, track, clip, and audio settings hierarchy inspired by Live 12.
- Correct product documentation to describe the connected Tauri playback path and state the remaining experimental limits.

## Capabilities

### New Capabilities

- `runtime-midi-playback`: clip-bounded MIDI scheduling, pause semantics, and fail-safe callback behavior.
- `desktop-workstation-ui`: a DAW-oriented desktop arrangement and transport surface.

### Modified Capabilities

None. The repository has no synchronized main specs yet; these new capabilities establish the behavior contracts without pretending older deltas are already canonical.

## Impact

Affected code includes `crates/audio-platform`, `crates/audio-engine`, `crates/synth`, and `crates/ui-shell`. README and native audio/render-plan documentation will be aligned. No serialized project format or hardware routing changes are intended.
