# Design

## Context

See `proposal.md` and `specs/audio-runtime-control/spec.md`. The Linux runtime currently has `PipeWireStreamConfig.period_frames` and a separate maximum PCM ring capacity, while the FluidSynth worker limits queued PCM internally. The default period is 256 frames and the SoundFont queue target is currently fixed at two 256-frame chunks. The application API and Tauri shell do not yet expose audio-runtime settings.

The PipeWire requested period is a negotiation request; the active graph quantum can differ. Device block negotiation and the DAW's queued playback frames therefore need separate models and readouts. Profile choices must not be described as hardware cache controls.

## Goals / Non-Goals

**Goals:**

- Give the DAW a backend-neutral settings model for requested device period, playback safety frames, and live/playback profile selection.
- Report requested and effective settings, including durations derived from sample rate and frame counts.
- Provide practical starting profiles that the user can tune per selected audio device.
- Keep MIDI and recording state intact when settings change.

**Non-Goals:**

- Promise a fixed key-to-speaker latency or dropout-free operation for any profile.
- Change AudioBox firmware/Universal Control settings, or emulate PreSonus Studio One's Dropout Protection.
- Change sample rate automatically or measure analog DAC-to-speaker latency.
- Implement the entire multiphase monitor/track engine in this change; settings must accurately disclose where the current engine cannot separate playback and monitoring buffers.

## Decisions

### Keep device period and playback safety buffer as separate values

Represent the requested device period in frames and the additional playback safety buffer in frames. Derive milliseconds from the active sample rate for display, rather than storing a rounded duration that could become stale after sample-rate changes. Keep backend negotiated period as a separate reported field. On PipeWire, pass the requested period through the existing stream configuration and query/report the actual active quantum using the backend's runtime information.

Alternative: expose one “latency” slider. Rejected because it conflates backend scheduling blocks with queued rendered audio and cannot explain which path acquires the added delay.

### Store preferences outside project audio content

Keep audio-device preferences and profile values in DAW user settings scoped to a device/backend identity, with in-memory fallback if persistence is unavailable. A session can select which profile is active, but projects do not overwrite machine-specific hardware preferences when opened. This avoids transferring an AudioBox-specific quantum into a different machine or interface.

Alternative: serialize the selected hardware period in `project.json`. Rejected because device support and allowed periods differ across machines; project intent and physical audio configuration have different portability.

### Profiles are editable presets over the same controls

Provide Live/Record and Multitrack Playback presets that populate device period and playback safety buffer. The user can tune both fields and save those values as personal settings. The profile name is a convenience label, not an automatic detector of session load. During overdubbing, the DAW can retain a low-latency monitored input path while buffering eligible backing playback only after the engine has distinct path scheduling. Until then, the UI reports the current shared-path limitation.

Alternative: automatically increase buffers when CPU use rises. Deferred because automatic mode switching can disrupt monitoring and the current engine has no per-path deadline/scheduling control.

### Apply changes through the control plane

Create and validate the new runtime configuration outside the callback. If the backend supports live renegotiation, publish/apply at a stream-safe boundary; otherwise stop/reopen the stream through the control plane and show a pending/restart state. MIDI queues and note/controller state must remain owned outside the callback and be reconciled explicitly across stream replacement. Sustain CC64 is ordinary timestamped controller state and must not be dropped by the profile transition.

Alternative: mutate PipeWire settings directly in the callback. Rejected because callback work must remain bounded and the existing architecture prohibits control I/O there.

### Show requested and effective values explicitly

The settings view displays the requested period, backend-reported effective period, sample rate, playback safety frames, and each known duration in frames/ms. If a value is not reported by a backend, mark it unavailable. Do not add the independent values into a single purported end-to-end latency figure.

## Risks / Trade-offs

- **[PipeWire may negotiate a different graph quantum]** → show requested and effective values separately, with backend diagnostics.
- **[Changing a running stream may interrupt audio]** → use a backend-supported safe update or clearly indicate when transport stop/restart is needed; preserve pending MIDI and recording data.
- **[Playback safety frames may not be independently consumed by the current render graph]** → implement the control contract and diagnostics first; do not imply independent monitoring latency until per-path buffering exists.
- **[Device period and safety buffer affect different parts of the signal path]** → label each control and explain frame/sample-rate conversion in the UI.
- **[Some backends expose incomplete period data]** → represent unknown effective values explicitly and avoid guessing.

## Migration Plan

1. Add backend-neutral audio runtime settings and diagnostics with defaults matching current runtime behavior (48 kHz default and 256-frame requested period where applicable; zero additional playback safety frames until consumed by an independent path).
2. Add backend support for requested/effective period reporting without changing existing CLI defaults.
3. Wire settings through the application control API and expose the two profiles in the DAW settings UI.
4. Add playback safety buffering only where the engine can apply it to eligible playback work; until then present its support state accurately.
5. Persist per-device user preferences in a versioned settings file. Existing projects require no migration.

Rollback removes the UI preference binding and returns to the existing stream configuration defaults; projects and MIDI takes remain unchanged.

## Open Questions

- The exact default frame values for the two profiles should be confirmed by deterministic runtime tests and measurements on supported backends during implementation; presets remain editable and are not guarantees.
- Future backends may expose different legal buffer increments; the settings model should allow backend validation to provide the supported choices rather than hard-coding AudioBox-specific values.
