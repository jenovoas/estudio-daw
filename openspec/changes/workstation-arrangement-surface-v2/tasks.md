## 0. Reference and truthful baseline

- [x] 0.1 Review Ableton Live 12 as the primary UX reference and record its main views/menu families/workflows. Consult Ardour narrowly for audio signal-flow and media/region concepts; it is explicitly not a visual or interaction reference. Sources and this boundary are recorded in `design.md`.
- [x] 0.2 Compile the product-facing Live 12 option inventory for File/Edit/Create/View/Options/Help (including macOS settings placement) plus core track, clip, scene, browser, device and mixer contexts. Compare each option family against planned command coverage. Device/plugin-specific menus remain delegated to installed integrations; no menu parity is claimed. See the inventory and scope note in `design.md`.
- [x] 0.3 Inspect current Tauri commands, project/command models, runtime paths and UI handlers. Record existing and missing capabilities in the source-reviewed implementation matrix in `design.md`.
- [x] 0.4 Correct README and UI guide capability claims to describe the current shell as an experimental partial interface until the replacement passes the completion review.

## 1. Project model and commands

- [ ] 1.1 Define versioned track state for MIDI, instrument, audio, bus/return and master roles, with stable identity/order, name/color, active/mute/solo, gain/pan and channel configuration. Add serde defaults/migrations and fixtures proving existing v1/v2 projects preserve identity, order and content.
- [ ] 1.2 Define audio source, region/clip and playlist ownership explicitly: source path/signature/hash/channel/rate; source offset; timeline position; duration; gain/fades; non-destructive edit semantics. Define scenes and clip-slot identity without duplicating the same musical clip between Session and Arrangement.
- [ ] 1.3 Add typed, validated and undoable commands for add/duplicate/rename/reorder/remove/activate tracks; set supported mixer state; create/edit/delete scenes and slots; and add/trim/move/gain/fade audio regions. Define undo/redo behavior and test command rejection for incompatible track/clip types.
- [ ] 1.4 Keep Portable Domain independent of PipeWire, ALSA, GUI and media-decoding processes. Put device, file-dialog and media inspection/decode operations behind appropriate adapters/workers; document command boundaries and prevent UI-side model mutation.

## 2. Audio track vertical workflow

- [ ] 2.1 Create a typed audio track from the UI with input channel layout, master/output default, name and stable ordering. Show it in Session, Arrangement and Mixer from the same project snapshot.
- [ ] 2.2 Import supported audio with audition, file metadata, copy/link provenance choice, channel mapping and placement choice (edit point/playhead/session start). Render real waveform/region bounds and preserve the source when regions are moved, trimmed or deleted.
- [ ] 2.3 Implement bounded, off-callback audio decode/buffering and timeline playback for imported clips, including seek, clip start/source offset, duration, loop, fades and gain; mix with MIDI/instrument tracks through the compiled audio plan. No simulated waveform playback or device restart on edit.
- [ ] 2.4 Implement track input/output selection, channel mapping and visible route direction through a platform adapter; reflect actual device availability and expose actionable errors. Do not show record-arm/monitor as functional before an audio capture path exists.
- [ ] 2.5 Implement audio recording/monitoring as a complete capture-to-project-region workflow, including arm, selected input, count-in/punch options supported by the runtime, file lifecycle, undoable region creation and playback verification. Otherwise keep those menu items absent/disabled with a specific reason.

## 3. Shared workstation and creative views

- [ ] 3.1 Replace the rejected card/timeline shell with a dense DAW work surface: control/transport bar, categorized browser, Session or Arrangement center surface, consistent track identity, contextual lower Clip/Device editor, and reachable mixer/status areas. Use Live's information hierarchy and interaction density without copying artwork.
- [ ] 3.2 Implement Session View as a playable track-by-scene matrix for both MIDI and audio clip types, with empty slots, per-track stop, per-scene launch, selected/queued/playing states and stable scene names. Reordering tracks/scenes keeps all references valid.
- [ ] 3.3 Implement Arrangement View for the same tracks with musical and time rulers, playhead, markers, audio waveforms and MIDI regions. Support selection, move, trim, split, duplicate, loop/range, snap/grid and undo/redo through commands, with correct audio source-offset semantics.
- [ ] 3.4 Implement Session↔Arrangement relationship and capture/commit semantics: shared track/mixer state, Session clip precedence, clear Back-to-Arrangement behavior, and switching views without stopping transport or losing content.
- [ ] 3.5 Implement a contextual lower editor selected by content: MIDI piano roll and note properties for MIDI; waveform/sample region and source controls for audio; device parameters only for loaded real devices. Changes use commands and appear in both views.
- [ ] 3.6 Implement the browser with search, navigation history, Collections/favorites, Library categories, user Places, type/filter metadata, preview/audition, drag/drop import and provenance. Never invent installed sounds, plugins, licensed content or preview behavior.

