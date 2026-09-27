# Spec Delta

## Purpose

Audio Runtime Control lets musicians tune device-period responsiveness and playback headroom from Estudio DAW. It provides distinct live/recording and multitrack playback starting profiles while reporting the values actually negotiated by the audio backend.

## ADDED Requirements

### Requirement: User can select and tune audio buffer profiles
The system MUST expose audio buffer settings from the DAW, including separate requested device period and playback safety-buffer values. It MUST provide editable Live/Record and Multitrack Playback profiles as starting points and retain the user's selected values for the active session.

#### Scenario: Choose a low-latency live profile
- **WHEN** the user selects Live/Record
- **THEN** the DAW requests the profile's device period and playback buffer settings and displays the requested values.

#### Scenario: Tune values directly
- **WHEN** the user changes the device period or playback safety buffer in audio settings
- **THEN** the active profile reflects those values and the DAW reports whether they are active or pending.

### Requirement: Report effective audio configuration
The system MUST distinguish requested settings from effective backend settings. It MUST show the effective sample rate and device period when available, plus the playback safety-buffer duration, in samples and milliseconds.

#### Scenario: Backend negotiates a different period
- **WHEN** PipeWire or another backend activates a period different from the requested period
- **THEN** the DAW shows both values and does not present the requested value as the achieved latency.

#### Scenario: Backend does not expose effective period
- **WHEN** the active backend cannot report its effective period
- **THEN** the DAW marks that value unavailable instead of estimating it as measured latency.

### Requirement: Separate live monitoring from playback headroom
The system MUST treat the device period and the DAW's additional playback safety buffer as separate controls. In multitrack playback, additional buffering MAY protect pre-rendered playback work, while the live monitored instrument/input path MUST use the configured low-latency path where supported. The DAW MUST identify any backend or engine limitation that prevents this separation.

#### Scenario: Multitrack playback with live input monitoring
- **WHEN** Multitrack Playback is active and a live input or instrument is monitored
- **THEN** playback safety buffering applies only to eligible playback work and the monitored path retains the configured device-period path where the engine supports independent scheduling.

#### Scenario: Current engine cannot isolate paths
- **WHEN** the engine cannot apply playback safety buffering independently from a monitored path
- **THEN** the DAW reports that limitation and does not claim that the playback buffer leaves monitoring latency unchanged.

### Requirement: Apply changes without corrupting audio or MIDI state
The system MUST apply supported buffer changes at a safe stream or block boundary, or clearly require a stream restart before activation. Applying settings MUST NOT drop, reorder, or alter MIDI events, including sustain pedal CC64, and MUST preserve active notes safely across the transition.

#### Scenario: Change profile while transport is active
- **WHEN** a profile change is requested during playback or recording
- **THEN** the DAW applies it at a supported safe boundary or reports that playback must stop/restart, without silently discarding queued MIDI events or recording data.

#### Scenario: Sustain pedal remains active through profile change
- **WHEN** the user holds the sustain pedal and changes the audio profile
- **THEN** CC64 state and subsequent Note Off behavior remain consistent with the MIDI stream before and after the change.

### Requirement: Explain buffer trade-offs and source
The system MUST describe device period as the backend/device scheduling block and playback safety buffering as additional DAW-managed buffering. It MUST NOT describe the latter as an AudioBox hardware cache or attribute Studio One Dropout Protection behavior to Estudio DAW.

#### Scenario: User inspects buffer settings
- **WHEN** the user opens audio latency settings
- **THEN** the DAW explains that smaller device periods can reduce buffering time while increasing scheduling/CPU pressure, and that additional playback buffering increases playback headroom and may add delay to the work it buffers.
