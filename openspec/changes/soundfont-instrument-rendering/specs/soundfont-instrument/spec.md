# Spec Delta

## Purpose

Provides a local sample-based instrument for practicing and producing with the existing MIDI and audio workflow. It uses user-owned SoundFont assets and keeps audio rendering isolated from control-plane and file-loading work.

## ADDED Requirements

### Requirement: el usuario puede elegir un instrumento SoundFont local
El sistema MUST permitir que el usuario seleccione un archivo SoundFont local y un preajuste de banco/programa disponible para un instrumento del proyecto. La configuración del instrumento MUST poder serializarse y MUST referenciar el archivo SoundFont sin copiarlo.

#### Scenario: elegir un preajuste disponible
- **WHEN** the user selects a readable SoundFont and a valid preset
- **THEN** the system loads it outside the audio callback and reports the selected preset as ready

#### Scenario: SoundFont no está disponible
- **WHEN** a project references a missing or unreadable SoundFont
- **THEN** the system reports the path and actionable error, leaves the project recoverable, and does not silently substitute another instrument

### Requirement: los instrumentos SoundFont responden a MIDI
El instrumento MUST responder a los eventos de activación/desactivación de nota, velocidad y selección de programa admitidos por el SoundFont elegido. Las tomas MIDI y sus eventos MUST permanecer independientes del generador de sonido seleccionado.

#### Scenario: activar y soltar una nota
- **WHEN** the instrument receives note-on followed by note-off
- **THEN** the selected preset renders the note with its SoundFont timbre and releases it according to the preset envelope

#### Scenario: cambiar de instrumento sin alterar la toma
- **WHEN** the user changes a track's instrument from the sine test source to a SoundFont preset
- **THEN** the MIDI event data remains unchanged and can be replayed through either available instrument

### Requirement: el audio en tiempo real permanece aislado del trabajo SoundFont
La carga de SoundFont, el acceso al sistema de archivos y el control de síntesis MUST NOT bloquear, asignar memoria ni realizar operaciones de entrada/salida dentro de la devolución de audio de PipeWire. Si el audio renderizado no está disponible temporalmente, la devolución MUST continuar con silencio en los cuadros afectados y exponer un diagnóstico de interrupción, en vez de esperar al proceso del instrumento.

#### Scenario: el proceso de trabajo incumple un plazo de audio
- **WHEN** the instrument worker has not published audio for a callback block
- **THEN** the callback emits silence for that block, increments an observable underrun counter, and continues processing the rest of the graph

#### Scenario: cargar o reemplazar un SoundFont durante la sesión
- **WHEN** a user requests a SoundFont load or replacement
- **THEN** the operation occurs outside the PipeWire callback and the currently active render plan remains valid until the replacement is ready

#### Scenario: una sustitución fallida conserva el instrumento actual
- **WHEN** preparation of a replacement SoundFont or render plan fails
- **THEN** the replacement is not published, the active plan and its worker remain playable, and retired plans/resources are destroyed only by the non-real-time control thread

### Requirement: las dependencias y los archivos del instrumento permanecen locales y auditables
La función SoundFont MUST funcionar sin conexión de red ni API pagada. La aplicación MUST NOT incluir ni redistribuir de forma predeterminada los SoundFont del usuario, MUST identificar el entorno de síntesis requerido cuando no esté disponible y MUST documentar los avisos de entornos de terceros por separado de las licencias de los archivos SoundFont.

#### Scenario: iniciar el instrumento sin conexión
- **WHEN** the required local synthesis runtime and SoundFont are present but the network is unavailable
- **THEN** the instrument can load and play without contacting an external service

#### Scenario: biblioteca de ejecución no disponible
- **WHEN** the user chooses the SoundFont backend but its runtime library is not installed
- **THEN** the system gives an installation/runtime diagnostic and leaves the sine test instrument available as an explicit alternative
