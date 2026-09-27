# Design

## Context

This change corrects an earlier UI pass that treated a DAW as a dark dashboard plus a read-only timeline. That pass did not satisfy the requested Ableton Live 12 creative workflow, did not provide a usable audio-track system, and its checked tasks overstated completion. Those tasks are reopened below. The official manuals for Ableton Live 12 and Ardour were reviewed on 2026-09-27; this document records the main views, top-level menu families and principal option domains used to shape the product. It is a product reference, not a claim that the current shell implements these features. A separate unchecked task requires the full option-by-option/context-menu inventory before menu parity can be claimed.

The implementation is currently a Tauri shell with project open/save, MIDI playback, transport, history, and audio-profile settings. The project model distinguishes MIDI and audio tracks and has non-destructive audio clip metadata operations, but the shell has no complete audio-track workflow, audio clip playback, recording, real Session clip grid, or functional mixer. Source code and executable checks, not this document, determine shipped capability.

## Reference audit: Ableton Live 12

The design takes its creative flow and surface hierarchy from Live, rather than copying proprietary artwork or branding.

| Surface / menu | Reference options and behavior to account for |
| --- | --- |
| **Control Bar / Transport** | Play/stop/record, tempo and meter, metronome, global launch quantization, Session/Arrangement switching, CPU/status feedback, view and browser visibility. It keeps frequent performance controls reachable while leaving editing surfaces large. |
| **Browser** | Search; back/forward history; Collections; Library labels for All, Sounds, Drums, Instruments, Audio Effects, MIDI Effects, Modulators, Max for Live, Plug-ins, Clips, Samples, Grooves and Templates; Places for Current Project, User Library and user folders; filters/tags, preview, and drag/drop into the set. External packs/integrations appear only when actually available. |
| **Session View** | Track columns × scene rows; per-slot clip launch/stop/selection and playing/queued state; scene launch across tracks; empty slots; one active clip per track; launch quantization, launch mode, legato, velocity, clip offset/nudge, loop and follow actions; mixer section and scene controls. A Session clip takes precedence over Arrangement playback on its track. |
| **Arrangement View** | Shared tracks displayed vertically against bars/beats; audio waveforms and MIDI regions; clip move/resize/split/duplicate; loop and time selection; take lanes and comping; track controls; rulers/markers; mixer. Switching views preserves playback and the shared track identity. |
| **Clip / Device detail** | Contextual lower panel follows the selection. MIDI clips expose note editing and clip timing/loop controls; audio clips expose sample/waveform and warp controls; devices expose their parameters. The panel changes with selection rather than remaining a generic settings card. |
| **Mixer / track controls** | Shared values across Session and Arrangement; track activator, solo, record-arm where supported, pan, volume, meters, I/O and configurable mixer controls. Send/return/master and cueing are real signal-flow concepts, not decorative buttons. |
| **Application menus** | File: set/project lifecycle, collect/manage files, import/export. Edit: undo/redo, clipboard, selection and edit operations. Create: tracks, clips, scenes, time/signature/markers. View: Session/Arrangement, Browser, detail panel, Mixer and selectable track/mixer controls. Options: audio, MIDI, Link, display, library, plug-in, file and Record/Warp/Launch settings. Help: learning, reference and info. Context menus extend these actions for the selected track, clip, scene, browser item or device. Menu entries must resolve to named commands and report unavailable capability honestly. |

### Ableton Live menu and context-action inventory

This is the product-facing inventory for the main application and creative surfaces. It captures the Live 12 commands relevant to project creation, performance, editing, mixing and media management; version-specific device/plugin menus remain provided by the actual device/plugin and are not fabricated by Estudio DAW.

