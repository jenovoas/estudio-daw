# runtime-midi-playback Specification

## Purpose

Define una reproducción fiable de clips MIDI, el comportamiento de pausa/reanudación y la gestión segura de fallos en el entorno de audio de escritorio.

## Requirements

### Requirement: la pausa congela el estado renderizado del instrumento
Cuando la reproducción de escritorio está en pausa, la devolución de audio MUST emitir silencio sin avanzar el estado de los nodos del plan de renderizado ni consumir PCM en cola del instrumento. Los procesos de SoundFont MUST detener el renderizado de PCM y el avance de sus voces mientras dure la pausa. El planificador MIDI MUST conservar la temporización de los eventos respecto del transporte reanudado.

#### Scenario: reanudar después de pausar
- **WHEN** playback is paused while a MIDI note or queued instrument audio is active
- **THEN** output is silent and render-plan/FluidSynth voices do not advance during the pause
- **AND** the existing bounded PCM queue remains stationary
- **AND** scheduled events continue from the same musical position after resume

### Requirement: la reproducción respeta los límites del clip MIDI
El planificador MUST ubicar cada evento admisible de la toma en `start_tick` del clip más el pulso local del evento, y MUST excluir los eventos cuyo pulso local sea mayor que `duration_ticks`.

#### Scenario: inicio y final del clip
- **WHEN** a MIDI clip has a nonzero start and a bounded duration
- **THEN** events before or at the duration boundary are scheduled at the clip-offset time
- **AND** events after the duration boundary are not scheduled

### Requirement: la reproducción en tiempo real falla de forma segura
La devolución de audio MUST NOT entrar en pánico si una ranura del plan de renderizado está vacía de forma inesperada. `SineSynth` MUST procesar de forma segura cualquier valor de nota MIDI recibido, sin indexar fuera de su tabla de frecuencias de 128 notas.

#### Scenario: estado de ejecución o tono no válido
- **WHEN** an active plan slot is empty
- **THEN** the callback returns silence without unwinding
- **WHEN** SineSynth receives a note above 127
- **THEN** it renders a bounded valid pitch without panicking

### Requirement: el inicio del dispositivo y el transporte de dominio permanecen coherentes
El adaptador de escritorio MUST NOT dejar la sesión en estado de reproducción si falla el inicio de PipeWire.

#### Scenario: falla el inicio del dispositivo de audio
- **WHEN** the session accepts Play but the audio backend cannot start
- **THEN** the adapter sends Stop to the domain session and returns the backend error
- **AND** it does not report a connected playback stream
