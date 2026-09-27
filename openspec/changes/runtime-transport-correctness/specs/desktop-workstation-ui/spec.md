# desktop-workstation-ui Specification

## Purpose

Define la jerarquía de arreglo de la ventana Tauri del DAW y la presentación veraz de los flujos implementados.

## ADDED Requirements

### Requirement: Arrangement es el espacio de trabajo principal
La interfaz de escritorio Tauri MUST presentar el proyecto como una estación de trabajo de audio digital, con transporte, tempo, encabezados de pista, línea de tiempo musical y contenido visible de clips MIDI. La jerarquía visual MUST seguir la dirección aprobada, inspirada en Ableton Live 12: controles oscuros compactos, cuadrícula de arreglo de alto contraste y colores diferenciados para los clips.

#### Scenario: abrir un proyecto con clips MIDI
- **WHEN** a project containing MIDI tracks and clips is shown
- **THEN** the arrangement occupies the primary workspace and exposes track/clip positions and note previews
- **AND** existing project and transport controls remain discoverable

### Requirement: la interfaz comunica las capacidades implementadas
La interfaz MUST vincular los controles sólo a comandos implementados y MUST describir la reproducción, grabación, edición y configuración de audio según su estado real de implementación.

#### Scenario: sesión vacía o experimental
- **WHEN** the user opens an empty project or an unsupported workflow
- **THEN** the UI gives a clear next action and does not imply that MIDI editing, live recording, or audio recording is available when it is not
