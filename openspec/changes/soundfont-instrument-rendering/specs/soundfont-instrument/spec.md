# Spec Delta

## Purpose

Provides a local sample-based instrument for practicing and producing with the existing MIDI and audio workflow. It uses user-owned SoundFont assets and keeps audio rendering isolated from control-plane and file-loading work.

## ADDED Requirements

### Requirement: User can select a local SoundFont instrument
The system MUST allow a user to select a local SoundFont file and an available bank/program preset for a project instrument. The instrument configuration MUST be serializable and MUST reference, not copy, the SoundFont asset.

#### Scenario: Select an available preset
- **WHEN** the user selects a readable SoundFont and a valid preset
- **THEN** the system loads it outside the audio callback and reports the selected preset as ready

#### Scenario: SoundFont is unavailable
- **WHEN** a project references a missing or unreadable SoundFont
- **THEN** the system reports the path and actionable error, leaves the project recoverable, and does not silently substitute another instrument

### Requirement: SoundFont instruments respond to MIDI
The instrument MUST respond to note-on, note-off, velocity, and program selection events supported by the selected SoundFont. MIDI takes and their events MUST remain independent of the chosen sound generator.

#### Scenario: Play and release a note
- **WHEN** the instrument receives note-on followed by note-off
- **THEN** the selected preset renders the note with its SoundFont timbre and releases it according to the preset envelope

#### Scenario: Switch instrument without changing a take
- **WHEN** the user changes a track's instrument from the sine test source to a SoundFont preset
- **THEN** the MIDI event data remains unchanged and can be replayed through either available instrument

### Requirement: Real-time audio remains isolated from SoundFont work
SoundFont loading, filesystem access, and synthesis control MUST NOT block, allocate, or perform I/O in the PipeWire audio callback. If rendered audio is temporarily unavailable, the callback MUST continue with silence for the affected frames and expose an underrun diagnostic rather than waiting for the instrument worker.

#### Scenario: Worker misses an audio deadline
- **WHEN** the instrument worker has not published audio for a callback block
- **THEN** the callback emits silence for that block, increments an observable underrun counter, and continues processing the rest of the graph

#### Scenario: Load or replace a SoundFont during a session
- **WHEN** a user requests a SoundFont load or replacement
- **THEN** the operation occurs outside the PipeWire callback and the currently active render plan remains valid until the replacement is ready

#### Scenario: Failed replacement preserves the current instrument
- **WHEN** preparation of a replacement SoundFont or render plan fails
- **THEN** the replacement is not published, the active plan remains playable, and retired plans are destroyed only by the non-real-time control thread

### Requirement: Instrument dependencies and assets stay local and auditable
The SoundFont capability MUST work without a network or paid API. The application MUST NOT bundle or redistribute user SoundFonts by default, MUST identify the required synthesis runtime when unavailable, and MUST document third-party runtime notices separately from SoundFont asset licenses.

#### Scenario: Offline instrument startup
- **WHEN** the required local synthesis runtime and SoundFont are present but the network is unavailable
- **THEN** the instrument can load and play without contacting an external service

#### Scenario: Runtime library unavailable
- **WHEN** the user chooses the SoundFont backend but its runtime library is not installed
- **THEN** the system gives an installation/runtime diagnostic and leaves the sine test instrument available as an explicit alternative
