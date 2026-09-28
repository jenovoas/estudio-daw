# Spec Delta

## MODIFIED Requirements

### Requirement: la reproducción en tiempo real falla de forma segura
La devolución de audio MUST NOT entrar en pánico si una ranura del plan de renderizado está vacía de forma inesperada. `SineSynth` MUST procesar de forma segura cualquier valor de nota MIDI recibido, sin indexar fuera de su tabla de frecuencias de 128 notas. Los fallos del helper de instrumento, incluidos locks envenenados, MUST convertirse en diagnósticos recuperables y no en pánicos del helper. La ausencia de dispositivo de salida o backend MUST devolverse como error estructurado; ningún constructor o valor predeterminado puede transformar esa condición en un pánico.

#### Scenario: estado de ejecución o tono no válido
- **WHEN** una ranura del plan activo está vacía
- **THEN** la llamada de retorno devuelve silencio sin entrar en pánico
- **WHEN** SineSynth recibe una nota superior a 127
- **THEN** procesa una altura válida dentro de los límites sin entrar en pánico

#### Scenario: mutex del helper está envenenado
- **WHEN** el helper vuelve a acceder al estado del plugin después de que un hilo haya entrado en pánico mientras mantenía el mutex
- **THEN** el helper recupera el guard o responde con un error estructurado
- **AND** no entra en pánico por el envenenamiento del mutex

#### Scenario: no existe dispositivo de salida
- **WHEN** se solicita iniciar reproducción sin backend o dispositivo de salida disponible
- **THEN** la capa de audio devuelve un diagnóstico estructurado
- **AND** el transporte queda detenido y la interfaz no informa reproducción conectada
