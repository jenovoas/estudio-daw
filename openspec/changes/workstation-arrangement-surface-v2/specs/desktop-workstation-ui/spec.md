# Spec Delta

## ADDED Requirements

### Requirement: A workstation menu exposes the complete product command map
The workstation MUST organize its supported actions into discoverable Project/Session, Edit, Create, View/Window, Track, Clip/Region, Transport, Audio/MIDI, Settings/Options and Help menu families, with contextual menus and keyboard shortcuts for high-frequency workflows. The audited reference command families in `../../design.md` are the minimum product inventory; the product roadmap MUST account for each family and may extend it with Estudio DAW's music-analysis, notation, coding and assisted-editing capabilities. A menu item MUST map to a typed command, real view/state action or external platform adapter. Each item MUST have a correct enablement condition, undo policy where it edits the project, accessible label, and truthful completion/error feedback. An unimplemented action MUST NOT be presented as enabled or complete.

#### Scenario: Inspect the product menus
- **WHEN** the user opens a main or context menu
- **THEN** supported commands are grouped by the relevant project, editing, track, clip, transport, view or device workflow
- **AND** commands disabled by current selection/project/device state explain the concrete reason
- **AND** no decorative control appears to invoke a capability that does not exist

#### Scenario: Trigger a menu action and its shortcut
- **WHEN** the user chooses an enabled menu item or its displayed shortcut
- **THEN** both dispatch the same typed operation
- **AND** the operation result, project state and undo/redo state agree

### Requirement: Audio tracks are complete first-class project entities
The project MUST model audio tracks separately from MIDI/instrument tracks, with stable identity/order, name/color, channel configuration, active/mute/solo, gain/pan, source and output routing state, and an ordered non-destructive playlist of audio regions. Regions MUST reference source media and preserve source offset, timeline position, duration, fades and region gain. Removing/moving/trimming a region MUST NOT silently delete or alter its underlying source file. Session, Arrangement and Mixer MUST render the same track identity and state. The UI MUST NOT label a track “audio” solely for visual appearance when its import/playback path is absent.

#### Scenario: Create and manage an audio track
- **WHEN** the user creates, names, reorders, duplicates, mutes, solos, activates or removes an audio track
- **THEN** the project command bus validates the operation and makes it undoable/redoable
- **AND** the same track order and state appear in Session, Arrangement and Mixer
- **AND** incompatible MIDI-only content cannot be attached to that track

#### Scenario: Import and edit audio media
- **WHEN** the user imports audio with a selected copy/link and channel mapping option
- **THEN** the source provenance and technical metadata are retained
- **AND** a non-destructive region appears at the selected musical/timeline position with a real waveform and editable duration/source offset
- **AND** undoing the region edit leaves the source file intact

#### Scenario: Play and route an audio track
- **WHEN** the transport plays a project containing imported audio regions
- **THEN** each region renders only within its timeline duration using its source offset, fades and gain, and reaches the selected real output through the track/mixer path
- **AND** the callback remains bounded and allocation-free
- **AND** selecting an input, arming or monitoring a track is enabled only when the corresponding capture/device path works

### Requirement: Session and Arrangement are linked views over shared tracks
The workstation MUST provide a clip/scene Session View and a timeline Arrangement View over the same ordered project tracks, media, mixer state and transport. Session organizes track columns against scene rows; Arrangement organizes the same tracks vertically against musical time. View switching MUST preserve project content and active playback state. Ableton Live 12 MUST be the primary visual and interaction reference for the creative information hierarchy, density and relationship between these views. Ardour may be consulted only for technical audio-engineering details; it MUST NOT define the workstation's visual hierarchy or creative flow. Styling alone does not satisfy this requirement.

#### Scenario: Open a project in Session View
- **WHEN** a project is shown in the primary creative view
- **THEN** track columns and scene rows display typed clip slots, playing/queued state and scene launch controls
- **AND** a slot offers actions appropriate to its track and clip type
- **AND** empty slots provide real create/import actions or a genuine empty state

#### Scenario: Switch between Session and Arrangement
- **WHEN** the user switches view during a project
- **THEN** the same tracks, clips, source references, track controls and playhead are represented
- **AND** playback does not stop or restart merely because the view changed

#### Scenario: Inspect audio and MIDI in Arrangement
- **WHEN** an audio or MIDI track has regions
- **THEN** Arrangement shows real audio waveform or MIDI-note content on that track at musical positions
- **AND** selection, move, trim, split, duplicate, loop and snap operations update project state through commands and can be undone

