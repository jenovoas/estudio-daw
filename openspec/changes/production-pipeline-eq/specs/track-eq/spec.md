# Spec Delta

## Purpose

Ecualizador paramétrico por pista: permite al productor moldear el espectro de
frecuencias de cada pista de audio o MIDI de forma no destructiva y en tiempo real
durante la reproducción.

## ADDED Requirements

### Requirement: Configuración de bandas de EQ por pista

Cada pista del proyecto **MUST** permitir entre 0 y 8 bandas de EQ paramétricas. La
configuración **MUST** persistirse en el modelo de proyecto y sobrevivir a guardar y recargar.

#### Scenario: Añadir una banda de EQ a una pista

- **WHEN** el usuario configura una o más bandas de EQ en una pista mediante el comando `set_track_eq`
- **THEN** las bandas **MUST** quedar persistidas en el proyecto con su tipo de filtro, frecuencia, ganancia y Q
- **THEN** el snapshot devuelto **MUST** reflejar las bandas configuradas en la pista

#### Scenario: Pista sin bandas de EQ

- **WHEN** una pista no tiene bandas configuradas (lista vacía)
- **THEN** el audio de esa pista **MUST** pasar sin ningún procesado de EQ

#### Scenario: Banda desactivada

- **WHEN** una banda tiene `enabled: false`
- **THEN** esa banda **MUST NOT** procesar el audio (bypass) pero su configuración **MUST** permanecer en el proyecto para poder reactivarla

### Requirement: Tipos de filtro soportados

El EQ **MUST** soportar los siguientes tipos de filtro por banda, cada uno con sus parámetros relevantes:

| Tipo       | Frecuencia | Ganancia dB | Q   |
|------------|-----------|-------------|-----|
| Bell       | ✓         | ✓           | ✓   |
| LowShelf   | ✓         | ✓           | ✓   |
| HighShelf  | ✓         | ✓           | ✓   |
| LowPass    | ✓         | —           | ✓   |
| HighPass   | ✓         | —           | ✓   |
| Notch      | ✓         | —           | ✓   |

#### Scenario: Tipo de filtro inválido

- **WHEN** se envía un tipo de filtro no reconocido en `set_track_eq`
- **THEN** el comando **MUST** devolver un error descriptivo y el proyecto **MUST NOT** modificarse

### Requirement: Procesado de audio en tiempo real

Cuando el motor de audio está reproduciendo, el EQ de cada pista **MUST** aplicarse
en tiempo real al buffer de audio de esa pista, antes del gain/pan del mezclador.

#### Scenario: EQ aplicado durante reproducción

- **WHEN** el motor está en play y una pista tiene bandas de EQ activas
- **THEN** el audio que sale de esa pista **MUST** tener el espectro de frecuencias modificado según las bandas configuradas

#### Scenario: Cambio de EQ durante reproducción

- **WHEN** el usuario modifica las bandas de EQ de una pista mientras el motor está reproduciendo
- **THEN** el nuevo plan de renderizado **MUST** publicarse en el motor sin interrumpir la reproducción

### Requirement: Panel de EQ en la UI

El Device Rack del panel inferior **MUST** exponer los controles de EQ de la pista enfocada.

#### Scenario: Pista sin EQ configurado

- **WHEN** la pista enfocada no tiene bandas de EQ
- **THEN** el panel **MUST** mostrar un estado vacío con opción de añadir la primera banda

#### Scenario: Estado del panel refleja el snapshot

- **WHEN** el snapshot llega con bandas de EQ en una pista
- **THEN** los controles del panel **MUST** mostrar los valores reales del snapshot, no valores locales especulativos
