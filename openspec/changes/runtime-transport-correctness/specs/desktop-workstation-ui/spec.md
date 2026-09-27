# desktop-workstation-ui Specification

## Purpose

Define la jerarquía de arreglo de la ventana Tauri del DAW y la presentación veraz de los flujos implementados.

## ADDED Requirements

### Requirement: Arrangement es el espacio de trabajo principal
La interfaz de escritorio Tauri MUST presentar el proyecto como una estación de trabajo de audio digital, con transporte, tempo, encabezados de pista, línea de tiempo musical y contenido visible de clips MIDI. La jerarquía visual MUST seguir la dirección aprobada, inspirada en Ableton Live 12: controles oscuros compactos, cuadrícula de arreglo de alto contraste y colores diferenciados para los clips.

#### Scenario: abrir un proyecto con clips MIDI
- **WHEN** se muestra un proyecto con pistas y clips MIDI
- **THEN** el arreglo ocupa el espacio de trabajo principal y muestra las posiciones de pistas y clips, además de vistas previas de notas
- **AND** las acciones existentes del proyecto y del transporte siguen siendo fáciles de encontrar

### Requirement: la interfaz comunica las capacidades implementadas
La interfaz MUST vincular los controles sólo a comandos implementados y MUST describir la reproducción, grabación, edición y configuración de audio según su estado real de implementación.

#### Scenario: sesión vacía o experimental
- **WHEN** el usuario abre un proyecto vacío o un flujo de trabajo no admitido
- **THEN** la interfaz indica claramente el siguiente paso y no sugiere que la edición MIDI, la grabación en vivo o la grabación de audio estén disponibles cuando no lo están
