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

- Se importan archivos con hasta tantos canales como tenga la salida de la
  pista. No existe un selector de mapeo ni downmix para archivos multicanal.
- La ubicación elegible es el compás inicial; todavía no hay cursor de edición
  ni colocación por tiempo absoluto.
- La waveform representa el primer canal convertido a mono para visualización;
  no altera la fuente ni decide cómo sonará la pista.
- No hay preescucha, edición visual de recorte/desplazamiento ni reproducción de
  regiones en el transporte. La forma de onda no implica que el audio se esté
  reproduciendo.
- Los procesos externos `ffmpeg` y `ffprobe` deben estar instalados. Un fallo se
  presenta como error de importación/forma de onda, sin registrar una región
  incompleta.

La tarea OpenSpec 2.2 permanece abierta hasta cubrir preescucha, mapeo de
canales y controles de límite/desplazamiento además de validar el flujo visual.
