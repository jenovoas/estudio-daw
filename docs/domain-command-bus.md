# Bus de comandos del dominio

`estudio-daw-command-bus` coordina las mutaciones del proyecto y del transporte
desde una frontera portable. La UI, CLI, MIDI, scripting y agentes pueden emitir
el mismo `CommandEnvelope`; no necesitan conocer los detalles internos de
`ProjectHistory` ni de `Session`.

## Flujo

```text
CommandEnvelope
  → DomainCommandBus bounded
  → validar id, versión y precondiciones
  → aplicar a Session o ProjectHistory
  → DomainEvent atribuido al autor
  → snapshot consultable por la UI
```

El productor usa `try_send`; una cola llena devuelve un error en vez de bloquear.
El consumidor drena comandos fuera del callback de audio. Este bus de dominio es
distinto de `estudio_daw_session::CommandBus`, que sigue siendo la cola live
pequeña para el transporte MIDI.

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

- transporte: los `SessionCommand` existentes;
- audio: agregar, recortar, ajustar ganancia y fades de clips;
- medios: asociar una fuente original/proxy a una pista de audio;
- MIDI: adjuntar una toma y cuantizar Note On/Off de un clip;
- historial: undo y redo de transacciones del proyecto.

Las operaciones MIDI reutilizan las funciones del modelo. Cuantizar mantiene
intactos los controladores y queda cubierto por el mismo historial reversible.
Las mutaciones del proyecto generan `ProjectEvent` dentro de un
`DomainEventPayload::ProjectChanged`.

## Límites actuales

- El crate tiene el runtime de dominio y una cola bounded en memoria; todavía no
  es un log durable de comandos.
- La CLI está aislada en `estudio-daw-cli`, por fuera del modelo portable. Sus
  comandos `attach-take`, `quantize`, `attach-media` y `add-audio-clip` ya se
  enrutan mediante `DomainCommandBus`; import/export, generación de proxies y
  operaciones de dispositivos siguen siendo adaptadores directos porque también
  coordinan archivos o servicios del sistema.
- La API de comandos se ampliará según las tareas aprobadas; no implica que toda
  mutación existente ya esté migrada.
- El callback RT no envía comandos a esta cola ni ejecuta `drain_into`.

## Verificación

```bash
cargo test -p estudio-daw-command-bus
```

Las pruebas cubren coordinación de sesión/proyecto, precondiciones obsoletas,
serialización, atribución de eventos, cuantización que conserva controladores y
undo de mutaciones MIDI y medios, además del rechazo de fuentes dirigidas a
pistas MIDI.