### Requirement: Session slots and scenes perform real playback
Session clip/scene launch MUST share the project transport clock, tempo and musical meter. Launch quantization, per-track replacement/stop and scene-wide launch MUST be scheduled outside the audio callback with bounded control messages; switching a slot MUST NOT rebuild or restart the audio device. MIDI notes and audio regions MUST stop/release correctly at clip end, replacement, pause and transport stop. Clip launch settings such as quantization, loop/launch mode and follow actions are exposed only when their exact behavior is implemented.

#### Scenario: Launch a MIDI or audio clip
- **WHEN** the user launches a supported clip with a quantization setting
- **THEN** it starts on the expected transport boundary and loops/ends according to persisted clip duration and launch settings
- **AND** replacing it stops/releases that track's previous clip without affecting other tracks or reopening the device

#### Scenario: Launch a scene
- **WHEN** the user launches a scene
- **THEN** compatible populated slots start together on the shared quantization boundary
- **AND** empty slots follow their explicit stop/continue behavior rather than launching fabricated content

### Requirement: Track headers and mixer strips represent real signal flow
Audio and MIDI/instrument track headers MUST identify track type and provide only implemented controls. The mixer and track controls MUST follow Live's shared, configurable Session/Arrangement model and show actual values and activity, not simulated controls or meters. For audio-track technical correctness, input, processor chain, pan/gain, meter and output MUST form a legible signal path, cross-checked against the Ardour audio-strip description where useful. Track gain/pan/mute/solo/active values MUST stay consistent across all views. Recording arm, input monitoring, routing, sends, buses and processors MUST be enabled only when their complete backend path is available.

#### Scenario: Change shared mixer state
- **WHEN** the user changes gain, pan, mute, solo or active state from a supported view
- **THEN** the value persists and immediately agrees in Session, Arrangement and Mixer
- **AND** its project command participates in undo/redo

#### Scenario: Open a track context menu
- **WHEN** the user opens a context menu on a MIDI, audio, bus or master track
- **THEN** it contains the applicable create/rename/reorder/edit/mix actions for that track type
- **AND** unsupported route, record or processor actions are absent or disabled with a reason

### Requirement: Browser and lower editor are contextual creative tools
The browser MUST provide searchable, navigable project/user media and installed instruments/devices, metadata filters, preview and drag/drop only where the backend supports them. It MUST preserve provenance and never claim access to content based only on its visual label. A lower contextual panel MUST change with selection, providing a MIDI note editor for MIDI clips, audio region/source controls for audio clips, or real parameters for loaded devices.

#### Scenario: Browse and preview media
- **WHEN** the user searches or previews a library item or audio file
- **THEN** results come from indexed project/user/installed content with type and provenance
- **AND** preview is audible and distinct from the session transport when shown as available
- **AND** dropping a result into a track creates a compatible project entity through the application command path

### Requirement: Transport and settings report actual engine state
The transport MUST expose play/pause/stop, musical position, tempo/meter and the implemented loop/click/navigation/recording operations. Position and playback state MUST come from the session transport clock and runtime events. Audio settings MUST distinguish persisted preferences from settings currently applied to the active stream, including whether restart is required. The UI MUST report device errors instead of showing a successful state when domain command or runtime startup fails.

#### Scenario: Pause and resume
- **WHEN** the user pauses during playback and later resumes
- **THEN** transport position, event scheduler, voices and audio consumption remain aligned
- **AND** elapsed wall-clock time while paused does not advance musical content

### Requirement: UI fidelity is verified as a music-production workflow
The UI MUST use a workstation hierarchy with an immediately usable creative surface, clear track/scene organization, readable audio waveform and MIDI note content, persistent but compact transport, contextual browser/editor and integrated mixer. Visual validation MUST inspect a running window at standard and minimum supported sizes and exercise the primary workflows; color tokens, screenshots of source code or a build alone do not establish product fidelity.

#### Scenario: Review a standard session
- **WHEN** a loaded mixed MIDI/audio project is reviewed at the supported desktop size
- **THEN** Session, Arrangement, Browser, track controls and Mixer read as one connected workstation
- **AND** the musical surfaces dominate available space instead of dashboard cards or generic web-page sections

#### Scenario: Review minimum window size
- **WHEN** the window is at its minimum supported dimensions
- **THEN** transport, track identity, selected view and essential editing/launch actions remain reachable without overlap
- **AND** optional panels collapse or scroll without obscuring the active track surface