| Menu / context | Live 12 options observed in the official reference | Estudio DAW coverage to plan |
| --- | --- | --- |
| **File** | New Live Set; Open Live Set/Open Recent Set; Save Live Set; Save Live Set As; Save a Copy; Save Live Set As Default Set; Save Live Set As Template; Collect All and Save; Manage Files; Export Audio/Video; Export MIDI Clip; close/quit. File Manager locates missing references, collects external media and manages project/library files. | New/open/recent/save/versioned save/template; media collection and relinking; audio/stem and MIDI export; safe close/quit. |
| **Edit** | Undo/Redo; Cut/Copy/Paste/Duplicate/Delete; Rename/Edit Info Text; Select All/Deselect; Split and Consolidate in Arrangement; Loop Selection; clip/note operations including Quantize, Group Notes, clip marker commands, and selection-based operations; track deletion and edit commands vary with selection/context. | Reversible command bus for project mutations; selection and clipboard model; MIDI and audio edit operations; context-sensitive enablement and edit history. |
| **Create** | Insert Audio Track; Insert MIDI Track; Insert Return Track; Insert Scene; Capture and Insert Scene; Insert Empty MIDI Clips; Import MIDI File; Consolidate Time to New Scene; additional actions are enabled by the current view/selection. | Typed track/scene/clip creation; capture currently playing Session material without interruption; import MIDI/audio to selected track/slot; convert Arrangement time selections into scenes. |
| **View** | Session/Arrangement; Browser; Info View/Help View; Clip View and Device View (including stacked detail views); Mixer; Mixer Controls for In/Out, Sends, Returns, Volume, Track Options/Delay, Crossfader and Performance Impact; Arrangement Track Controls; File Manager/Groove Pool/other panels; full screen and second window. Exact entries vary by version and platform. | Immediate Session/Arrangement switch; persistent optional Browser/detail/Mixer panels; configurable mixer strips and Arrangement headers; zoom/fold/overview; preferences for reachable controls and saved workspace layout. |
| **Options (Windows) / Live settings menu (macOS)** | Computer MIDI Keyboard; Key Map Mode and MIDI Map Mode; Step Input Mode and MIDI Editor Note Preview; automation-related toggles; Settings/Preferences, including Display & Input, Theme & Colors, Audio, Link, Tempo & MIDI, File & Folder, Library, Plug-Ins, Record/Warp/Launch and Licenses & Updates; Accessibility commands in current Live 12. | Application-scoped audio backend/device/rate/buffer, MIDI ports/mapping, display/theme/keyboard, media/library paths, launch/record/warp behavior, accessibility and help. Clearly distinguish applied/live settings from restart-required settings. |
| **Help** | Help View/Info View, built-in lessons and learning resources, version-matched manual/reference, support/reporting and About/version information (menu placement depends on OS). | Context-sensitive help, shortcuts/reference, runtime diagnostics/logs, about/version/license; help links never imply installed capabilities. |
| **Track header context** | Rename/info text/color; create/duplicate/delete/deactivate/freeze-related track operations; fold/unfold; track width/height and mixer/arrangement control visibility; group/link/take-lane operations; arm/solo/mute and track-specific routing/device actions. | Add/duplicate/rename/reorder/delete/activate, type/color, fold/height, group and supported record/mix actions, all filtered by track kind and available engine paths. |
| **Session clip slot / scene context** | Launch/stop/select; create/insert/capture scene; clip name/info/color; duplicate/copy/paste/delete; clip launch mode, legato, velocity, quantization, offset/nudge, loop and follow actions; scene launch and scene tempo/time signature; add/remove clip stop buttons. | Same creative essentials, with persisted state, per-track replacement, scene launch and actual quantized playback; every launch state is visible. |
| **Arrangement clip / selection context** | Select/split/duplicate/move/resize/trim/reverse; loop; consolidate; fades and clip gain; warp/sample controls; crop/insert/delete time; selection/range commands; MIDI note and automation operations. | Non-destructive audio/MIDI editing, waveform/note visualization, source offsets, fades, snapping and undoable selection edits. Unsupported transformations stay absent until implemented. |
| **Browser item context** | Load/insert/preview; back/forward history; search/filter/tag/collections; rename/remove where allowed; show file extensions/columns; hot-swap/locate missing media; manage project/library and collect files. | Search/user places/favorites/metadata filters, truthful audition, drag/drop, media provenance, relinking and missing-file diagnostics. Installed licensed content appears only when discovered through a real integration. |
| **Mixer/track controls** | Track activator, Solo/Cue, Arm, pan, volume/meter; input/output choosers; sends/returns/main; track delay; crossfade assignment/curve; performance impact indicators; show/hide configurable sections. | Same core controls in Session and Arrangement over shared values. Routing/cue/sends/delay/plugin/record controls become enabled only with actual command and runtime support. |
| **MIDI Note Editor context** | Note selection/move/resize/velocity/chance; quantize and grid; group notes/play-one; transpose/transform/generate tools; clip start/end/loop markers; note preview and editor modes for envelopes/MPE. | Piano roll basics first, followed by supported expression/transform tools; edits persist and participate in undo. Advanced generators are separately specified. |

