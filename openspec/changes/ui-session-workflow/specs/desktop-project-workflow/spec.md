# Spec Delta

## Purpose

Permite comenzar una sesión musical nueva desde la aplicación de escritorio y usar sus controles de transporte sin tener que localizar primero un archivo existente.

## ADDED Requirements

### Requirement: Crear un proyecto nuevo desde la interfaz
La aplicación SHALL ofrecer una acción visible para crear una sesión de proyecto nueva con valores iniciales válidos y una pista MIDI vacía.

#### Scenario: Crear sesión desde el estado inicial
- **WHEN** el usuario selecciona Nuevo proyecto sin una sesión activa
- **THEN** la aplicación crea una sesión válida, muestra su nombre y pista MIDI, y habilita las acciones de guardar y transporte

#### Scenario: Reemplazar la sesión activa
- **WHEN** el usuario selecciona Nuevo proyecto mientras hay otra sesión cargada
- **THEN** la aplicación detiene la reproducción activa antes de cambiar a la nueva sesión

### Requirement: Transporte disponible con una sesión
La aplicación SHALL habilitar los controles Play, Pause y Stop cuando exista una sesión de proyecto cargada, y SHALL mantenerlos deshabilitados cuando no haya ninguna sesión.

#### Scenario: Crear proyecto activa transporte
- **WHEN** una sesión nueva se crea correctamente
- **THEN** Play, Pause y Stop quedan disponibles para esa sesión

#### Scenario: Proyecto vacío
- **WHEN** el usuario pulsa Play en una sesión sin clips MIDI reproducibles
- **THEN** la aplicación mantiene el transporte operativo y no presenta la sesión vacía como si contuviera audio


### Requirement: Sesión demo reproducible
La aplicación SHALL ofrecer un proyecto de demostración con un clip MIDI y notas que permitan verificar audiblemente la salida desde el transporte.

#### Scenario: Crear demo y escucharla
- **WHEN** el usuario selecciona Demo MIDI y luego Play
- **THEN** la sesión contiene una secuencia corta visible en la pista MIDI y el motor reproduce las notas mediante el instrumento configurado
