# Bus de comandos del dominio

`estudio-daw-command-bus` coordina las mutaciones del proyecto y del transporte
desde una frontera portable. La interfaz, la CLI, MIDI, los guiones y los agentes pueden emitir
el mismo `CommandEnvelope`; no necesitan conocer los detalles internos de
`ProjectHistory` ni de `Session`.

## Flujo

```text
CommandEnvelope
  → DomainCommandBus de capacidad acotada
  → validar id, versión y precondiciones
  → aplicar a Session o ProjectHistory
  → DomainEvent atribuido al autor
  → instantánea consultable por la interfaz
```

El productor usa `try_send`; una cola llena devuelve un error en vez de bloquear.
El consumidor drena comandos fuera de la llamada de retorno de audio. Este bus
de dominio es distinto de `estudio_daw_session::CommandBus`, que sigue siendo la
cola pequeña de ejecución en vivo para el transporte MIDI.

## Contrato del comando

Cada `CommandEnvelope` contiene:

- `id` único y `version` de schema;
- `author` (`user`, `midi`, `script`, `agent` o `import`);
- precondición opcional de revisión del proyecto;
- precondición opcional de estado del transporte;
- comando serializable de sesión o proyecto.

Una precondición que no coincide produce un diagnóstico y deja el estado intacto.
Los eventos guardan el id y autor del comando para poder auditar quién causó una
mutación.

## Operaciones disponibles

- transporte: los comandos `SessionCommand` existentes;
- pistas: agregar, duplicar, renombrar, reordenar, activar y quitar; ajustar
  silencio, solo, ganancia y panorama; asignar/quitar un nombre de grupo a una
  selección de pistas en una sola operación reversible, sin vincular sus valores
  de mezcla; asignar una entrada física opaca a una pista de audio y una salida
  interna a una pista de audio, bus o Master; armar/desarmar grabación;
- escenas/casillas: crear, renombrar, reordenar y quitar escenas; asignar o
  quitar casillas que referencian clips existentes sin copiarlos;
- audio: agregar, recortar, mover, ajustar ganancia y desvanecimientos de clips;
  asignar/quitar entrada y armar pistas con `SetTrackInputRoute` y
  `SetTrackRecordArm`;
- medios: asociar una fuente original o proxy a una pista de audio mediante un
  comando reversible; validar que la firma y el hash del proxy correspondan a
  la fuente;
- MIDI: adjuntar una toma, mover/dividir/duplicar clips y cuantizar eventos de activación/desactivación de nota; la división parte eventos en el cursor y conserva el estado de canal recuperable al comenzar el segundo fragmento;
- historial: deshacer y rehacer transacciones del proyecto.

Las operaciones se validan contra las relaciones del proyecto antes de
confirmarse. Las rutas de salida deben apuntar a una pista de audio existente y
no pueden formar ciclos; quitar un destino reasigna sus entradas a la salida que
tenía ese destino. Por ejemplo, una casilla no puede enlazar un clip MIDI a una pista
de audio. Duplicar una pista crea identidades nuevas para sus clips, fuentes,
lista de reproducción y casillas; no copia los archivos fuente. Mover una región
de audio cambia su posición musical y conserva el desplazamiento/duración de la
fuente. Cuantizar mantiene intactos los controladores. Dividir un clip MIDI cierra al corte las notas activas del primer fragmento y las rearticula con estado CC/pitch bend/presión/programa al inicio del segundo; SysEx anterior no se vuelve a emitir. Estas modificaciones
quedan cubiertas por el mismo historial reversible. Las mutaciones del proyecto
generan `ProjectEvent` dentro de `DomainEventPayload::ProjectChanged`.

La entrada física se persiste como clave opaca de dispositivo más uno o dos
índices de canal. `SetTrackInputRoute` y `SetTrackRecordArm` son reversibles; el
dominio sólo permite armar una pista de audio con entrada asignada. PipeWire
resuelve la clave al iniciar Play/Record. El adaptador captura F32 estéreo a un
ring SPSC y el plan mezcla los canales seleccionados en la pista antes de aplicar
ganancia/pan y ruteo interno. Durante Record un worker escribe además un WAV por
pista armada bajo `media/recordings/`; Stop finaliza el archivo y ejecuta
`ImportAudio` para crear la fuente y región reversibles en el punto de inicio.
El callback no hace I/O. La primera versión requiere proyecto guardado y no
ofrece cuenta previa, A/B ni pausa durante grabación. Las claves ausentes
producen un error accionable. Siguen pendientes capacidades reales por
dispositivo y salidas físicas por pista.

## Límites actuales

- El módulo contiene el motor de ejecución del dominio y una cola acotada en
  memoria; todavía no constituye un registro persistente de comandos.
- Los datos serializables de las tomas y eventos MIDI viven en
  `estudio-daw-midi-types`, sin dependencia de ALSA. La captura, descubrimiento
  y salida MIDI pertenecen a `estudio-daw-midi-engine` y sus adaptadores del
  sistema.
- El modelo portable no depende de PipeWire, ALSA, el motor de audio ni los
  diagnósticos de dispositivos. La inspección de archivos, firmas/hash y
  generación/validación de proxies con `ffmpeg`/`ffprobe` pertenecen a
  `estudio-daw-media-adapter`; esos procesos no se ejecutan desde una mutación
  del modelo ni desde la llamada de retorno de audio.
- El adaptador de medios puede generar archivos derivados y actualizar su
  manifiesto, pero no modifica el proyecto. La asociación del proxy se solicita
  mediante `ProjectCommand::SetAudioSourceProxy`; el bus comprueba su
  procedencia y ofrece deshacer/rehacer.
- `estudio-daw-application` es la fachada de ciclo de vida: mantiene vivo el
  motor para que una sesión de interfaz pueda encadenar comandos y deshacer o
  rehacer, y publica eventos e instantáneas al abrir o guardar un proyecto. La
  persistencia guarda el estado resultante, no serializa la pila del historial
  entre cierres.
- La interfaz de línea de comandos está aislada en `estudio-daw-cli`, por fuera del modelo portable. Sus
  comandos `attach-take`, `quantize`, `attach-media` y `add-audio-clip` delegan
  ahora en la misma API de aplicación; la interoperabilidad DAWproject y las
  operaciones de dispositivos también se atienden como adaptadores porque
  coordinan formatos, archivos o servicios del sistema.
- La API de comandos se ampliará según las tareas aprobadas; no implica que toda
  mutación existente ya esté migrada.
- La llamada de retorno de tiempo real no envía comandos a esta cola ni ejecuta
  `drain_into`.

## Verificación

```bash
cargo test -p estudio-daw-command-bus
```

Las pruebas cubren coordinación de sesión/proyecto, precondiciones obsoletas,
serialización, atribución de eventos, cuantización que conserva controladores,
duplicación reversible de pistas MIDI con sus clips/casillas, cambios
reversibles de escenas, casillas, estado de pista y posición de audio,
preservación de la fuente al mover regiones, asociación reversible de proxy con
verificación de procedencia y rechazo de casillas con tipos de pista/clip
incompatibles y de fuentes dirigidas a pistas MIDI.