This is a high-level menu/context overview, not the exhaustive option-by-option inventory requested for task 0.2. It does not yet establish menu parity. Context menus are selection-specific and grow with device integrations; each built-in command is to be represented by the typed action registry, not copied as a list of dead buttons.

The critical Ableton principle is that Session and Arrangement are not separate projects: they are two ways to perform and edit the same tracks. The Session grid is immediately playable, and ideas can be captured/arranged on the timeline without replacing the identity of tracks or mixer state.

## Secondary technical audit: Ardour (not a UX reference)

Ardour is deliberately not a visual or creative-flow reference for this product. Consult its manual only to cross-check technical audio-production concepts where useful: source versus playlist/region ownership, channel configuration, input/processor/pan/gain/meter/output signal flow, capture/monitoring, and routing. Its menus (**Session, Transport, Edit, Region, Track, View, Window and Help**) are recorded as technical context, not as a menu layout or interaction blueprint Estudio DAW must imitate. The Ableton Live 12 model remains primary when references differ.

| Menu / surface | Reference options and behavior to account for |
| --- | --- |
| **Audio track header and mixer strip** | Technical cross-check only: track name/color, height, input selection, processor chain, panner, record/monitor, mute/solo, gain/fader and output meter, automation/group/meter point, output routing and comments. This describes signal path; it does not prescribe our screen design. |
| **Media and editing model** | Technical cross-check only: session owns sources; tracks play ordered playlists of non-destructive regions. Import can create tracks or place media at edit point/playhead/start, copy into the session, preserve channel mapping and audition before import. Track and playlist are separate concepts; regions reference media rather than being the source file. |
| **Cue Grid** | Secondary confirmation that audio/MIDI slots can share tracks/scenes, launch state, cue-row triggering and mixer integration. Ableton Session View remains the primary reference for how this creative surface should feel and behave. |

## Product menu and command policy

- Estudio DAW must reach at least the reference breadth represented by these menu families and add the planned musical analysis, code, notation and assisted-editing workflows from the project vision.
- This is a staged product target, not a requirement to ship every advanced command in one vertical slice. Each stage must list the implemented commands and deferred menu families explicitly.
- Every visible menu item, toolbar button, context action, shortcut and control maps to a typed application/domain command, a real view toggle, or a clearly disabled item with a specific reason. There are no decorative/fake buttons and no “coming soon” controls presented as actions.
- Commands that alter project content go through the command bus and participate in undo/redo where the operation is reversible. Platform operations (device setup, media decoding, file dialogs) stay in adapters/workers.
- Menu and shortcut registries are the single source for labels, enablement, key bindings, accessibility names and command dispatch. Context menus filter this registry by selection and track type; they do not create undocumented alternate mutations.

## Current implementation matrix (source review)

| Area | Exists in source | Missing for the approved workstation contract |
| --- | --- | --- |
| Project UI/IPC | Tauri new/demo/open/save/save-as, project summary, undo/redo, transport and persisted audio-profile settings. | Typed track CRUD, Scene/Slot CRUD, general menu/action registry, selection-aware context menus. |
| Project model | Current persisted JSON schema marker: `estudio-daw.project.v4` (internal data-format revision, not a product release); MIDI/Audio media kinds and MIDI/Instrument/Audio/Bus/Return/Master roles; input/output channel counts; shared track color and active/mute/solo/gain/pan state; MIDI and audio clips; audio source/provenance and non-destructive audio clip add/trim/gain/fades; MIDI take attach/quantize. `Track::new` and project validation enforce the data contract. v0/v1/v2/v3 migrate in memory; tests cover v1/v2 identity/order/content and v3→v4/default recovery. | Device-specific channel maps and route selection, runtime routing for bus/return/master, scenes/slots, robust source/playlist abstraction. |
| Project commands | Reversible typed add/rename/reorder/remove track commands and validated mixer-state assignment, plus audio clip and MIDI commands. Add uses model validation for role/media compatibility and channel/mixer bounds; project validation enforces unique track IDs and one master. Track removal also removes its owned regions while retaining external files. | Duplicate/activate controls, scenes/slots, routing, and audio region move/split command coverage. |
| Snapshot/UI | Track summaries carry shared mixer values and project color. Tauri can create MIDI and audio tracks through commands; arrangement labels distinguish audio tracks from MIDI note counts. | Audio clip summaries/waveform/source status, interactive shared mixer, input/output/meter state, shared Session/Arrangement playhead, editable MIDI/audio detail. |
| Runtime | PipeWire playback builds MIDI instrument sources and schedules MIDI events; play/pause/stop and duration-bounded MIDI clip scheduling exist. | Audio region decoding/playback/mixing, live per-track clip launch/scene quantization, recording/input monitoring, deterministic transport position reflected in Tauri. |
| Frontend | Project actions, MIDI Demo, MIDI playback transport, history, audio-profile controls, read-only MIDI Arrangement preview. | Real Session View (currently marked “PRONTO”), menu options/shortcuts, audio-track creation/import/playback, mixer, clip/note editing, user-media browser, a visually accepted Ableton/Ardour-informed layout. |

