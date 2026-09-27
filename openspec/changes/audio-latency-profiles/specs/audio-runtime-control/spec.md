# Spec Delta

## Purpose

El control del entorno de audio permite que los músicos ajusten desde Estudio DAW la respuesta del período del dispositivo y el margen de reproducción. Ofrece perfiles iniciales separados para interpretación/grabación y reproducción multipista e informa los valores realmente negociados por el motor de audio.

## ADDED Requirements

### Requirement: el usuario puede elegir y ajustar perfiles de búfer de audio
El sistema MUST ofrecer ajustes de búfer de audio desde el DAW, incluidos valores independientes para el período solicitado al dispositivo y el búfer de seguridad de reproducción. MUST ofrecer perfiles editables de interpretación/grabación y reproducción multipista como puntos de partida, y conservar los valores elegidos por el usuario durante la sesión activa.

#### Scenario: elegir un perfil de baja latencia para interpretación
- **WHEN** the user selects Live/Record
- **THEN** the DAW requests the profile's device period and playback buffer settings and displays the requested values.

#### Scenario: ajustar valores directamente
- **WHEN** the user changes the device period or playback safety buffer in audio settings
- **THEN** the active profile reflects those values and the DAW reports whether they are active or pending.

### Requirement: informar la configuración efectiva de audio
El sistema MUST distinguir los ajustes solicitados de los valores efectivos del motor de plataforma. MUST mostrar la frecuencia de muestreo y el período efectivo del dispositivo cuando estén disponibles, además de la duración del búfer de seguridad de reproducción en muestras y milisegundos.

#### Scenario: el motor de plataforma negocia un período distinto
- **WHEN** PipeWire or another backend activates a period different from the requested period
- **THEN** the DAW shows both values and does not present the requested value as the achieved latency.

#### Scenario: el motor de plataforma no informa el período efectivo
- **WHEN** the active backend cannot report its effective period
- **THEN** the DAW marks that value unavailable instead of estimating it as measured latency.

### Requirement: separar la monitorización en vivo del margen de reproducción
El sistema MUST tratar el período del dispositivo y el búfer adicional de seguridad de reproducción del DAW como controles separados. Durante la reproducción multipista, el búfer adicional MAY proteger el trabajo de reproducción prerenderizado, mientras que la ruta de interpretación/entrada con monitorización MUST usar la ruta de baja latencia configurada cuando exista soporte. El DAW MUST identificar cualquier limitación del motor de plataforma o del motor de audio que impida separar ambas rutas.

#### Scenario: reproducción multipista con monitorización de entrada en vivo
- **WHEN** Multitrack Playback is active and a live input or instrument is monitored
- **THEN** playback safety buffering applies only to eligible playback work and the monitored path retains the configured device-period path where the engine supports independent scheduling.

#### Scenario: el motor actual no puede aislar las rutas
- **WHEN** the engine cannot apply playback safety buffering independently from a monitored path
- **THEN** the DAW reports that limitation and does not claim that the playback buffer leaves monitoring latency unchanged.

### Requirement: aplicar cambios sin corromper el estado de audio o MIDI
El sistema MUST aplicar los cambios de búfer admitidos en un límite seguro de flujo o bloque, o indicar claramente que es necesario reiniciar el flujo antes de activarlos. La aplicación de ajustes MUST NOT descartar, reordenar ni alterar eventos MIDI, incluido el pedal de sustain CC64, y MUST conservar de forma segura las notas activas durante la transición.

#### Scenario: cambiar el perfil durante el transporte
- **WHEN** a profile change is requested during playback or recording
- **THEN** the DAW applies it at a supported safe boundary or reports that playback must stop/restart, without silently discarding queued MIDI events or recording data.

#### Scenario: el pedal sustain permanece activo durante el cambio de perfil
- **WHEN** the user holds the sustain pedal and changes the audio profile
- **THEN** CC64 state and subsequent Note Off behavior remain consistent with the MIDI stream before and after the change.

### Requirement: explicar las compensaciones del búfer y sus causas
El sistema MUST describir el período del dispositivo como el bloque de planificación del motor de plataforma/dispositivo y el búfer de seguridad de reproducción como un búfer adicional administrado por el DAW. MUST NOT describir este último como una caché de hardware de AudioBox ni atribuir a Estudio DAW el comportamiento de protección contra interrupciones de Studio One.

#### Scenario: el usuario consulta los ajustes del búfer
- **WHEN** the user opens audio latency settings
- **THEN** the DAW explains that smaller device periods can reduce buffering time while increasing scheduling/CPU pressure, and that additional playback buffering increases playback headroom and may add delay to the work it buffers.
