# Spec Delta

## Purpose

El control del entorno de audio permite que los músicos ajusten desde Estudio DAW la respuesta del período del dispositivo y el margen de reproducción. Ofrece perfiles iniciales separados para interpretación/grabación y reproducción multipista e informa los valores realmente negociados por el motor de audio.

## ADDED Requirements

### Requirement: el usuario puede elegir y ajustar perfiles de búfer de audio
El sistema MUST ofrecer ajustes de búfer de audio desde el DAW, incluidos valores independientes para el período solicitado al dispositivo y el búfer de seguridad de reproducción. MUST ofrecer perfiles editables de interpretación/grabación y reproducción multipista como puntos de partida, y conservar los valores elegidos por el usuario durante la sesión activa.

#### Scenario: elegir un perfil de baja latencia para interpretación
- **WHEN** el usuario selecciona En vivo/Grabación
- **THEN** la DAW solicita los valores de período del dispositivo y búfer de reproducción del perfil, y muestra los valores solicitados.

#### Scenario: ajustar valores directamente
- **WHEN** el usuario cambia el período del dispositivo o el búfer de seguridad de reproducción en los ajustes de audio
- **THEN** el perfil activo refleja esos valores y la DAW informa si están activos o pendientes.

### Requirement: informar la configuración efectiva de audio
El sistema MUST distinguir los ajustes solicitados de los valores efectivos del motor de plataforma. MUST mostrar la frecuencia de muestreo y el período efectivo del dispositivo cuando estén disponibles, además de la duración del búfer de seguridad de reproducción en muestras y milisegundos.

#### Scenario: el motor de plataforma negocia un período distinto
- **WHEN** PipeWire u otro motor activa un período distinto del solicitado
- **THEN** la DAW muestra ambos valores y no presenta el valor solicitado como la latencia alcanzada.

#### Scenario: el motor de plataforma no informa el período efectivo
- **WHEN** el motor activo no puede informar su período efectivo
- **THEN** la DAW marca ese valor como no disponible en vez de estimarlo como latencia medida.

### Requirement: separar la monitorización en vivo del margen de reproducción
El sistema MUST tratar el período del dispositivo y el búfer adicional de seguridad de reproducción del DAW como controles separados. Durante la reproducción multipista, el búfer adicional MAY proteger el trabajo de reproducción prerenderizado, mientras que la ruta de interpretación/entrada con monitorización MUST usar la ruta de baja latencia configurada cuando exista soporte. El DAW MUST identificar cualquier limitación del motor de plataforma o del motor de audio que impida separar ambas rutas.

#### Scenario: reproducción multipista con monitorización de entrada en vivo
- **WHEN** está activa la reproducción multipista y se monitoriza una entrada o instrumento en vivo
- **THEN** el búfer de seguridad se aplica sólo a la reproducción que corresponda, y la ruta monitorizada conserva el período configurado del dispositivo cuando el motor admite planificación independiente.

#### Scenario: el motor actual no puede aislar las rutas
- **WHEN** el motor no puede aplicar el búfer de seguridad de reproducción de forma independiente de una ruta monitorizada
- **THEN** la DAW informa esa limitación y no afirma que el búfer de reproducción deje intacta la latencia de monitorización.

### Requirement: aplicar cambios sin corromper el estado de audio o MIDI
El sistema MUST aplicar los cambios de búfer admitidos en un límite seguro de flujo o bloque, o indicar claramente que es necesario reiniciar el flujo antes de activarlos. La aplicación de ajustes MUST NOT descartar, reordenar ni alterar eventos MIDI, incluido el pedal de sustain CC64, y MUST conservar de forma segura las notas activas durante la transición.

#### Scenario: cambiar el perfil durante el transporte
- **WHEN** se solicita cambiar el perfil durante la reproducción o grabación
- **THEN** la DAW aplica el cambio en un límite seguro admitido o informa que hay que detener y reiniciar la reproducción, sin descartar silenciosamente eventos MIDI en cola ni datos grabados.

#### Scenario: el pedal sustain permanece activo durante el cambio de perfil
- **WHEN** el usuario mantiene pisado el pedal de sustain y cambia el perfil de audio
- **THEN** el estado CC64 y el comportamiento posterior de Note Off siguen siendo coherentes con el flujo MIDI antes y después del cambio.

### Requirement: explicar las compensaciones del búfer y sus causas
El sistema MUST describir el período del dispositivo como el bloque de planificación del motor de plataforma/dispositivo y el búfer de seguridad de reproducción como un búfer adicional administrado por el DAW. MUST NOT describir este último como una caché de hardware de AudioBox ni atribuir a Estudio DAW el comportamiento de protección contra interrupciones de Studio One.

#### Scenario: el usuario consulta los ajustes del búfer
- **WHEN** el usuario abre los ajustes de latencia de audio
- **THEN** la DAW explica que reducir el período del dispositivo puede acortar el tiempo de búfer y aumentar la carga de planificación y CPU, y que añadir búfer a la reproducción da más margen y puede retrasar el trabajo que almacena.
