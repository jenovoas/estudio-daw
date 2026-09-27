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
  explícita mantienen la interpretación anterior de usar todos sus canales.
  Esta elección aún no implementa el ruteo ni el downmix de reproducción.
- El diálogo permite inspeccionar metadatos y preescuchar hasta 30 segundos de
  Opus mediante WebAudio antes de confirmar la importación. Las regiones
  importadas conservan el mismo control de preescucha; no mueve el transporte.
- La ubicación elegible es el compás inicial; todavía no hay cursor de edición
  ni colocación por tiempo absoluto.
- La waveform representa el primer canal convertido a mono para visualización;
  no altera la fuente ni decide cómo sonará la pista.
- La preescucha está disponible después de seleccionar el archivo, antes de
  confirmarlo, pero todavía no hay una biblioteca para buscar/filtrar archivos
  ya importados. No hay edición visual de recorte/desplazamiento ni reproducción
  de regiones en el transporte. La forma de onda no implica que el audio se
  esté reproduciendo.
- Los procesos externos `ffmpeg` y `ffprobe` deben estar instalados. Un fallo se
  presenta como error de importación/forma de onda, sin registrar una región
  incompleta.

La tarea OpenSpec 2.2 permanece abierta hasta cubrir controles de
límite/desplazamiento, borrado, cursor de edición y validar el flujo visual.
