# Tareas

## 1. Contrato de ajustes del entorno de audio

- [x] 1.1 Añadir ajustes serializables e independientes del motor de plataforma para el período solicitado al dispositivo, los cuadros de seguridad de reproducción, el perfil seleccionado y el estado admitido/pendiente; comprobar su validación y las pruebas de ida y vuelta.
- [x] 1.2 Añadir diagnósticos del período solicitado/efectivo y de la frecuencia de muestreo, con conversión de cuadros a milisegundos; comprobar mediante pruebas unitarias los períodos distintos y los valores no disponibles del motor.
- [x] 1.3 Validar los períodos de los perfiles frente a las restricciones de los flujos PipeWire e informar si se requiere reinicio o aún no se conoce el cuanto efectivo, hasta que un anfitrión conectado pueda comunicar el cuanto negociado del grafo; comprobar el rechazo de valores no válidos y que el período solicitado nunca se presente como latencia efectiva.

## 2. Separar el almacenamiento temporal de reproducción y la continuidad MIDI

- [x] 2.1 Permitir configurar el objetivo de la cola PCM SoundFont fuera de la devolución de audio e independientemente del período PipeWire y la capacidad del anillo; comprobar que cada ajuste sólo modifica su propia métrica de cola.
- [x] 2.2 Persistir cambios de perfil mediante el plano de control e informar que se aplicarán al iniciar/reiniciar el siguiente flujo, mientras la ventana actual no tenga conexión con un motor activo; comprobar que las preferencias se guardan aparte de los datos del proyecto/toma y no alteran el flujo CLI en ejecución.
- [x] 2.3 Comprobar que los eventos MIDI `Note On`/`Note Off` y el estado de sustain CC64 conservan orden y semántica al cambiar de perfil, incluido el caso de una nota sostenida por el pedal.
- [x] 2.4 Identificar qué rutas del grafo de reproducción pueden consumir cuadros de seguridad adicionales independientemente de la monitorización en vivo; añadir pruebas deterministas de rutas separadas y un diagnóstico cuando no se admita esa separación o se compartan rutas.

## 3. Ajustes de aplicación y controles del DAW

- [x] 3.1 Exponer los ajustes del entorno de audio, la selección de perfiles, los diagnósticos efectivos y los resultados aplicados/pendientes mediante la API de aplicación; comprobar que las pruebas de esta capa no requieren hardware.
- [x] 3.2 Añadir a la interfaz Tauri controles editables de los perfiles de interpretación/grabación y reproducción multipista, con unidades separadas para el período del dispositivo y el búfer de reproducción; comprobar que los valores mostrados se actualizan desde el estado de la aplicación.
- [x] 3.3 Añadir explicaciones breves en la interfaz sobre cuadros/frecuencia de muestreo, compensaciones de CPU/planificación, negociación del motor de plataforma y estado actual de soporte de monitorización; comprobar que la documentación no lo presenta como caché de hardware ni afirma latencia total de extremo a extremo.
- [x] 3.4 Persistir los ajustes de la política de ruteo PipeWire predeterminada actual en un archivo versionado de preferencias del usuario, sin guardar ajustes específicos de la máquina en el JSON del proyecto; comprobar carga, guardado, valores predeterminados si falta el archivo y rechazo de versiones no válidas.

## 4. Verificación de integración y guía de uso

- [x] 4.1 Documentar cómo elegir y ajustar ambos perfiles, interpretar los valores solicitados/efectivos y entender las limitaciones de monitorización en vivo del motor actual; comprobar que cada ajuste documentado coincide con el comportamiento de la interfaz.
- [x] 4.2 Verificar en hardware PipeWire los cambios de perfil durante interpretación, grabación y reproducción; registrar el cuanto efectivo cuando esté disponible, profundidad de cola, interrupciones, eventos MIDI perdidos y comportamiento de reinicio, sin presentar medidas parciales como latencia física total.
  - Verificación en hardware (2026-09-27, AudioBox USB 96, 48 kHz): durante la entrada KeyLab en vivo se observó un flujo de interpretación/grabación de 256 cuadros y cuanto 256; 1 evento MIDI y 0 pérdidas MIDI. La grabación WAV con perfil de 256 capturó 188.928 muestras intercaladas sin descartes; la fuente indicó cuanto 256 mientras el controlador del destino de salida informó 1024. En el perfil de reproducción, el período 512 coincidió con cuanto 512 en destino/cliente; el proceso FluidR3_GM SF2 con objetivo PCM 1024 terminó con 1.024/2.048 cuadros y cero interrupciones FluidSynth, errores del proceso o pérdidas MIDI. Los cambios se leen al abrir el siguiente flujo; la ventana Tauri actual no conecta los ajustes a un motor/transporte en ejecución. Los totales `ERR` observados en `pw-top` fueron 235 (salida) y 16.140 (fuente); como no se registró una línea base por ejecución, se anotan como totales, no como errores causados por un perfil particular. No se afirma latencia física de extremo a extremo.
- [x] 4.3 Ejecutar `cargo fmt --all -- --check`, las pruebas de crates pertinentes, `cargo test --workspace -- --test-threads=1`, `git diff --check` y `openspec validate audio-latency-profiles --strict`; registrar por separado las comprobaciones de hardware que dependan del entorno.

### Estado de integración posterior (2026-09-27)

El transporte Tauri ahora abre un flujo de reproducción PipeWire y consume el
perfil seleccionado al iniciar el flujo. Planifica clips de varias pistas MIDI,
carga el instrumento Sine o FluidSynth configurado en cada pista y admite
pausa/reanudación/detención. Este trabajo posterior no añade reproducción de
clips de archivos de audio ni grabación/monitorización de entrada en la ventana
Tauri; `audio-record` y `midi-synth-live` siguen usando el perfil de
interpretación/grabación desde la CLI. El indicador Tauri todavía no puede
mostrar el cuanto efectivo de PipeWire.
