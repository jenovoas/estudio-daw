# runtime-midi-playback Specification

## Purpose

Define una reproducción fiable de clips MIDI, el comportamiento de pausa/reanudación y la gestión segura de fallos en el entorno de audio de escritorio.

## Requirements

### Requirement: la pausa congela el estado renderizado del instrumento
Cuando la reproducción de escritorio está en pausa, la devolución de audio MUST emitir silencio sin avanzar el estado de los nodos del plan de renderizado ni consumir PCM en cola del instrumento. Los procesos de SoundFont MUST detener el renderizado de PCM y el avance de sus voces mientras dure la pausa. El planificador MIDI MUST conservar la temporización de los eventos respecto del transporte reanudado.

#### Scenario: reanudar después de pausar
- **WHEN** se pausa la reproducción mientras hay una nota MIDI o audio del instrumento en cola
- **THEN** la salida queda en silencio y las voces del plan de renderizado/FluidSynth no avanzan durante la pausa
- **AND** la cola PCM acotada permanece inmóvil
- **AND** los eventos programados continúan desde la misma posición musical al reanudar

### Requirement: la reproducción respeta los límites del clip MIDI
El planificador MUST ubicar cada evento admisible de la toma en `start_tick` del clip más el pulso local del evento, y MUST excluir los eventos cuyo pulso local sea mayor que `duration_ticks`.

#### Scenario: inicio y final del clip
- **WHEN** un clip MIDI tiene un inicio distinto de cero y una duración limitada
- **THEN** los eventos anteriores o iguales al límite de duración se programan con el desplazamiento temporal del clip
- **AND** los eventos posteriores al límite de duración no se programan

### Requirement: la reproducción en tiempo real falla de forma segura
La devolución de audio MUST NOT entrar en pánico si una ranura del plan de renderizado está vacía de forma inesperada. `SineSynth` MUST procesar de forma segura cualquier valor de nota MIDI recibido, sin indexar fuera de su tabla de frecuencias de 128 notas.

#### Scenario: estado de ejecución o tono no válido
- **WHEN** una ranura del plan activo está vacía
- **THEN** la llamada de retorno devuelve silencio sin entrar en pánico
- **WHEN** SineSynth recibe una nota superior a 127
- **THEN** procesa una altura válida dentro de los límites sin entrar en pánico

### Requirement: el inicio del dispositivo y el transporte de dominio permanecen coherentes
El adaptador de escritorio MUST NOT dejar la sesión en estado de reproducción si falla el inicio de PipeWire.

#### Scenario: falla el inicio del dispositivo de audio
- **WHEN** la sesión acepta Play, pero el motor de audio no puede iniciarse
- **THEN** el adaptador envía Stop a la sesión de dominio y devuelve el error del motor
- **AND** no informa que haya un flujo de reproducción conectado
