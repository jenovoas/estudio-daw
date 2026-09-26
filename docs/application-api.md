# API de aplicación y ciclo de proyecto

`estudio-daw-application` ofrece una fachada estable entre UI/CLI y el modelo.
La capa conserva una instancia de `CommandRuntime` y `DomainCommandBus` mientras
el proyecto está abierto; por ello una interfaz mantiene el historial de undo y
redo entre acciones, en lugar de reconstruir el runtime por cada comando.

## Flujo de sesión

```text
open(project.json) → ProjectApplication viva
                         ├── execute / dispatch → eventos atribuidos
                         ├── snapshot → revisión + estado para las vistas
                         ├── undo / redo → historial de esta sesión
                         └── save / save_as → JSON por reemplazo atómico
```

Las mutaciones se ejecutan fuera del callback de audio. `dispatch` acepta un
`CommandEnvelope` completo para conservar autor, id y precondiciones; `execute`
crea el id y captura la revisión actual como precondición. Cada llamada devuelve
los `DomainEvent` producidos. Un comando rechazado devuelve un diagnóstico
estructurado y deja el snapshot intacto.

## Persistencia e historial

- `open` reconstruye el modelo desde JSON y comienza una sesión de historial nueva.
- El transporte inicia con el tempo del proyecto; al guardar, el tempo actual del
  transporte se sincroniza de nuevo con `Project.transport`.
- `save` escribe sobre la ruta abierta; `save_as` guarda y asocia una ruta nueva.
- `save_to` permite a adaptadores como la CLI escribir una salida explícita sin
  cambiar la ruta actual.
- El reemplazo escribe un temporal único junto al destino, sincroniza sus bytes y
  lo renombra al destino; un fallo previo al renombrado conserva el archivo
  anterior.
- El JSON conserva el estado musical actual, no el stack de undo/redo. Undo/redo
  opera durante la sesión abierta; después de guardar y reabrir, el último estado
  guardado es la nueva base del historial.
- `history_state()` permite que la UI habilite sus acciones Undo/Redo sin acceder
  al `ProjectHistory` interno.

## Verificación

```bash
cargo test -p estudio-daw-application
```

La prueba vertical abre una fixture JSON, aplica un cambio, lo deshace, lo rehace,
ajusta el tempo live, guarda sobre la ruta abierta y vuelve a abrir el archivo.
Otra prueba verifica que un comando rechazado no altera el snapshot ni habilita
undo.
