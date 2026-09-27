# Diseño

## Context

Consultar `proposal.md` y `specs/audio-runtime-control/spec.md`. El entorno Linux actual ofrece `PipeWireStreamConfig.period_frames` y una capacidad máxima independiente para el anillo PCM, mientras que el proceso FluidSynth limita internamente el PCM en cola. El período predeterminado es de 256 cuadros y el objetivo de la cola SoundFont está fijado actualmente en dos bloques de 256 cuadros. La API de aplicación y la ventana Tauri aún no exponen los ajustes del entorno de audio.

El período solicitado a PipeWire es una petición de negociación; el cuanto activo del grafo puede ser distinto. Por eso, la negociación del bloque del dispositivo y los cuadros de reproducción en cola del DAW necesitan modelos e indicadores separados. Las opciones de perfil no deben describirse como controles de caché del hardware.

## Goals / Non-Goals

**Objetivos:**

- Ofrecer al DAW un modelo de ajustes independiente del motor de plataforma para el período solicitado al dispositivo, los cuadros de seguridad de reproducción y la selección de perfiles de interpretación/reproducción.
- Informar los ajustes solicitados y efectivos, incluidas las duraciones derivadas de la frecuencia de muestreo y la cantidad de cuadros.
- Ofrecer perfiles iniciales prácticos que el usuario pueda ajustar para cada dispositivo de audio seleccionado.
- Conservar intacto el estado MIDI y de grabación cuando cambien los ajustes.

**Exclusiones:**

- Prometer una latencia fija entre tecla y parlante o funcionamiento sin interrupciones para algún perfil.
- Cambiar el firmware/los ajustes de Universal Control de AudioBox ni emular la protección contra interrupciones de PreSonus Studio One.
- Cambiar automáticamente la frecuencia de muestreo o medir la latencia analógica entre DAC y parlante.
- Implementar en este cambio todo el motor de pistas/monitorización multipista; los ajustes deben indicar con precisión cuándo el motor actual no puede separar los búferes de reproducción y monitorización.

## Decisions

### Mantener separados el período del dispositivo y el búfer de seguridad de reproducción

Representar en cuadros el período solicitado al dispositivo y el búfer adicional de seguridad de reproducción. Calcular los milisegundos a partir de la frecuencia de muestreo activa al mostrarlos, en vez de guardar una duración redondeada que quedaría obsoleta si cambia la frecuencia. Informar por separado el período negociado por el motor de plataforma. En PipeWire, pasar el período solicitado mediante la configuración de flujo existente y consultar/informar el cuanto activo real usando la información de ejecución del motor.

Alternativa: ofrecer un único control deslizante de «latencia». Se descartó porque mezcla los bloques de planificación del motor con el audio renderizado en cola y no permite explicar qué ruta recibe el retardo adicional.

### Guardar las preferencias fuera del contenido de audio del proyecto

Guardar las preferencias de dispositivos de audio y los valores de perfiles en los ajustes del usuario del DAW, asociados a la identidad del dispositivo/motor de plataforma; usar memoria como alternativa si no se pueden persistir. Una sesión puede elegir el perfil activo, pero al abrir proyectos no se sobrescriben las preferencias de hardware específicas de la máquina. Así se evita trasladar a otra máquina o interfaz el cuanto configurado para AudioBox.

Alternativa: serializar en `project.json` el período elegido para el hardware. Se descartó porque la compatibilidad de dispositivos y los períodos admitidos varían entre máquinas; el contenido del proyecto y la configuración física de audio tienen distinta portabilidad.

### Los perfiles son preajustes editables de los mismos controles

Ofrecer perfiles de interpretación/grabación y reproducción multipista que establezcan el período del dispositivo y el búfer de seguridad de reproducción. El usuario puede ajustar ambos campos y guardar esos valores como preferencias personales. El nombre del perfil es sólo una etiqueta práctica, no un detector automático de la carga de la sesión. Durante la sobregrabación, el DAW podrá conservar una ruta de entrada monitorizada de baja latencia y almacenar en búfer sólo el acompañamiento admisible cuando el motor disponga de planificación separada por ruta. Hasta entonces, la interfaz informa de la limitación actual de ruta compartida.

