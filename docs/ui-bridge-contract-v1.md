# Contrato UI/backend v1

Este documento fija los payloads del bridge Tauri y sirve también como frontera
para una futura UI web. Los structs correspondientes viven en
`crates/ui-shell/src/main.rs`; sus pruebas comprueban serialización camelCase,
versionado y ausencia de muestras/rutas locales en los contratos de vista.

## Reglas comunes

- Cada payload lleva `schemaVersion`; los cambios incompatibles crean una nueva
  versión (`*.v2`) en lugar de reinterpretar silenciosamente campos existentes.
- El bridge es un plano de control y vista, no un transporte de audio: nunca
  contiene PCM, muestras individuales, buffers de DSP ni handles/texturas `wgpu`.
- Los IDs de artefacto son opacos. El frontend no construye rutas ni recibe
  acceso general al filesystem; el backend resuelve cada ID contra el proyecto,
  la revisión y el scope autorizados.
- Toda petición de rango se valida y limita en backend. Resultados obsoletos se
  descartan comparando revisión/procedencia y, para streams, `sequence`.
- El render WebView puede usar su propia GPU, pero no se presupone compartir
  memoria, textura o dispositivo con `wgpu` nativo.

## Snapshot de proyecto

`project_snapshot` devuelve `ui-snapshot.v1`: revisión, estado de transporte,
tempo, conteos, lista compacta de pistas y capacidades de undo/redo. No incluye
el modelo serializado completo ni rutas de medios como permiso de acceso. La
ruta visible existente es sólo una etiqueta informativa; las operaciones sobre
medios deben usar sus comandos e IDs backend, nunca esa cadena.

```json
{
  "schemaVersion": "ui-snapshot.v1",
  "projectId": "song-01",
  "projectRevision": 7,
  "tempoBpm": 92.0,
  "transportState": "stopped",
  "trackCount": 2,
  "midiClipCount": 1,
  "audioClipCount": 1,
  "tracks": [{"id":"trk-1","name":"Voz","kind":"audio","noteCount":0}],
  "canUndo": true,
  "canRedo": false,
  "audioEngineConnected": false
}
```

La UI no debe tratar `audioEngineConnected: false` como reproducción activa. El
shell actual aún no une el transporte visual con el `RenderPlan`.

## Telemetría de medidores

`meter-frame.v1` contiene agregados por canal/ruta y bloque o ventana, con
posición de muestra y secuencia monotónica. `peakDbfs` y `rmsDbfs` son dBFS
finitos; el publicador limita la frecuencia (objetivo inicial: hasta 30 frames/s)
y coalesce frames atrasados en vez de acumular una cola visual ilimitada.

```json
{
  "schemaVersion": "meter-frame.v1",
  "sequence": 120,
  "atSample": 192000,
  "sampleRateHz": 48000,
  "windowFrames": 256,
  "meters": [{"channelId":"master","peakDbfs":-1.5,"rmsDbfs":-12.0,"clipping":false}]
}
```

El bridge descarta o agrupa actualizaciones atrasadas por `sequence`; ningún
medidor necesita muestras crudas para dibujar la aguja/barra.

## Referencias de artefactos visuales

`visualization-artifact-ref.v1` describe un derivado reproducible, por ejemplo
una pirámide min/max de waveform o un espectrograma. Incluye ID opaco, revisión,
digest de fuente y geometría/timebase. No expone ruta, descriptor, dirección de
memoria ni textura GPU. Si cambia la fuente/proxy, cambia su digest y los
artefactos incompatibles se invalidan.

### Waveform

El backend responde consultas de nivel y rango como `waveform-chunk.v1`, con un
límite fijo de bins por respuesta (máximo inicial: 4096). Cada bin lleva mínimo y
máximo normalizados a `[-1, 1]`, calculados al generar el derivado, no leyendo ni
serializando toda la fuente al solicitar el dibujo.

```json
{
  "schemaVersion": "waveform-chunk.v1",
  "artifactId": "viz_opaque_01HXYZ",
  "level": 3,
  "firstBin": 128,
  "bins": [{"min":-0.8,"max":0.9}]
}
```

### Espectrograma

La UI solicita coordenadas de tile (`x`, `y`, nivel) mediante el ID opaco. El
backend valida alcance/revisión y sirve bytes de imagen acotados con una política
local Tauri de asset/protocolo restringida al caché de visualización. El bridge
JSON devuelve `spectrogram-tile-ref.v1` (geometría y encoding, sin base64); el
payload binario se obtiene por separado y se limita, inicialmente, a tiles de
256×256 y formatos de imagen permitidos. Nunca se exporta una textura nativa
`wgpu` al WebView.

La compresión, paleta y cuantización son propiedades versionadas del artefacto;
la UI sólo presenta el tile. Un backend que no pueda servir el tile conserva la
vista alternativa CPU/canvas y no afecta reproducción ni captura.

## Estado de implementación

El snapshot Tauri ya se emite. Esta iteración incorpora los tipos de contrato y
pruebas de serialización para medidores y artefactos; los publicadores de
telemetría, jobs que generan pirámides/tiles y el renderer visual quedan como
trabajo de implementación posterior. La ruta y los límites de datos ya quedan
definidos para no acoplarlos al callback de audio ni a memoria GPU compartida.
