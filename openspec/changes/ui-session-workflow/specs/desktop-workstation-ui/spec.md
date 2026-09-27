# Spec Delta

## Purpose

Presenta el shell de escritorio como una superficie de trabajo musical compacta y legible, con controles y jerarquía visual propios de una DAW y coherentes con la referencia visual aprobada por el usuario.

## ADDED Requirements

### Requirement: La shell se presenta como una estación de trabajo de escritorio
La aplicación SHALL usar una composición compacta de escritorio, controles agrupados y superficies visuales de alto contraste que prioricen transporte, sesión y pistas sobre la presentación de tarjetas típica de un panel web.

#### Scenario: Abrir la interfaz de escritorio
- **WHEN** la shell Tauri inicia
- **THEN** el usuario ve una barra de acciones de proyecto, transporte, área de sesión y estado del motor en una jerarquía visual consistente con una DAW

#### Scenario: Referencia visual aprobada
- **WHEN** la interfaz aplica su tema de escritorio
- **THEN** utiliza una paleta oscura neutral y un acento cálido con densidad y jerarquía inspiradas en Ableton Live 12 Suite, sin reutilizar sus recursos gráficos propietarios

#### Scenario: Ajustar ventana estrecha
- **WHEN** el ancho de la ventana se reduce dentro del mínimo soportado
- **THEN** controles y preferencias siguen legibles y no se solapan
