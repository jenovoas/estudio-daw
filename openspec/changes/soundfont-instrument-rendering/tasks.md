# Tasks

## 1. Instrument contract and dependency boundary

- [x] 1.1 Add serializable SoundFont instrument configuration with a local asset reference, optional content hash, and bank/program selection; verify round-trip and v0/v1 migration defaults in project-model tests (19 passed). Missing-file diagnostics remain in the native-adapter tests, not the portable model.
- [x] 1.2 Add an optional, narrowly scoped FluidSynth runtime adapter with runtime/version detection and required license notices; verify a deterministic missing-library diagnostic, local runtime rendering, and that the sine path still builds (`cargo test -p estudio-daw-synth`).

## 2. SoundFont rendering worker

- [x] 2.1 Implement worker-owned FluidSynth initialization, SoundFont loading, preset enumeration/selection, and MIDI note dispatch; verify audible PCM from the locally installed SF2 without checking the asset into Git.
- [x] 2.2 Add a bounded MIDI command queue and preallocated PCM ring; verify ordering, empty/full/disconnected behavior, nonblocking overflow accounting, and clean worker shutdown. Counters are exposed by `SoundFontInstrumentWorker`.
- [x] 2.3 Add the render-graph audio-source adapter that only drains PCM and emits silence/counts underruns when empty; verify the worker-to-node path and zero allocations in the callback test.
- [x] 2.4 Add non-destructive backend replacement at a render-plan boundary; verify a failed SoundFont load leaves the previous instrument and project playable. `RenderPlanProcessor` adopts a fully prepared plan at block start; plans retain their instrument workers, and the control endpoint alone reclaims retired plans/resources. Tests verify the failed local SF2 load leaves the active plan rendering, handoff adds zero callback allocations, and destruction stays off the callback.

## 3. Project and CLI integration

- [x] 3.1 Persist the selected SoundFont reference and preset without copying the asset; verify project save/reopen and v0/v1 migration defaults for existing sine-based projects.
- [x] 3.2 Allow `midi-synth-live` to select either the sine source or a local SoundFont preset while preserving the take format; CLI option parsing is tested and the hardware recording path remains shared.
- [x] 3.3 Allow `midi-synth-play` to replay the same take through either backend; the same timestamped MIDI scheduler and take format drive both render plans.

## 4. Verification and operating guide

- [x] 4.1 Add deterministic worker/bridge tests for event timing, starvation recovery, and callback allocation constraints; verified with `cargo test --workspace`.
- [x] 4.2 Document local runtime installation, SoundFont path/preset selection, runtime and asset licensing, diagnostics, and fallback; CLI option tests cover the documented forms and no sample bank is bundled.
- [ ] 4.3 Run the full live KeyLab → FluidSynth SoundFont → PipeWire capture-and-playback integration on Linux hardware; saved KeyLab take replayed through FluidSynth/PipeWire (37,120 callbacks, 0 MIDI drops, 0 PCM underruns, 0 worker errors). A 3-second live stream opened the KeyLab/AudioBox path with no intentional note sequence (8,960 callbacks, 0 MIDI drops, 0 PCM underruns, 0 worker errors). The PCM ring peaked at 2048/2048 frames (42.67 ms at 48 kHz); replay ended at 288 frames and live capture at 800 frames. Requested PipeWire period: 32 frames (0.67 ms). These are queue-depth/buffer-equivalent observations, not end-to-end latency. Still requires deliberate KeyLab note verification and physical MIDI-to-AudioBox loopback latency measurement; subjective sound-quality assessment also remains open.
