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
  error. Los cambios del rango durante la reproducción aplican al siguiente
  inicio. Los cambios de región se guardan de inmediato y se oyen al
  detener e iniciar de nuevo; no reinician el dispositivo como parte de la
  edición. La waveform sólo visualiza datos y no simula la señal reproducida.
- Los procesos externos `ffmpeg` y `ffprobe` deben estar instalados. Un fallo se
  presenta como error de importación/forma de onda, sin registrar una región
  incompleta.

La tarea OpenSpec 2.2 permanece abierta hasta completar QA visual del flujo de
importación y edición en la aplicación. La tarea 2.3 también sigue abierta por
permanecer abierta por QA funcional/acústica y actualización del plan de región
al editar durante reproducción; el bucle A/B ya está conectado al runtime, pero
la suite no se ejecutó en esta intervención.
