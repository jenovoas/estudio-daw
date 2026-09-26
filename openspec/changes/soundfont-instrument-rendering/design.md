# Design

## Context

See `proposal.md` for the motivation and `specs/soundfont-instrument/spec.md` for observable behavior. `SineSynthNode` currently receives MIDI inside the existing render plan. The PipeWire callback must remain allocation-free and non-blocking. FluidSynth's official audio-rendering API documents that render calls block and belong to its synthesis thread; SoundFont loading also performs work unsuitable for the callback ([rendering API](https://www.fluidsynth.org/api/group__audio__rendering.html), [synthesis context](https://www.fluidsynth.org/api/synth-context.html), [SoundFont loading](https://www.fluidsynth.org/api/LoadingSoundfonts.html)).

## Goals / Non-Goals

**Goals:**

- Render a user-selected local SF2 preset through the existing PipeWire output and DSP graph.
- Keep the PipeWire callback independent of FluidSynth calls, file access, and locks.
- Keep the sine node and current MIDI take format working.
- Make missing runtime/font and audio starvation observable.

**Non-Goals:**

- Ship a SoundFont, claim that a SoundFont is license-free, or package a specific piano library.
- Reimplement a sample-based synthesizer or create an instrument editor in this change.
- Claim zero added latency; the worker-to-callback PCM queue is observable, while end-to-end latency requires a physical MIDI/audio loopback measurement.
- Replace the existing external ALSA MIDI destination workflow.

## Decisions

### Use the official FluidSynth engine behind an optional narrow runtime adapter

The first backend uses the official `libfluidsynth` C API loaded as a system shared library. Keep FFI isolated in the synth/platform boundary, and do not expose FluidSynth types through the portable project model. The old `fluidsynth` Rust crate found during research is version 0.0.1 and dates from 2018; do not depend on it without a separate maintenance and safety audit. `fluidlite` is a maintained-looking safe wrapper candidate but targets a different, minimal FluidLite engine; it remains an alternative, not an invisible substitute. FluidSynth is LGPL-2.1 licensed; dynamic system linking and required notices are preferred, subject to a distribution-license review ([upstream license](https://github.com/FluidSynth/fluidsynth/blob/master/LICENSE), [upstream licensing FAQ](https://www.fluidsynth.org/wiki/LicensingFAQ/)).

Alternatives considered:

- **Write a SoundFont engine in Rust:** rejected for this slice; it would duplicate mature format parsing, sample playback, modulators, and envelopes.
- **Call FluidSynth directly from `RenderPlan::process_block`:** rejected because its render API blocks and requires synthesis-context ownership, and the callback contract forbids blocking.
- **Launch only the standalone `fluidsynth` executable:** retained as an existing interoperability option, but rejected as the primary backend because its audio stream and presets would not be represented as an instrument source in our render graph.

### Give one worker exclusive ownership of the FluidSynth instance

Create the synth, load/unload the SoundFont, select presets, dispatch MIDI, render samples, and destroy the synth on one dedicated worker thread. This follows FluidSynth's synthesis-thread model and avoids concurrent calls into mutable synth state. Control commands use a bounded queue; the worker publishes preallocated float audio blocks through a bounded single-producer/single-consumer queue. Queue capacity, block sizing, startup pre-roll, and sample rate are configured outside the callback.

The render-plan adapter only drains available PCM frames into its preallocated `AudioBlock`. It never calls the C API or waits for the worker. If the queue is empty it writes silence for missing frames and increments an underrun counter. A successful instrument load is swapped at a safe plan boundary; the old instrument remains active until then. A failed replacement does not invalidate the active plan.

Render-plan replacement uses two preallocated ownership slots shared by a
single control producer and the audio processor. The producer fully constructs
the replacement before publishing its slot with release ordering. At the next
`RenderPlanProcessor::process` boundary, the callback adopts that slot and marks
the old slot retired. It never destroys a plan or node; `RenderPlanControl`
reclaims retired plans from the control thread. A second publication is refused
until that reclamation completes. Instrument workers are retained by their
`RenderPlan`, so a SoundFont worker stays alive while its plan is pending or
active; reclaiming the retired plan releases that ownership off-RT. This keeps
node/worker destructors and their joins out of the audio callback. PipeWire
exposes controlled output and duplex entry points so a host can retain the
control endpoint while streaming.

### Persist references, not sample-bank bytes

Portable project state records the backend, local SoundFont reference, optional expected content hash, and bank/program selection. It never embeds or copies the SoundFont. Resolution supports a project-relative asset when the user has deliberately placed it in project media, otherwise a local external path; missing references produce a recoverable diagnostic. Bundling and sharing projects with third-party SoundFonts is outside this change.

### Preserve explicit fallback behavior

The sine source remains available for tests and systems without FluidSynth. Selecting the SoundFont backend never silently changes the instrument. If the selected backend cannot start, report the specific reason and let the caller explicitly choose the sine source or retry.

## Risks / Trade-offs

- **[Worker scheduling can add latency or underrun]** → expose current/peak PCM queue depth and underruns; document buffer-equivalent queue duration separately from end-to-end latency. Measure key-to-audio latency with a physical MIDI/audio loopback on target hardware. Do not block the callback to hide starvation.
- **[Native ABI or library version mismatch]** → validate required symbols and minimum runtime version before creating the instrument; return an actionable compatibility error.
- **[SoundFont formats and presets vary]** → start with SF2, enumerate presets from the selected file, and test with a user-provided fixture without checking copyrighted banks into Git.
- **[SoundFont licensing is independent of FluidSynth]** → store no bundled bank; show the external path and document asset-license responsibility.
- **[Extra FluidSynth threads could compete with PipeWire]** → configure one synthesis owner and conservative internal worker settings initially; benchmark before exposing parallel FluidSynth cores.

## Migration Plan

No existing project or take schema is removed. Add an optional instrument configuration with an explicit backend discriminator and a migration default that preserves existing projects as the current sine/test instrument. The CLI keeps its current commands and gains explicit SoundFont selection. If the optional runtime is absent, existing sine and external MIDI routes continue to work. Rollback consists of selecting the sine instrument; project MIDI remains unchanged.

## Open Questions

- Which user-provided SoundFont will be used for the first manual listening test? This does not block implementation; tests must not depend on redistributing it.
