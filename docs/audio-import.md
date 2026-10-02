# Importación inicial de audio

La shell Tauri permite importar un archivo a una pista de audio existente. La
persona elige si copia el archivo al directorio `media/` junto al proyecto o si
vincula su ubicación original, y define el compás inicial de la región. La
operación del proyecto registra procedencia y región dentro de una transacción
reversible; mover o recortar la región no reescribe el archivo fuente.

`estudio-daw-media-adapter` consulta frecuencia de muestreo, cantidad de canales
y duración con `ffprobe`, calcula firma/hash, y genera pares min/max para la
representación visual mediante `ffmpeg`. Sólo se envían a la interfaz los
metadatos y los bins reducidos; el PCM no cruza el puente Tauri. El cálculo de la
forma de onda limita la duración procesada a diez minutos y produce como máximo
4096 bins.

## Límites actuales

- Grabación desde Tauri: guarda primero el proyecto, asigna una fuente y arma
  una o más pistas de audio en el Mezclador; Record inicia la captura desde el
  cursor de Arreglo y Stop finaliza un WAV por pista bajo
  `media/recordings/`. Cada toma se importa como fuente del proyecto y crea una
  región en la pista armada. La región usa el historial normal, por lo que
  deshacer quita la región y su asociación al proyecto, pero conserva el WAV;
  rehacer vuelve a asociarla. Si el ring de captura se desborda, la región se
  conserva y el aviso informa las muestras descartadas. La continuidad y el
  resultado audible deben verificarse en hardware.
- Esta primera modalidad no tiene cuenta previa, grabación por secciones ni
  pausa durante la toma; A/B debe estar desactivado. No hay todavía destino
  físico independiente por pista ni medidor de entrada dedicado. Al cambiar una
  asignación mientras suena, se aplica en el siguiente inicio.

- Se puede seleccionar un canal mono o una pareja estéreo de la fuente; la
  selección se conserva en la región. Las regiones históricas sin selección
  explícita se convierten a estéreo con el downmix estándar de `ffmpeg`.
  Durante reproducción, la selección mono se duplica a L/R y la pareja elegida
  se enruta a L/R.
- El diálogo permite inspeccionar metadatos y preescuchar hasta 30 segundos de
  Opus mediante WebAudio antes de confirmar la importación. Las regiones
  importadas conservan el mismo control de preescucha; no mueve el transporte.
- El cursor de edición de Arreglo se coloca haciendo clic en la regla o en un
  espacio vacío de pista, con resolución de un cuarto de pulso. La región se
  importa desde ese punto; el campo de compás coloca el cursor al comienzo del
  compás indicado. Todavía no es un cursor compartido con Session ni sigue el
  transporte.
- En Arreglo se arrastra una región para moverla y sus bordes para recortarla
  hacia dentro. El botón × quita sólo la región; conserva la fuente registrada
  y el archivo original. Los cambios entran en el historial deshacer/rehacer.
- La waveform representa el primer canal convertido a mono para visualización;
  no altera la fuente ni decide cómo sonará la pista.
- La preescucha está disponible después de seleccionar el archivo, antes de
  confirmarlo, pero todavía no hay una biblioteca para buscar/filtrar archivos
  ya importados. El recorte sólo reduce los límites actuales; no permite
  extenderlos otra vez con el ratón. Play decodifica incrementalmente cada
  región con `ffmpeg` en un worker y un ring PCM acotado, y el plan mezcla esas
  muestras con las pistas de instrumentos MIDI. Al iniciar desde detenido,
  Play parte del cursor de Arreglo y ajusta el desplazamiento de fuente; durante
  Play se puede buscar desde la regla sin cerrar PipeWire. Al iniciar Play con
  un rango A/B guardado, el runtime repite audio y MIDI sin cerrar PipeWire; el
  plan silencia desde B hasta aplicar el salto a A. La primera vuelta adicional
  se prepara antes de abrir el stream y las siguientes fuera del callback. Si
  falla la preparación o el intercambio, la consulta de posición devuelve un
  error. Al buscar/iniciar dentro de un clip MIDI se restauran sus notas activas,
  el último valor previo de CC 0–127, Pitch Bend, presión de canal, Program
  Change y presión por tecla activa. SysEx no se reproduce. Los cambios del rango durante la reproducción aplican al siguiente
  inicio. Al mover, recortar o quitar una región durante Play, el runtime
  recompila el plan desde la posición actual y lo publica en el siguiente límite
  de bloque sin cerrar PipeWire. En pausa, el proyecto se actualiza y el plan
  nuevo se aplica al reanudar. La waveform sólo visualiza datos y no simula la
  señal reproducida.
- Los procesos externos `ffmpeg` y `ffprobe` deben estar instalados. Un fallo se
  presenta como error de importación/forma de onda, sin registrar una región
  incompleta.
Tras confirmar una importación, la shell identifica la nueva región por su ID,
la selecciona en Arrangement y abre el detalle contextual. El estado del archivo
seleccionado se anuncia con `aria-live`; la importación no depende únicamente
del color o del cambio de forma de onda para comunicar éxito.
**Verificación final agrupada 2026-09-30:** tras la mejora de selección de la
nueva región, pasaron sintaxis de los tres scripts frontend, formato Rust,
`cargo test -p estudio-daw-ui-shell` (32 pasadas, 1 ignorada),
`cargo test -p estudio-daw-media-adapter` (8 pasadas en 2 suites) y la prueba
focal de importación del `command-bus`. OpenSpec y `git diff --check` también
pasaron.
El mezclador de clips tiene cobertura determinista de límites y desvanecimientos:
una región no mezcla fuera de su duración y aplica entrada/salida gradual por
canal. Esta prueba no sustituye la reproducción real ni la QA acústica.




La tarea OpenSpec 2.2 permanece abierta hasta completar QA visual del flujo de
importación y edición en la aplicación. La tarea 2.3 también sigue abierta por
QA funcional/acústica, posicionamiento compartido del transporte y ajuste
sample-accurate del scheduler; los cambios de región durante Play ya
reconstruyen el plan activo.

**Verificación 2026-09-30:** `cargo test -p estudio-daw-media-adapter` pasó con
8 pruebas en 2 suites; `cargo test -p estudio-daw-command-bus import_audio`
pasó la prueba transaccional de importación y deshacer. La inspección visual
Tauri a tamaños objetivo sigue limitada por la geometría tiled de Hyprland; no
se presenta el preview como sustituto de esa QA.
