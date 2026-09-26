# Tasks

## 1. Instrument contract and dependency boundary

- [ ] 1.1 Add serializable SoundFont instrument configuration with a local asset reference, optional content hash, and bank/program selection; verify round-trip and missing-file diagnostics in project-model tests.
- [ ] 1.2 Add an optional, narrowly scoped FluidSynth runtime adapter with runtime/version detection and required license notices; verify startup diagnostics when the library is unavailable and that the existing sine path still builds.

## 2. SoundFont rendering worker

- [ ] 2.1 Implement worker-owned FluidSynth initialization, SoundFont loading, preset enumeration/selection, and MIDI note dispatch; verify against a local SF2 file without checking the asset into Git.
- [ ] 2.2 Add a bounded MIDI command queue and preallocated PCM block queue; verify ordering, full/empty behavior, clean shutdown, and documented queue counters.
- [ ] 2.3 Add the render-graph audio-source adapter that only drains PCM and emits silence/counts underruns when empty; verify fixed-block processing and zero allocations in the audio callback.
- [ ] 2.4 Add non-destructive backend replacement at a render-plan boundary; verify a failed SoundFont load leaves the previous instrument and project playable.

## 3. Project and CLI integration

- [ ] 3.1 Persist the selected SoundFont reference and preset without copying the asset; verify save/reopen and migration defaults for existing sine-based projects.
- [ ] 3.2 Allow `midi-synth-live` to select either the sine source or a local SoundFont preset while preserving the take format; verify note capture and routing with the command documented in `docs/native-instrument.md`.
- [ ] 3.3 Allow `midi-synth-play` to replay the same take through either backend; verify identical MIDI event counts and no event loss for both choices.

## 4. Verification and operating guide

- [ ] 4.1 Add deterministic worker/bridge tests for event timing, starvation recovery, and callback allocation constraints; verify with the relevant synth and audio-engine test suites.
- [ ] 4.2 Document local runtime installation, SoundFont path/preset selection, runtime and asset licensing, diagnostics, and fallback; verify every CLI example parses and references no bundled sample bank.
- [ ] 4.3 Run the KeyLab → FluidSynth SoundFont → PipeWire → recorded-take playback integration on Linux hardware; record measured queue depth, underruns, and added latency without marking subjective sound quality as an automated test.