## 4. Mixer, transport and runtime

- [ ] 4.1 Implement shared track controls for active, mute, solo, gain, pan, meter and selection/group behavior in Session, Arrangement and Mixer. Values stay consistent between surfaces and persist in the project.
- [ ] 4.2 Implement mixer strips that show the real signal path (input → processors → pan/gain → meter → output), track order, master, and actual supported sends/buses. Enable I/O, monitor, plugin, automation and control-master actions only when end-to-end implementations exist.
- [ ] 4.3 Replace wall-clock-only UI transport state with the deterministic session transport clock; expose musical position/tempo/meter, play/pause/stop, loop, click, navigation and all-notes-off with correct pause/resume and clip-duration semantics.
- [ ] 4.4 Implement bounded scheduler commands for MIDI and audio clip/scene launch with per-clip/global quantization, launch mode and stop/replacement behavior. Keep the callback allocation-free and avoid rebuilding PipeWire on each launch; test unrelated tracks continue and voices stop correctly.
- [ ] 4.5 Add routing, recording, automation, buses/returns, plugin chain, latency compensation and export families as complete vertical slices with project persistence, commands, undo policy and runtime behavior. Until a slice lands, the corresponding commands stay out of enabled menus.

## 5. Menu, shortcut and command coverage

- [ ] 5.1 Add one typed command/action registry as the source for menu labels, shortcuts, context-menu filtering, enablement, accessibility and dispatch. Every visible actionable item maps to a real handler and reports success/error/state.
- [ ] 5.2 Cover Project/File and Session options: new/open/recent/close/save/save-as/rename/snapshot/template/archive/import/export/mix/stems/metadata/media cleanup, with each item enabled only when implemented and applicable.
- [ ] 5.3 Cover Edit and Region/Clip options: undo/redo, selection and clipboard, split/separate/combine/consolidate/align/fades/analyze, MIDI transpose/quantize/transform, gain/trim/lock/nudge/duplicate/bounce/export/remove and range/marker operations. Add only after their typed commands and undo rules exist.
- [ ] 5.4 Cover Track, Create and Mixer options: create MIDI/audio/instrument/bus/return/master tracks, duplicate/remove/reorder/height/visibility, playlists/takes, arm/solo/mute/active and mixer operations, aligned to the audio/MIDI capabilities actually implemented.
- [ ] 5.5 Cover Transport and View/Window options: play/stop/record/selection/loop/punch/count-in/click/panic/playhead navigation; Session/Arrangement/Editor/Mixer/Browser/detail visibility; meters, connections, diagnostics and preferences. Device-specific options route to the platform adapter.
- [ ] 5.6 Cover Settings/Options and Help: audio buffer/rate/device, MIDI inputs, library/media paths, display/theme, recording/launch behavior, keyboard/ MIDI mapping, help/reference/about. Each setting persists at the right scope (application vs project) and has a live effect or an honest restart requirement.
- [ ] 5.7 Add keyboard shortcuts and context menus for high-frequency workflows; expose discoverable shortcut editing. Verify shortcut conflicts, focused text-entry behavior and equivalent command dispatch.

## 6. Completion and honest release state

- [ ] 6.1 Add deterministic checks for schema migration, all reversible commands, Session/Arrangement identity, menu/action registry completeness, import/media provenance, audio clip boundaries, transport/quantization and undo/redo. Add runtime tests for callback safety and bounded playback.
- [ ] 6.2 Run formatting and the required crate/workspace checks after implementation. Record exact commands and results; do not mark failures or hardware-only checks as passed.
- [ ] 6.3 Inspect the running UI at standard and minimum supported sizes and review each primary workflow. Correct actual visual/interaction issues found; source inspection alone does not satisfy visual review.
- [ ] 6.4 Review README, OpenSpec, user guide and audit log against executable code and tests. Record completed versus deferred menu families, known limits, initial/final SHA and audit entry. Do not commit/publish until this review is accurate.
