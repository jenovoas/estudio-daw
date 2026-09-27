# Proposal

## Why

Estudio DAW currently chooses the PipeWire period and SoundFont PCM queue depth internally, so the user cannot tune responsiveness versus dropout resistance from the DAW. Separate user-selectable live/record and multitrack playback profiles are needed, with honest feedback about the effective device period and the extra playback buffering.

## What Changes

- Add DAW-level audio settings for the requested device period and a separate playback safety buffer.
- Provide starting profiles for Live/Record and Multitrack Playback; allow the user to adjust and see the effective values.
- Preserve a low-latency monitored/live instrument path while allowing already-rendered playback tracks to use additional buffering where the engine supports it.
- Keep MIDI events, including sustain pedal CC64, intact across profile changes and recording/playback.
- Explain when PipeWire or the audio device negotiates a different effective period than requested.

## Capabilities

### New Capabilities

- `audio-runtime-control`: user-facing audio buffer profiles, effective settings, safe runtime changes, and separation of device period from playback safety buffering.

### Modified Capabilities

None. The repository has no canonical `openspec/specs` entries yet; the related runtime requirements currently live in the bootstrap architecture change. This change adds an independently verifiable capability without altering that historical plan.

## Impact

- Likely affected areas: `audio-platform`, `synth`, `application`, UI shell, and runtime diagnostics.
- PipeWire remains responsible for negotiating graph quantum; requested and effective periods must be shown separately.
- The extra playback buffer is DAW-managed and must not be labeled as an AudioBox hardware cache or as PreSonus Studio One Dropout Protection.
- Presets should be starting points, not promises of a fixed end-to-end latency or dropout-free performance.