Alternativa: aumentar automáticamente los búferes cuando crezca el uso de CPU. Se aplazó porque el cambio automático de modo puede interrumpir la monitorización y el motor actual no permite controlar plazos/planificación de cada ruta.

### Aplicar cambios mediante el plano de control

Crear y validar la nueva configuración de ejecución fuera de la devolución de audio. Si el motor de plataforma admite renegociación en vivo, publicar/aplicar en un límite seguro del flujo; en caso contrario, detener/reabrir el flujo mediante el plano de control e indicar que queda pendiente o requiere reinicio. Las colas MIDI y el estado de notas/controladores deben permanecer bajo propiedad externa a la devolución y conciliarse explícitamente al sustituir el flujo. Sustain CC64 es un estado de controlador con marca de tiempo y no debe descartarse al cambiar de perfil.

Alternativa: modificar los ajustes PipeWire directamente en la devolución. Se descartó porque el trabajo de esta ruta debe permanecer acotado y la arquitectura actual prohíbe allí operaciones de control/entrada/salida.

### Mostrar explícitamente los valores solicitados y efectivos

La vista de ajustes muestra el período solicitado, el período efectivo informado por el motor de plataforma, la frecuencia de muestreo, los cuadros de seguridad de reproducción y cada duración conocida en cuadros y milisegundos. Si el motor no informa un valor, se muestra como no disponible. No se suman valores independientes para presentar una supuesta cifra de latencia de extremo a extremo.

## Risks / Trade-offs

- **[PipeWire podría negociar un cuanto distinto para el grafo]** → mostrar por separado los valores solicitado y efectivo, junto con diagnósticos del motor.
- **[Cambiar un flujo en ejecución podría interrumpir el audio]** → usar una actualización segura admitida por el motor o indicar claramente si es necesario detener/reiniciar el transporte; conservar los datos MIDI y de grabación pendientes.
- **[El grafo de renderizado actual podría no consumir por separado los cuadros de seguridad de reproducción]** → implementar primero el contrato de control y sus diagnósticos; no insinuar una latencia de monitorización independiente hasta que exista almacenamiento temporal por ruta.
- **[El período del dispositivo y el búfer de seguridad afectan partes distintas de la ruta de señal]** → etiquetar cada control y explicar la conversión de cuadros/frecuencia de muestreo en la interfaz.
- **[Algunos motores no exponen todos los datos de período]** → representar explícitamente los valores efectivos desconocidos y evitar estimarlos.

## Migration Plan

1. Añadir ajustes y diagnósticos del entorno de audio independientes del motor, con valores predeterminados que coincidan con el comportamiento actual (48 kHz y período solicitado de 256 cuadros cuando corresponda; cero cuadros adicionales de seguridad hasta que una ruta independiente los consuma).
2. Añadir al motor la capacidad de informar los períodos solicitado/efectivo sin cambiar los valores predeterminados actuales de la CLI.
3. Conectar los ajustes mediante la API de control de aplicación y mostrar los dos perfiles en la interfaz de ajustes del DAW.
4. Añadir búferes de seguridad de reproducción sólo donde el motor pueda aplicarlos a trabajo de reproducción admisible; mientras tanto, informar con precisión el estado de soporte.
5. Persistir las preferencias de usuario por dispositivo en un archivo de ajustes versionado. Los proyectos existentes no requieren migración.

Para revertir el cambio, quitar la conexión de preferencias de la interfaz y volver a los valores predeterminados actuales de configuración de flujo; los proyectos y las tomas MIDI permanecen intactos.

## Open Questions

- Los valores predeterminados exactos en cuadros para los dos perfiles deben confirmarse mediante pruebas deterministas de ejecución y mediciones de los motores admitidos durante la implementación; los preajustes siguen siendo editables y no constituyen garantías.
- Futuros motores podrían admitir incrementos distintos de búfer; el modelo de ajustes debería permitir que cada motor proporcione las opciones válidas mediante su propia validación, en vez de fijar valores específicos de AudioBox.
