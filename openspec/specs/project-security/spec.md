# project-security Specification

## Purpose

Define cómo Estudio DAW mantiene autoridad local sobre la ejecución de código externo y la configuración del entorno cuando abre proyectos persistidos que pueden no ser confiables.

## Requirements

### Requirement: las referencias de código de un proyecto requieren confianza local
Abrir o reproducir un proyecto MUST NOT ejecutar una aplicación standalone ni cargar un plugin externo sólo porque el proyecto contenga su ruta. Antes del primer lanzamiento/carga de una identidad binaria, Estudio DAW MUST solicitar aprobación explícita y guardar cualquier confianza concedida en configuración local fuera del proyecto. La identidad aprobada MUST corresponder a la ruta canónica y al contenido que se ejecutará; un cambio de destino o contenido invalida la aprobación anterior. La persona usuaria MUST poder revocar la confianza local.

#### Scenario: proyecto nuevo referencia un ejecutable no confiable
- **WHEN** se solicita Play para un proyecto cuya aplicación standalone todavía no está aprobada
- **THEN** Estudio DAW no inicia el proceso
- **AND** presenta una solicitud de aprobación que identifica el ejecutable resuelto
- **AND** sólo inicia el proceso después de una aprobación explícita

#### Scenario: se reemplaza un ejecutable aprobado
- **WHEN** una ruta aprobada resuelve a un destino canónico o contenido distinto del aprobado
- **THEN** Estudio DAW invalida la confianza previa y solicita aprobación otra vez
- **AND** no inicia ni carga ese destino hasta recibir aprobación

#### Scenario: el usuario revoca una aprobación
- **WHEN** se revoca la confianza local de una aplicación o plugin
- **THEN** proyectos abiertos o posteriores no pueden volver a ejecutarlo sin aprobación explícita

### Requirement: la configuración del proyecto no controla el entorno de procesos
`WINEPREFIX` y otras variables de entorno del proceso MUST provenir de configuración local controlada por la aplicación o el usuario, nunca de valores suministrados por el proyecto. Los campos persistidos heredados pueden conservarse al cargar/guardar para compatibilidad, pero MUST NOT determinar el entorno efectivo.

#### Scenario: proyecto contiene un prefijo Wine
- **WHEN** un proyecto guarda un valor `wine_prefix` distinto de la configuración local
- **THEN** Play usa sólo el prefijo local configurado
- **AND** el valor del proyecto no se aplica al entorno del proceso

### Requirement: los proyectos no confiables no eluden la aprobación al cargar plugins
La referencia a un plugin externo persistida en un proyecto MUST pasar por la misma política de aprobación local que una aplicación standalone antes de cargar código ejecutable del plugin.

#### Scenario: proyecto referencia un plugin aún no aprobado
- **WHEN** Play requiere cargar un plugin externo que no está aprobado localmente
- **THEN** el plugin no se carga
- **AND** Estudio DAW presenta el plugin resuelto para aprobación explícita antes de continuar