This matrix is based on `crates/ui-shell/src/main.rs`, `crates/ui-shell/src/audio_runtime.rs`, `crates/project-model/src/lib.rs`, `crates/command-bus/src/lib.rs`, and `crates/ui-shell/frontend/{index.html,main.js}`. It is source inspection only; no UI capability in the missing column is assumed complete from model metadata or an unconnected command.

## Goals / Non-Goals for this change

**Goals:**

- Deliver a real, coherent first workstation slice with Session and Arrangement surfaces over one ordered set of tracks.
- Establish explicit MIDI and audio track contracts, including the audio signal path, clip/media relationship, mixer state and supported actions.
- Make the primary visible menus and controls functional and command-backed; avoid claiming parity for deferred advanced features.
- Use a dense, readable instrument-oriented visual hierarchy: transport/control bar, categorized browser, track/scene grid or timeline, track headers/mixer and a contextual lower editor.
- Preserve existing project files through serde defaults/versioned migration and preserve non-destructive media provenance.

**Non-Goals for the first slice:**

- Implementing every effect, plugin format, routing graph, control surface, scripting API, analysis tool, or export preset found in the mature references.
- Reproducing Ableton or Ardour assets, exact artwork, or proprietary source code.
- Showing record-arm, input monitoring, device routing, plugin insertion, audio playback, or recording as functional before the backend path exists and is verified.

## Decisions

### One shared track model; two creative views

Session View is a matrix with track columns and scene rows. Arrangement View is a musical timeline with the same ordered tracks. Both read the same project entities and mixer state. Switching surface does not clone content or reset playback. Track creation, rename, reorder and deletion update both views through reversible commands.

Alternative considered: keep the current single timeline and add a Session label. Rejected because it is not the non-linear clip-launch workflow the user requested.

### Audio tracks model sources, regions, playlists, and signal flow

An audio track is a project entity with persistent name/order/channel configuration, active/mute/solo state, gain/pan, media source(s), and an ordered non-destructive clip/region list. A clip points to source media and stores source offset, timeline position, duration, fades and gain. The track signal path is input/source → processing chain → pan/gain → meter → output; each node/operation is implemented only when its processing/routing contract exists. MIDI/instrument tracks have MIDI input/clip/instrument/output identity, not an audio card painted differently.

The first implementation must be honest about the current lack of audio playback and recording. Audio import/placement/playback, recording/monitoring, routing/processing and buses are explicit vertical slices; do not expose controls before each slice works end to end.

### Menus describe product capabilities, not implementation decoration

The primary menu and view organization follows Ableton Live's File, Edit, Create, View, Options, Help and platform-specific Live/Options settings placement, plus the Control Bar, Browser, Session, Arrangement, Clip/Device View and Mixer. Estudio-specific technical actions such as Audio/MIDI setup may be added where needed. Ardour contributes technical audio concepts only; its menu layout and visual organization are not targets. A menu entry is shipped only alongside its command, enablement predicate, undo policy, keyboard shortcut when appropriate, status feedback and deterministic verification. The command catalog remains broader than the initial enabled menu and is staged in OpenSpec tasks.

### Mix controls are consistent in editor and mixer

Track gain, pan, mute, solo and active state are shared project state and displayed consistently in Session, Arrangement and Mixer. Record arm/input monitor only appear enabled when the recording path and selected hardware input are operational. Mixer strips communicate input, processing and output direction; meters report measured values, never simulated levels.

### Real-time clip launch is a runtime feature

