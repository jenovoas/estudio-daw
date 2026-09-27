# Tasks

## 1. Audio runtime settings contract

- [x] 1.1 Add serializable backend-neutral settings for requested device period, playback safety frames, selected profile, and supported/pending state; verify validation and round-trip tests.
- [x] 1.2 Add requested/effective period and sample-rate diagnostics with frame-to-millisecond conversion; verify unit tests cover differing requested/effective periods and unavailable backend values.
- [x] 1.3 Validate profile periods against PipeWire stream constraints and expose restart-required/effective-quantum-unavailable status until a connected host can report the negotiated graph quantum; verify invalid values fail and requested values are never called effective latency.

## 2. Separate playback buffering and MIDI continuity

- [x] 2.1 Make the SoundFont PCM queue target configurable outside the callback independently from PipeWire period and ring capacity; verify tests show each setting affects only its own queue metric.
- [x] 2.2 Persist profile changes through the control plane and report next-stream/restart application while the current shell has no live engine connection; verify preference writes are independent from project/take data and do not mutate the running CLI stream.
- [x] 2.3 Verify MIDI Note On/Off and CC64 sustain state remain ordered and semantically consistent across profile changes, including a pedal-held Note Off scenario.
- [x] 2.4 Identify which playback graph paths can consume extra safety frames independently of live monitoring; add deterministic tests for separated paths and a diagnostic for unsupported/shared-path operation.

## 3. Application settings and DAW controls

- [x] 3.1 Expose audio-runtime settings, profile selection, effective diagnostics, and apply/pending results through the application API; verify application-layer tests do not require hardware.
- [x] 3.2 Add Live/Record and Multitrack Playback editable profile controls in the Tauri UI, with separate device-period and playback-buffer units; verify displayed values update from application state.
- [x] 3.3 Add concise UI explanations of frame/sample-rate units, CPU/scheduling trade-offs, backend negotiation, and the current monitored-path support state; verify documentation makes no hardware-cache or end-to-end-latency claim.
- [x] 3.4 Persist settings for the current PipeWire default routing policy in a versioned user-settings file, keeping machine-specific settings out of project JSON; verify load, save, missing-file defaults, and invalid-version handling.

## 4. Integration verification and operating guide

- [x] 4.1 Document selecting and tuning both profiles, interpreting requested/effective values, and limitations of live monitoring with the current engine; verify every documented setting matches UI behavior.
- [x] 4.2 Verify profile changes during live play, recording, and playback on PipeWire hardware; record effective quantum when available, queue depth, xruns, MIDI drops, and restart behavior without presenting component measurements as total physical latency.
  - Hardware verification (2026-09-27, AudioBox USB 96, 48 kHz): Live/Record 256-frame stream observed at quantum 256 during live KeyLab input; 1 MIDI event, 0 MIDI drops. WAV recording with Live/Record 256 captured 188,928 interleaved samples with 0 discarded; the source suggested quantum 256 while the output sink driver reported 1024. Playback profile period 512 observed at sink/client quantum 512; FluidR3_GM SF2 worker with PCM target 1024 ended at 1,024/2,048 frames, with 0 FluidSynth underruns, worker errors, and MIDI drops. Changes are read by the next stream; the current Tauri shell does not connect settings to a running engine/transport. `pw-top` ERR totals observed were 235 (sink) and 16,140 (source); no per-run baseline was taken, so these are recorded as totals, not as errors caused by an individual profile. No end-to-end physical latency is claimed.
- [x] 4.3 Run `cargo fmt --all -- --check`, relevant crate tests, `cargo test --workspace -- --test-threads=1`, `git diff --check`, and `openspec validate audio-latency-profiles --strict`; record environment-dependent hardware checks separately.

### Follow-up integration status (2026-09-27)

The Tauri transport now opens a PipeWire playback stream and consumes the
selected profile when the stream starts. It schedules clips from multiple MIDI
tracks, loads each track's configured Sine or FluidSynth instrument, and
supports pause/resume/stop. This follow-up does not add audio-file clip playback
or live input recording/monitoring to the Tauri shell; `audio-record` and
`midi-synth-live` continue to consume the Live/Record profile through CLI. The
effective PipeWire quantum remains unavailable in the Tauri readout.
