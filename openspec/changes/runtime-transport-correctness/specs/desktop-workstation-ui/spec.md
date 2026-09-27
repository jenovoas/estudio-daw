# desktop-workstation-ui Specification

## ADDED Requirements

### Requirement: Arrangement is the primary workspace
The Tauri desktop UI MUST present the project as a DAW workspace, with transport, tempo, track headers, a musical timeline, and visible MIDI clip content. Visual hierarchy MUST follow the approved Ableton Live 12 inspired direction: compact dark chrome, high-contrast arrangement grid, and distinct clip colors.

#### Scenario: Open a project with MIDI clips
- **WHEN** a project containing MIDI tracks and clips is shown
- **THEN** the arrangement occupies the primary workspace and exposes track/clip positions and note previews
- **AND** existing project and transport controls remain discoverable

### Requirement: UI communicates implemented capabilities
The interface MUST bind controls only to implemented commands and MUST describe playback, recording, editing, and audio configuration according to their actual implementation state.

#### Scenario: Empty or experimental session
- **WHEN** the user opens an empty project or an unsupported workflow
- **THEN** the UI gives a clear next action and does not imply that MIDI editing, live recording, or audio recording is available when it is not