Clip and scene launch use the session tempo/quantization, bounded control messages to an off-callback scheduler, and per-track playback state. The callback remains allocation-free and does not rebuild the device/render plan. MIDI events stop correctly on replacement/stop. Audio clips will join this scheduler only after a bounded decode/buffer path exists.

### Reference browser is useful but truthful

The browser offers real project tracks/clips and user-selected media locations with search, navigation history, filtering and audition when backed by the corresponding source/preview service. It will not display fictional factory sounds, licensed libraries or plug-ins. User-owned SoundFont/Analog Lab content is shown only through a real installed-device/media integration.

## Data migration

Additive fields use serde defaults and a versioned migration. Existing `TrackKind::Midi`/`Audio`, MIDI clips, audio clip provenance and current JSON projects remain readable. New scene identity/slots and mixer fields receive explicit defaults. A migration test must prove old fixtures open with stable track/clip identity/order and can still be saved.

## Risks / trade-offs

- **Wide reference feature set** → implement staged, useful slices; keep the full menu/capability inventory as a roadmap, never a fake full-parity claim.
- **Audio track semantics span many layers** → define the serializable media/region/playlist model and command contracts before UI polish.
- **Session launches require live scheduling** → isolate the launch controller from the PipeWire callback and avoid device restarts for clip changes.
- **Dense workstation surfaces can overwhelm** → keep selection-specific detail contextual and allow browser/mixer/detail visibility to be toggled, while retaining a clear visual hierarchy.
- **Generated browser entries can imply licenses/features** → list only installed, indexed and previewable user content with its provenance.

## Migration and verification plan

1. Reconcile these requirements/tasks with model and command bus; add versioned model fields and reversible commands.
2. Add application/Tauri operations and coherent runtime support for the actual vertical slice.
3. Build the Session, Arrangement, browser, contextual editor and mixer surfaces from snapshots and command results.
4. Verify all enabled menu/control paths, persistence/migration, undo/redo, musical placement and audio runtime behavior with deterministic checks. Run workspace checks only after the slice compiles; hardware checks are limited to capabilities that require actual devices.
5. Review a running UI at standard and minimum window sizes. Record exact completed tasks, evidence, known deferred menus and user-facing limitations. No task is complete on visual source review alone.

## Official reference sources

Reviewed 2026-09-27:

- Ableton Live 12: [Live concepts and control bar](https://www.ableton.com/en/manual/live-concepts/), [Session View](https://www.ableton.com/en/manual/session-view/), [Arrangement View](https://www.ableton.com/en/manual/arrangement-view/), [launching clips](https://www.ableton.com/en/manual/launching-clips/), [mixing](https://www.ableton.com/en/manual/mixing/), [browser](https://www.ableton.com/en/live-manual/12/working-with-the-browser/), [settings](https://www.ableton.com/en/live-manual/12/first-steps/).
- Ardour: [main-menu index](https://manual.ardour.org/ardours-interface/main-menu/), [Session menu](https://manual.ardour.org/ardours-interface/main-menu/Session-menu/), [Transport menu](https://manual.ardour.org/ardours-interface/main-menu/Transport-menu/), [Edit menu](https://manual.ardour.org/ardours-interface/main-menu/Edit-menu/), [Region menu](https://manual.ardour.org/ardours-interface/main-menu/Region-menu/), [Track menu](https://manual.ardour.org/ardours-interface/main-menu/Track-menu/), [View menu](https://manual.ardour.org/ardours-interface/main-menu/View-menu/), [Window menu](https://manual.ardour.org/ardours-interface/main-menu/Window-menu/), [audio track controls](https://manual.ardour.org/working-with-tracks/audio-track-controls/), [mixer strips](https://manual.ardour.org/ardours-interface/audio-midi-mixer-strips/), [sessions and tracks](https://manual.ardour.org/working-with-tracks/), [media import](https://manual.ardour.org/adding-pre-existing-material/), [regions](https://manual.ardour.org/working-with-regions/), [Cue workflow](https://manual.ardour.org/cue/).

The Ableton list reflects the Live 12 manual's application surfaces, menu/function domains and configuration pages; the Ardour list names every top-level main-menu family and groups its documented actions by functional family above. These are not yet a line-by-line catalog of every context-sensitive action across all devices, track types and selection states. Task 0.2 remains open for that catalog, so this planning document does not claim the menu audit is exhaustive or feature parity exists in the current build.
