# Spec Delta

## Purpose

Provides a local sample-based instrument for practicing and producing with the existing MIDI and audio workflow. It uses user-owned SoundFont assets and keeps audio rendering isolated from control-plane and file-loading work.

## ADDED Requirements

### Requirement: el usuario puede elegir un instrumento SoundFont local
El sistema MUST permitir que el usuario seleccione un archivo SoundFont local y un preajuste de banco/programa disponible para un instrumento del proyecto. La configuración del instrumento MUST poder serializarse y MUST referenciar el archivo SoundFont sin copiarlo.

#### Scenario: elegir un preajuste disponible
- **WHEN** el usuario selecciona un banco SoundFont legible y un ajuste válido
- **THEN** el sistema lo carga fuera de la llamada de retorno de audio e informa que el ajuste seleccionado está listo

#### Scenario: SoundFont no está disponible
- **WHEN** un proyecto hace referencia a un banco SoundFont ausente o ilegible
- **THEN** el sistema informa la ruta y un error con pasos posibles, mantiene el proyecto recuperable y no sustituye otro instrumento silenciosamente

### Requirement: los instrumentos SoundFont responden a MIDI
El instrumento MUST responder a los eventos de activación/desactivación de nota, velocidad y selección de programa admitidos por el SoundFont elegido. Las tomas MIDI y sus eventos MUST permanecer independientes del generador de sonido seleccionado.

#### Scenario: activar y soltar una nota
- **WHEN** el instrumento recibe una nota activada seguida de una nota apagada
- **THEN** el ajuste seleccionado reproduce la nota con el timbre SoundFont y la libera según su envolvente

#### Scenario: cambiar de instrumento sin alterar la toma
- **WHEN** el usuario cambia el instrumento de una pista desde la fuente de prueba sinusoidal a un ajuste SoundFont
- **THEN** los datos de eventos MIDI permanecen intactos y pueden reproducirse con cualquiera de los instrumentos disponibles

### Requirement: el audio en tiempo real permanece aislado del trabajo SoundFont
La carga de SoundFont, el acceso al sistema de archivos y el control de síntesis MUST NOT bloquear, asignar memoria ni realizar operaciones de entrada/salida dentro de la devolución de audio de PipeWire. Si el audio renderizado no está disponible temporalmente, la devolución MUST continuar con silencio en los cuadros afectados y exponer un diagnóstico de interrupción, en vez de esperar al proceso del instrumento.

#### Scenario: el proceso de trabajo incumple un plazo de audio
- **WHEN** el proceso auxiliar del instrumento no ha publicado audio para un bloque de la llamada de retorno
- **THEN** la llamada de retorno emite silencio para ese bloque, incrementa un contador observable de faltas de audio y continúa procesando el resto del grafo

#### Scenario: cargar o reemplazar un SoundFont durante la sesión
- **WHEN** el usuario solicita cargar o reemplazar un banco SoundFont
- **THEN** la operación ocurre fuera de la llamada de retorno de PipeWire y el plan de renderizado activo sigue siendo válido hasta que el reemplazo esté listo

#### Scenario: una sustitución fallida conserva el instrumento actual
- **WHEN** falla la preparación de un banco SoundFont o de un plan de renderizado de reemplazo
- **THEN** el reemplazo no se publica, el plan activo y su proceso auxiliar siguen disponibles para reproducir, y los planes y recursos retirados sólo se destruyen desde el hilo de control ajeno al tiempo real

### Requirement: las dependencias y los archivos del instrumento permanecen locales y auditables
La función SoundFont MUST funcionar sin conexión de red ni API pagada. La aplicación MUST NOT incluir ni redistribuir de forma predeterminada los SoundFont del usuario, MUST identificar el entorno de síntesis requerido cuando no esté disponible y MUST documentar los avisos de entornos de terceros por separado de las licencias de los archivos SoundFont.

#### Scenario: iniciar el instrumento sin conexión
- **WHEN** están disponibles el motor de síntesis local requerido y el banco SoundFont, pero no hay conexión de red
- **THEN** el instrumento puede cargarse y reproducir sin contactar un servicio externo

#### Scenario: biblioteca de ejecución no disponible
- **WHEN** el usuario elige el motor SoundFont, pero su biblioteca de ejecución no está instalada
- **THEN** el sistema informa el diagnóstico de instalación y ejecución, y conserva el instrumento de prueba sinusoidal como alternativa explícita
