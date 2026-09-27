# runtime-midi-playback Specification

## ADDED Requirements

### Requirement: Pause freezes rendered instrument state
When desktop playback is paused, the audio callback MUST output silence without advancing render-plan node state or consuming queued instrument PCM. SoundFont workers MUST stop rendering further PCM and advancing their instrument voices while paused. The MIDI scheduler MUST preserve event timing relative to the resumed transport.

#### Scenario: Resume after pause
- **WHEN** playback is paused while a MIDI note or queued instrument audio is active
- **THEN** output is silent and render-plan/FluidSynth voices do not advance during the pause
- **AND** the existing bounded PCM queue remains stationary
- **AND** scheduled events continue from the same musical position after resume

### Requirement: Playback respects MIDI clip bounds
The scheduler MUST place each eligible take event at the clip's `start_tick` plus its local event tick, and MUST exclude events whose local tick is greater than `duration_ticks`.

#### Scenario: Clip start and end
- **WHEN** a MIDI clip has a nonzero start and a bounded duration
- **THEN** events before or at the duration boundary are scheduled at the clip-offset time
- **AND** events after the duration boundary are not scheduled

### Requirement: Realtime playback fails safely
The audio callback MUST NOT panic if a render-plan slot is unexpectedly empty. SineSynth MUST safely handle every incoming MIDI note byte without indexing outside its 128-key frequency table.

#### Scenario: Invalid runtime state or pitch
- **WHEN** an active plan slot is empty
- **THEN** the callback returns silence without unwinding
- **WHEN** SineSynth receives a note above 127
- **THEN** it renders a bounded valid pitch without panicking

### Requirement: Device startup and domain transport remain consistent
The desktop adapter MUST not leave the session in Play when PipeWire startup fails.

#### Scenario: Audio device startup fails
- **WHEN** the session accepts Play but the audio backend cannot start
- **THEN** the adapter sends Stop to the domain session and returns the backend error
- **AND** it does not report a connected playback stream
