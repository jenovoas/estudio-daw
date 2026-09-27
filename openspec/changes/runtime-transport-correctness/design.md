# Diseño

## Comportamiento de ejecución

- En la devolución de PipeWire, un flujo pausado llena de silencio el búfer del dispositivo y no llama a `RenderPlanProcessor::process`; así congela el estado de los nodos y evita extraer PCM SoundFont de la cola. El proceso SoundFont comparte la marca de pausa, sigue atendiendo los controles de detención/MIDI y espera sin renderizar nuevos bloques. El planificador MIDI desplaza los plazos de los eventos según la duración de la pausa y sigue siendo el reloj de transporte de este adaptador de reproducción.
- Los eventos de clip son admisibles si su pulso local es menor o igual que `duration_ticks`, conservando un `NoteOff` que ocurra exactamente en el límite final. Su posición absoluta es `start_tick + event.tick`.
- El intercambio del plan de renderizado debe devolver silencio si no hay un plan activo, en vez de desenrollar la pila dentro de la devolución de audio. `SineSynth` limita el tono de `NoteOn` al intervalo 0–127 antes de consultar su tabla de frecuencias.
- La toma MIDI de demostración de la interfaz calcula los microsegundos usando sus propias constantes PPQ y de tempo.

## Dirección visual de la interfaz

Conservar el puente de comandos Tauri y el modelo de datos existentes. Rediseñar la ventana alrededor de un transporte compacto de DAW, una cuadrícula de arreglo marcada, encabezados de pista legibles, regiones MIDI coloreadas con vista previa de notas y una paleta oscura contenida con acentos naranja cálido. Los controles seguirán vinculados sólo a comandos existentes; no insinuar capacidades de edición o grabación que no estén implementadas.

## Límite de alcance

Este cambio no implementa un DAG de audio ni modifica los límites de dependencias entre crates. Son cambios arquitectónicos independientes y requieren su propio plan explícito de capacidades y tareas. La semántica existente de `RenderPlanBuilder::connect` sigue pendiente de auditoría hasta que se especifique ese trabajo posterior.

## Verificación

Usar pruebas unitarias deterministas para límites de clips, conversión temporal MIDI, seguridad de tonos y respuesta segura del plan de renderizado. Antes de publicar, ejecutar formato, la suite completa del espacio de trabajo en un solo hilo, su comprobación, la validación estricta de OpenSpec y la revisión de diferencias.
