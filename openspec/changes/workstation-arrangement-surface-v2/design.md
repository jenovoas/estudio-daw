# Diseño

## Contexto

Este cambio corrige una pasada anterior de interfaz que trató un DAW como un panel oscuro más una línea de tiempo de sólo lectura. Aquella pasada no satisfizo el flujo creativo solicitado de Ableton Live, no entregó un sistema utilizable de pistas de audio y marcó tareas como completas antes de tiempo. Esas tareas se reabren aquí. La referencia se actualizó el 2026-09-27 contra el manual oficial de Live 12 (PDF del 2026-04-30), las notas oficiales vigentes de Live 12.4.6 (2026-09-15) y la ayuda oficial en español. Este documento registra vistas, menús y ámbitos de opciones de referencia; no afirma paridad con Live. La tarea 0.2 se cerró con el cotejo documental de las opciones estáticas pertinentes; las variantes por plataforma, edición, selección y dispositivo se identifican como condicionales.

La implementación actual es una ventana Tauri con apertura/guardado de proyectos, reproducción MIDI/audio, transporte con posición basada en frames procesados, historial y ajustes de perfil de audio. La reproducción decodifica regiones en workers y las mezcla desde rings acotados; permite iniciar y buscar desde el cursor de Arreglo, reconstruir estados MIDI dentro de clips y coordinar el rango A/B durante Play. El runtime recompila el plan activo al cambiar regiones o mezcla. La captura de entrada por pista graba las pistas armadas a WAV y añade regiones al detener, pendiente de QA física. El ruteo interno de pista a bus/audio y de éstos a Master se procesa con un nodo compuesto y scratch por pista reservado antes del stream; permanecen incompletos el ruteo de salidas físicas por pista, envíos, retornos y procesamiento por complementos. Session muestra una matriz editable de escenas/casillas para clips existentes, con lanzamiento MIDI/audio cuantizado y sustitución por pista; «Volver a Arreglo» recompila el material del arreglo en el tick actual sin cerrar PipeWire. Sigue pendiente capturar la interpretación Session como material de Arrangement. El código fuente y las comprobaciones ejecutables, no este documento, determinan las capacidades presentes.

### Revisión correctiva de la composición visual — 2026-09-29

El usuario observó que el corte anterior sólo produjo un cambio mínimo y volvió
a priorizar el flujo creativo de Ableton Live. Esta revisión hace un cambio de
composición visible: transporte, posición, historial, vistas, mezclador y
herramientas de edición se reúnen en una barra de control; el navegador izquierdo
tiene secciones funcionales Proyecto y Crear; esta última reutiliza acciones
existentes para añadir pistas MIDI/audio y buses. Arrangement/Session siguen
ocupando el lienzo principal y el detalle contextual/Mezclador quedan abajo. No
declara terminadas categorías de biblioteca ni añade contenido ficticio.
La persona usuaria reporta que este corte sólo mejora un poco el orden y que ni
se acerca al objetivo. Es una corrección de rumbo: la solución actual no está
aceptada. No continuar funciones hasta hacer un rediseño visual sustancial y
recibir feedback explícito. OpenSpec 3.1 permanece abierta; la revisión en runtime
con proyecto cargado y tamaños de ventana normal/estrecho vendrá después del
rediseño, no sustituye su trabajo.

## Identidad musical y complementos de pista

La identidad de la pista es información de composición elegida por la persona, no
una etiqueta de formato. `Track.color`, una marca breve (`Track.marker`) y una
nota descriptiva (`Track.annotation`) se guardan con la pista. `SetTrackIdentity`
actualiza los tres valores como una transacción reversible. Al leer proyectos
anteriores, los campos ausentes reciben color predeterminado, marca vacía y nota
vacía. La interfaz ofrece un editor compacto desde el encabezado y conserva el
color/marca en Arreglo, Sesión, Mezclador y piano roll; la nota se muestra al
volver a editar la identidad.

Analog Lab y sus VST instalados en Wine son un objetivo explícito de integración
en pista. La persona usuaria confirma que la aplicación independiente funciona
en Arch mediante Wine sin latencia perceptible y aclara que también tiene los
VST instalados en Wine. Estudio DAW debe ofrecer las dos formas reales de uso:
cargar el VST en el flujo de instrumento de una pista, y abrir/reutilizar la
aplicación independiente como instrumento externo. No se considera completa la
integración si sólo ofrece una etiqueta/preset, si exige reemplazar el flujo
standalone que ya funciona o si deja el alojamiento VST para un futuro indefinido.

En este equipo se encontraron Wine, `Analog Lab V.exe`, el bundle Windows
`Analog Lab V.vst3` en `~/.wine/drive_c/Program Files/Common Files/VST3` y PipeWire
con puente MIDI ALSA. Se configuró yabridge 5.1.1 para cuatro plugins VST3 de
Arturia, y Carla 2.5.10 cargó Analog Lab V real mediante JACK/PipeWire. Esto
valida el stack local de compatibilidad, no su integración en pista dentro de
Estudio DAW. [yabridge](https://github.com/robbert-vdh/yabridge)
es el bridge comprobado en este equipo para exponer VST Windows a hosts Linux;
siguen pendientes instancia/estado/MIDI/audio/GUI dentro del host del DAW. Su
[documentación de arquitectura](https://github.com/robbert-vdh/yabridge/blob/master/docs/architecture.md)
describe la comunicación entre el host Linux y el proceso Wine, incluidos los
buffers de audio compartidos; el adaptador debe respetar esos límites de tiempo
real en el motor de Estudio DAW.

La configuración de proyecto debe distinguir una instancia VST alojada en la
pista de una aplicación standalone asociada a puertos MIDI/audio. Ambas guardan
identidad y estado recuperable; ambas conservan una GUI real y muestran fallos
sin sustituciones silenciosas. Lanzamiento, descubrimiento de puertos,
serialización y bridge quedan fuera del callback. La ruta VST bridged debe
procesarse en un worker desacoplado con buffers acotados si el IPC del host lo
requiere; se medirán latencia y estabilidad antes de integrarla al plan de audio.
La latencia perceptible ausente del flujo standalone actual se registra como
requisito de experiencia de la persona usuaria, no como medida de la ruta VST aún
no integrada.

El modelo portable identifica una instancia VST3 con formato, ruta descubierta,
ID estable del plugin, bridge y una referencia relativa/hash del estado. Para
Standalone guarda ejecutable/prefijo Wine, puerto de salida MIDI y nodo/canales
de retorno PipeWire. `SetTrackInstrument` cambia esa asignación mediante el bus
reversible. La primera ruta ejecutable envía eventos del scheduler desde un puerto ALSA
por worker y enlaza ese origen a la entrada MIDI PipeWire elegida mediante
`pw-link`, sin invocar ALSA ni crear enlaces desde el callback. La cola MIDI tiene
capacidad acotada; si se llena, sólo se detiene brevemente el scheduler para
conservar eventos Note Off, nunca el callback de audio. Captura el nodo de retorno
por la entrada PipeWire existente hacia la pista, el ruteo interno y Master. La
disponibilidad del puerto/nodo se comprueba al iniciar. La aplicación reutiliza
el proceso gestionado si coinciden ejecutable y prefijo Wine; si esa pista ya
tiene otro ejecutable activo, solicita cerrarlo antes de cambiar la asignación.
El flujo de pista ya permite elegir/abrir el ejecutable standalone y seleccionar
puertos MIDI PipeWire y retorno; Play abre/reutiliza el proceso configurado. El
alojamiento VST3 ya se conectó al motor en un worker de proceso aislado, con MIDI
desde el scheduler y un ring PCM previo a la mezcla por pista. El helper debe
construirse y acompañar al ejecutable. La GUI nativa se abre por XWayland en la
sesión Hyprland. Guardar o detener con el motor conectado escribe el estado
binario en `plugin-state/` junto al proyecto y `SetTrackInstrument` guarda la
referencia relativa; Play restaura ese blob o informa el archivo ausente sin
sustituir el instrumento. Aún faltan QA de reproducción y preset en Tauri con
Analog Lab, y comprobar recuperación/cierre. La ruta standalone tampoco se ha
verificado aún con Analog Lab real en esta versión. Cada backend incompleto
informa el error y no sustituye el instrumento silenciosamente.

### Verificación aislada VST3 — 2026-09-28

El host `vst3-host` 0.9.0 sin modificar no pudo inspeccionar el bundle yabridge porque Analog Lab expone una clase `Plugin Compatibility Class` cuya creación de `IPluginCompatibility` devuelve `0x3`. En una copia temporal del crate con esa interfaz opcional tratada como lista de compatibilidad vacía, el probe enumeró Analog Lab V 5.12.5.6878 (Arturia, MIDI-in, una salida estéreo, GUI disponible). Después, el host aislado cargó el VST3 real, envió MIDI Note On 60 y renderizó audio a 48 kHz/512 frames; el pico observado fue 0,2568381. La corrección acotada de `IPluginCompatibility` y el helper upstream MIT se integraron en `crates/vst3-host`; `audio_runtime` inicia el host fuera del callback, mantiene un prebúfer de dos bloques y entrega el PCM por ring a la pista. El producto aún no se ha abierto en Tauri para verificar la reproducción integrada. La prueba aislada no cubre QA Tauri, recuperación de crash ni latencia de ida y vuelta. El helper Linux ya abre GUI por XWayland y persiste el estado binario al guardar o detener.

## Auditoría de referencia: Ableton Live 12.4.6

El diseño toma de Live el flujo creativo y la jerarquía de superficies, sin copiar recursos gráficos ni marca protegidos.

| Superficie / menú | Opciones de referencia y comportamiento que debemos considerar |
| --- | --- |
| **Barra de control / Transporte** | Reproducir/detener/grabar, tempo y compás, metrónomo, cuantización global de lanzamiento, cambio entre las vistas Sesión y Arreglo, estado/uso de CPU y visibilidad de vistas/navegador. Mantiene accesibles los controles frecuentes y deja espacio para editar. |
| **Navegador** | Búsqueda; historial atrás/adelante; colecciones; categorías de biblioteca: Todo, Sonidos, Baterías, Instrumentos, Efectos de audio, Efectos MIDI, Moduladores, Max for Live, Complementos, Clips, Muestras, Grooves y Plantillas; ubicaciones: Proyecto actual, Biblioteca de usuario y carpetas del usuario; filtros/etiquetas, escucha previa y arrastre al conjunto. Los paquetes e integraciones externos sólo aparecen cuando existen. |
| **Vista Session** | Columnas de pistas × filas de escenas; lanzamiento/parada/selección de clips por casilla y estado reproduciendo/en cola; lanzamiento de escena entre pistas; casillas vacías; un clip activo por pista; cuantización, modo de lanzamiento, legato, velocidad, desplazamiento/empujón de clip, bucle y acciones posteriores; sección de mezclador y controles de escena. En su pista, un clip Session tiene precedencia sobre la reproducción de Arrangement. |
| **Vista Arrangement** | Las mismas pistas en vertical contra compases/pulsos; formas de onda y regiones MIDI; mover/cambiar tamaño/dividir/duplicar clips; bucle y selección temporal; carriles de tomas y compilación; controles de pista; reglas/marcadores; mezclador. Cambiar de vista conserva reproducción e identidad compartida de pistas. Estado actual parcial: reglas/cursor/rango, selección, waveform y edición reversible de audio; clips MIDI se mueven, duplican y cuantizan mediante comandos, y su división desde el cursor cierra/rearticula notas y restaura CC, pitch bend, presión y programa. No se reemite SysEx previo; falta QA visual y edición de notas. |
| **Detalle de clip/dispositivo** | El panel inferior contextual sigue la selección. Los clips MIDI ofrecen edición de notas y controles de tiempo/bucle; los clips de audio, muestra/forma de onda y deformación temporal; los dispositivos, sus parámetros. El panel cambia con la selección y no se queda como tarjeta genérica de ajustes. |
| **Mezclador / controles de pista** | Valores compartidos entre Session y Arrangement; activador, solo, armado de grabación si está admitido, panorama, volumen, medidores, entrada/salida y controles configurables del mezclador. Envíos/retornos/master y escucha de referencia son conceptos reales de señal, no botones decorativos. |
| **Menús de aplicación** | Archivo, Edición, Crear, Vista, Reproducción, Navegación y Ayuda; Opciones en Windows y el menú de la aplicación Live en macOS. La ubicación y disponibilidad cambian por plataforma, edición, contexto y versión. Los ajustes de macOS se abren desde Live; en Windows, desde Opciones. No tratar ambas barras de menú como idénticas. |

### Inventario de menús y acciones contextuales de Ableton Live

Este inventario describe la aplicación principal y las superficies creativas. La base es Live 12.4.6: el manual oficial de referencia publicado el 2026-04-30, las notas de versiones hasta 12.4.6, la comparación oficial de ediciones y guías oficiales en español. Incluye comandos pertinentes a creación de proyectos, interpretación, edición, mezcla y gestión de medios. Los menús de dispositivos y complementos dependen del elemento cargado y del fabricante; se registran las opciones descritas oficialmente y se identifica cuándo el contenido cambia con el dispositivo, sin presentar esa variación como una lista fija. La verificación se limita deliberadamente a lo que publica el fabricante; no requiere ejecutar Live en Linux.

| Menú / contexto | Opciones de Live 12 observadas en la referencia oficial | Cobertura prevista para Estudio DAW |
| --- | --- | --- |
| **Archivo** | Crear conjunto; abrir y abrir recientes; guardar, guardar como y guardar copia; guardar como conjunto predeterminado o plantilla; administrar archivos del conjunto/proyecto/biblioteca; buscar y volver a enlazar archivos ausentes; reunir archivos externos y guardar; revisar/eliminar archivos sin uso; empaquetar un proyecto; instalar un paquete; exportar audio/vídeo o un clip MIDI; cerrar conjunto y salir. | Crear/abrir/recientes/guardar/guardar como/plantilla; reunir y volver a enlazar medios; exportar audio/pistas separadas/MIDI; cierre/salida seguros. |
| **Edición** | Deshacer/rehacer; cortar/copiar/pegar/duplicar/eliminar; renombrar y editar texto informativo; seleccionar todo/quitar selección/invertir selección; seleccionar el bucle; volver al valor predeterminado; cuantizar; dividir/consolidar según contexto; establecer marcadores de inicio/final/bucle en clips MIDI; duplicar el bucle; operaciones de notas y envolventes según editor/selección; congelar o descongelar pista y, en versiones recientes, rebotar/consolidar a audio. | Bus de comandos reversible para mutaciones; modelo de selección/portapapeles; edición MIDI/audio; disponibilidad contextual e historial de edición. |
| **Crear** | Insertar pista de audio/MIDI/retorno; insertar escena o escena capturada; insertar clip MIDI vacío; importar archivo MIDI; consolidar el intervalo seleccionado en una escena; crear/eliminar localizador; insertar o eliminar transitorios en el editor de muestra; crear desvanecimientos/cruces; convertir audio a MIDI según material y edición. | Crear pistas/escenas/clips tipados; capturar material de sesión activo sin interrumpirlo; importar MIDI/audio a pista/casilla elegida; convertir selecciones temporales de Arrangement en escenas. |
| **Vista** | Cambiar entre Sesión/Arreglo; mostrar/ocultar navegador, vista de aprendizaje, vista de información, vista de clip, vista de dispositivo, mezclador y envolventes; abrir simultáneamente las vistas de clip y dispositivo; configurar secciones del mezclador: entrada/salida, envíos, retornos, volumen, opciones/retardo de pista, fundido cruzado e impacto de rendimiento; mostrar controles de pista del Arreglo; mostrar colección de grooves/afinaciones, historial de deshacer, administrador de archivos y pantalla completa; acercar el clip o la selección. Algunas opciones dependen de edición, versión y selección. | Cambio inmediato entre vistas; paneles opcionales persistentes de navegador/detalle/mezclador; canales configurables y encabezados; acercamiento/plegado/resumen; preferencias de accesibilidad y distribución guardada. |
| **Reproducción** | Menú principal documentado para acciones de reproducción y grabación. Las notas de versión nombran «Mover marcador de inserción al cabezal». Sólo se atribuyen al menú los comandos que aparecen en las fuentes oficiales; las opciones no enumeradas no se infieren. | Controles de reproducir/pausar/detener/grabar, salto/navegación y funciones realmente implementadas; documentar por separado acciones aún sin conexión. |
| **Opciones (Windows) / menú Live (macOS)** | Teclado MIDI de computadora; asignación de teclas/MIDI; modo de dibujo; ajuste a rejilla; seguir reproducción; perseguir notas MIDI; compensación de latencia; bloqueo de envolventes; armado/reactivación de automatización; entrada por pasos; preescucha de navegador y notas MIDI; funciones de accesibilidad. Live 12.4 añade modo de desarrollo de Max for Live. La ventana Ajustes se abre desde Opciones en Windows y desde Live en macOS (atajo Ctrl+, / Cmd+,). | Dispositivo/frecuencia/búfer de audio de ámbito de aplicación, puertos/asignación MIDI, pantalla/tema/teclado, rutas de medios/biblioteca, comportamiento de lanzamiento/grabación/deformación y accesibilidad. Distinguir ajustes aplicados de los que requieren reinicio. |
| **Páginas de Ajustes** | Pantalla y entrada (idioma, zoom, barras de desplazamiento, foco, seguimiento de Arreglo/clips, etiquetas, navegación, ratón/lápiz y restaurar avisos); Tema y colores (claro/oscuro/seguir sistema, tono, contraste, brillo, cuadrícula, matiz e intensidad, colores automáticos de pista/clip); Audio (controlador/dispositivos y canales de entrada/salida, frecuencia, latencia, prueba/calibración); Link y Link Audio; Tempo y MIDI (Tempo Follower, sincronización/re-sincronización, superficies de control, puertos y asignaciones); Archivo y carpeta (archivos de análisis, editor externo, carpeta temporal, rutas/cache de decodificación y Max); Biblioteca (ubicaciones de Packs/biblioteca/usuario, copiar dependencias y visibilidad de Cloud/Push/Splice); Complementos (rutas, carpetas habilitadas y ventanas); Grabación, deformación y lanzamiento (formato de grabación, Warp/Fade y ajustes por defecto de selección/lanzamiento/grabación); Licencias y actualizaciones (autorización, actualización automática y datos de uso). | Guardar preferencias globales fuera del proyecto cuando corresponda; exponer sólo ajustes disponibles para el motor local y su estado efectivo. Cloud, Push, Splice, Packs/Max y licencias de Ableton quedan excluidos como integraciones o servicios propios (no son comandos de Estudio DAW); cualquier función genérica equivalente se implementa sólo si tiene una tarea explícita. |
| **Navegación** | Cambiar el foco entre barra de control, Sesión, Arreglo, vista de clip, vista de dispositivo, navegador, colección de grooves, vista de aprendizaje, mezclador y el mismo control de pistas vecinas; activar Tab para mover el foco, navegación circular y movimiento de clips con flechas. Incluye navegación por ajustes, controles de una fila, escenas y encabezados; atajos y modificadores varían por plataforma. | Navegar entre vistas y controles con atajos visibles; no interceptar escritura en campos; foco accesible y comportamiento documentado. |
| **Ayuda** | Vista de aprendizaje (lecciones y progreso) e información contextual; abrir el manual correspondiente a la versión y recursos/base de conocimiento; información de paquetes; cargar el conjunto de demostración (documentado desde Live 12.3). En Live 12.4 Learn View reemplaza la antigua vista Ayuda. «Acerca de» y salir pertenecen al menú de aplicación de macOS cuando lo determine el sistema. | Ayuda contextual, atajos/referencia, diagnósticos/registros del motor e información del producto; enlaces y contenido sólo cuando estén disponibles. No se promete catálogo de Packs de Ableton. |
| **Contexto del encabezado de pista** | Renombrar/información/color; insertar, duplicar, eliminar, desactivar, congelar, plegar/desplegar; ajustar alto; agrupar/vincular; carriles de tomas; armar/solo/silenciar; mostrar controles de mezclador y acciones de dispositivo/ruteo según pista. | Agregar/duplicar/renombrar/reordenar/quitar/activar, tipo/color, plegado/alto, grupo y mezcla/grabación admitidas; filtrar según tipo y rutas disponibles. |
| **Contexto de casilla/escena Session** | Lanzar/detener/seleccionar clip; insertar/capturar escena; renombrar y elegir color; duplicar/copiar/pegar/eliminar; modo de lanzamiento, legato, velocidad, cuantización, desplazamiento, bucle y acciones posteriores; lanzar escena y ajustar tempo/compás; añadir/quitar botones de parada. | Funciones creativas con estado persistido, sustitución por pista, lanzamiento de escena y reproducción cuantizada real; hacer visible cada estado de lanzamiento. |
| **Contexto de clip/selección Arrangement** | Seleccionar/dividir/duplicar/mover/cambiar tamaño/recortar/invertir; bucle; consolidar; desvanecimientos/ganancia; controles de deformación/muestra; recortar/insertar/borrar tiempo; acciones de selección/rango; notas MIDI y automatización. | Edición MIDI/audio no destructiva, forma de onda/notas, desplazamientos de fuente, desvanecimientos, ajuste y edición reversible. Omitir transformaciones no admitidas. |
| **Contexto de elemento del navegador** | Cargar/insertar/escuchar; navegar atrás/adelante; buscar/filtrar/etiquetar/coleccionar; renombrar/quitar cuando corresponda; mostrar extensiones y elegir columnas; sustituir/localizar medios ausentes; administrar proyecto/biblioteca y reunir archivos. | Búsqueda/ubicaciones/favoritos/filtros, escucha veraz, arrastrar y soltar, procedencia, revinculación y diagnóstico. Mostrar contenido licenciado instalado sólo tras descubrirlo mediante una integración real. |
| **Controles de mezclador/pista** | Activador de pista, solo/escucha de referencia, armado, panorama, volumen/medidor; selectores de entrada/salida; envíos/retornos/salida principal; retardo de pista; asignación/curva de fundido cruzado; indicadores de rendimiento; mostrar/ocultar secciones configurables. | Los mismos controles centrales en Session y Arrangement sobre valores compartidos. Habilitar ruteo/escucha/envíos/retardo/complementos/grabación sólo con soporte real de comandos y motor. |
| **Contexto del editor de notas MIDI** | Seleccionar/mover/cambiar tamaño/velocidad/probabilidad de nota; cuantizar/rejilla; agrupar/reproducir una nota; herramientas de transposición/transformación/generación; marcadores de inicio/final/bucle de clip; preescucha de nota y modos de envolventes/MPE. | Primero las funciones básicas de piano roll y luego expresión/transformaciones admitidas; las ediciones se guardan y se deshacen. Los generadores avanzados se especifican por separado. |
| **Contexto de dispositivo/complemento** | Mostrar/ocultar ventana del complemento; cargar/sustituir/preescuchar preajustes; agrupar/desagrupar; comparar/alternar estados A/B; editar en Max y modo de desarrollo de Max for Live; comandos propios de cada instrumento/efecto. Las acciones dependen del dispositivo, fabricante, foco y edición Suite. | Mostrar sólo operaciones del dispositivo cargado y conectadas; controlar selección, parámetros, derivación, edición y deshacer mediante contratos tipados. No inventar funciones de fabricante. |
| **Contexto de navegador** | Abrir/cargar, escucha previa, atrás/adelante; búsqueda, categorías/colecciones/filtros/etiquetas; mostrar/ocultar grupos de filtros y editor de etiquetas; etiquetas rápidas (crear/añadir/quitar etiquetas o grupos en 12.4); reorganizar/ocultar etiquetas; mostrar extensiones y personalizar columnas; opciones por tipo de elemento, paquete y ubicación. | Búsqueda/ubicaciones/favoritos/filtros, escucha veraz, arrastrar y soltar, procedencia, revinculación y diagnóstico. Mostrar contenido licenciado instalado sólo tras descubrirlo mediante una integración real. |

### Cotejo de familias de opciones contra el catálogo de Estudio DAW

Este cotejo usa la matriz de capacidades que sigue, los comandos existentes del modelo/bus y las tareas de implementación. «Existe en el modelo» no significa que haya una opción gráfica utilizable. La columna de alcance identifica el trabajo previsto; no afirma que exista un registro de comandos de interfaz, que todavía no está implementado.

| Familia de Live | Correspondencia en Estudio DAW | Estado observado y tarea relacionada |
| --- | --- | --- |
| Archivo: nuevo/abrir/guardar/guardar como | Comandos de proyecto y API de aplicación | Hay flujo Tauri parcial para esas acciones; conjuntos recientes, plantillas, copias, administración/recopilación de medios, paquetes y exportación no forman un flujo de menú completo. Tareas 2.2 y 5.2. |
| Edición: deshacer/rehacer y operaciones de selección/portapapeles | Historial de aplicación y `ProjectCommand` | Deshacer/rehacer existe para el subconjunto conectado; no hay selección/portapapeles generales, editor de notas o conjunto de acciones contextuales. Tareas 3.3, 3.5, 5.1 y 5.3. |
| Crear: pistas, escenas, casillas, clips, marcadores/importación | `ProjectCommand` y tipos persistidos | Alta de pista de audio está conectada desde Tauri; persiste canales y destino interno al Master (creado en la misma transacción si falta). Importación/edición están en 2.2 y la reproducción incremental inicial en 2.3; siguen pendientes capacidades adicionales del transporte y mezcla. Tareas 2.2–2.3, 3.2, 5.1 y 5.4. |
| Vista: Sesión/Arreglo, navegador, editores y mezclador | `UiSnapshot` compartida por las tres superficies de pistas | Arreglo muestra pistas/cursor; Session muestra columnas de pistas y filas de escenas, con asignación reversible de clips existentes y controles de escena; sus lanzamientos siguen deshabilitados hasta conectar el planificador. Las superficies ofrecen ACT/M/S, ganancia/panorama y medidores de pico/RMS de bloque para pistas no master. El Mezclador añade un canal Master con medidor de salida final; en un Master persistido, ACT/silencio/ganancia procesan la suma antes del medidor. La selección de pistas se comparte y sus grupos organizativos persisten sin enlazar la mezcla. Faltan navegador creativo, lanzamiento y edición contextual. Tareas 3.1–3.6, 4.1–4.2 y 5.5. |
| Reproducción: transporte y acciones de reproducción/grabación | Comandos de transporte del shell, reloj por frames procesados, planificador MIDI y decodificador incremental de audio | Play/Pause/Stop reproduce clips MIDI y regiones de audio; Play desde detenido usa el cursor de Arreglo y un clic en la regla durante Play reubica el plan sin cerrar PipeWire. Arreglo muestra la posición del plan. Loop A/B coordina planes en límites de bloque; el pánico MIDI y el metrónomo tienen controles reales. Record captura las pistas armadas a WAV en `media/recordings/` y Stop añade regiones reversibles. No hay matriz de lanzamiento Session, cuenta previa, tomas por secciones ni sincronía sample-accurate del scheduler MIDI. Tareas 2.3, 2.5, 4.3–4.4 y 5.5. |
| Opciones/Ajustes: audio, MIDI, pantalla, biblioteca y accesibilidad | Perfil de audio persistido y adaptadores de plataforma | El perfil puede guardarse desde el shell; la selección efectiva de dispositivo/período, asignaciones MIDI y demás páginas no están expuestas como un sistema íntegro de ajustes. Tareas 4.3, 5.1 y 5.6. |
| Navegación/Ayuda: foco, atajos, lecciones y diagnóstico | Registro frontend parcial para menús y atajos iniciales; no hay panel de ayuda | Los menús Proyecto/Edición/Crear/Sesión/Vista/Transporte y Ctrl+1/2/3/S/Z usan `UI_ACTIONS`; Proyecto abre importación de audio y Sesión añade escena. No hay atajos reasignables, ayuda contextual ni aprendizaje dentro de la interfaz. Tareas 5.1, 5.5–5.7. |
| Menús contextuales de pista/clip/escena/navegador/dispositivo | Registro frontend parcial, filtrado por tipo y manejadores UI existentes | Tauri ofrece seleccionar y cuantizar clips MIDI a la rejilla actual; seleccionar/preescuchar/quitar regiones de audio; y duplicar/mover/quitar pistas cuando el control correspondiente está disponible. Escena, navegador y dispositivo aún no tienen menús contextuales; las acciones reutilizan comandos/adaptadores existentes. Tareas 3.2–3.6 y 5.1–5.7. |

#### Trazabilidad de las opciones estáticas publicadas

La matriz siguiente cierra el cotejo documental solicitado por 0.2: todas las familias y opciones estáticas pertinentes recogidas arriba tienen una acción/comando/tarea destino o una exclusión justificada. La correspondencia es de hoja de ruta, no de implementación ni de paridad. Las opciones que sólo aparecen al seleccionar una entidad se cubren en la fila de contexto correspondiente; no se pretende enumerar cada comando de cada dispositivo/plugin.

| Familia/opciones documentadas | Destino en Estudio DAW | Tratamiento / exclusión |
| --- | --- | --- |
| Archivo/Sesión: crear, abrir, recientes, guardar/copia/plantilla, renombrar/cerrar, gestión y reparación de medios, recopilar/limpiar/empaquetar, importar MIDI, instalar paquete y exportar audio/vídeo/MIDI/stems | API de proyecto y medios; tareas 2.2, 5.2, 6.4 | Cada operación se incorpora al menú sólo al tener flujo real; paquetes de Ableton y vídeo quedan fuera salvo especificación futura. |
| Edición y contexto de clip/Arreglo: historial, selección/portapapeles, duplicar/quitar, texto informativo, bucle/marcadores, cuantizar, tiempo, división/consolidación, desvanecimientos/ganancia, Warp/muestra, edición multi-clip y automatización | Comandos reversibles y editores; tareas 3.3, 3.5, 5.1, 5.3, 6.1 | Transformaciones/destructivas se omiten hasta tener comando, procedencia y deshacer/rehacer; opciones de Warp dependen de motor/formato. |
| Crear: pistas, escenas/captura, clips MIDI vacíos, MIDI desde archivo, localizadores, transitorios/fades y conversiones de audio a MIDI | Modelo/comandos y flujo de medios; tareas 2.1–2.3, 3.2–3.4, 5.4 | Crear una entidad en el modelo no basta para habilitar una opción gráfica. Conversiones y análisis requieren tareas de producto explícitas. |
| Vista: Session/Arrangement, navegador, detalle clip/dispositivo, mezclador y secciones, controles de pista, historial/gestión de archivos, grooves/afinaciones y pantalla completa | Superficies compartidas y preferencias de presentación; tareas 3.1–3.6, 4.1–4.2, 5.5–5.6 | Mostrar u ocultar una superficie es acción de vista; secciones inexistentes se omiten, no se simulan. Grooves/afinaciones dependen de futuras capacidades musicales. |
| Reproducción: transporte, grabación, mover cursor/cabezal, grabación por secciones y comportamiento asociado | Reloj de sesión y planificador; tareas 2.5, 4.3–4.4, 5.5 | Sólo controles que actúan en el motor; selección/grabación deshabilitada u omitida mientras no exista ruta completa. |
| Opciones/Live: teclado MIDI de computadora, mapeos de teclas/MIDI, modo de dibujo, rejilla, seguimiento, persecución MIDI, compensación de latencia, automatización, entrada por pasos, preescucha, accesibilidad y desarrollo Max | Ajustes/acciones del motor y de interfaz; tareas 4.3–4.5, 5.1, 5.6–5.7 | El modo de desarrollo y opciones propias de Max for Live se excluyen como integración de fabricante; accesibilidad nativa sí se considera. |
| Ajustes: pantalla/tema/audio/Link/Tempo-MIDI/archivos/biblioteca/plugins/registro-Warp-lanzamiento/licencias-actualizaciones | Preferencias de aplicación, backend de audio/MIDI y ajustes de proyecto; tareas 2.2, 2.4–2.5, 4.3–4.5, 5.6 | Link Audio, controladores y plugins sólo se ofrecen con soporte de extremo a extremo. Rutas de Ableton, autorización/actualización de Ableton y servicios Cloud/Push/Splice son específicos del fabricante y quedan excluidos. |
| Navegación/Ayuda: foco, vecinos, Tab, escenas, manual, lecciones, información contextual, recursos y conjunto de demostración | Foco/atajos y ayuda del producto; tareas 5.1, 5.5–5.7 | Ayuda, demo y diagnósticos corresponden al producto propio; no se carga contenido de Ableton ni se afirma equivalencia de sus lecciones. |
| Contextos de pista, escena/casilla, navegador, mezclador y dispositivo/plugin | Registro de acciones contextual, filtrado por entidad/capacidad y adaptadores; tareas 3.2–3.6, 4.1–4.5, 5.1–5.7 | Lo que depende de selección/edición, dispositivo, fabricante o licencia se identifica como condicional; no se construye un inventario universal de menús de terceros. |

La cobertura se comprobó contra el manual de referencia de Live 12 (PDF oficial publicado el 2026-04-30), las notas de lanzamiento hasta Live 12.4.6 (2026-09-15), la comparación oficial de ediciones y la ayuda oficial de Ableton. El manual documenta que ajustes se abren desde Opciones en Windows y Live en macOS, y organiza diez páginas de ajustes; también describe navegación, controles del mezclador, aprendizaje e información contextual. Las notas 12.4 documentan Link Audio y el comando contextual de separación de stems para una selección temporal; las opciones de dispositivo y fabricante varían por selección/edición. Las fuentes no publican un listado invariable de todos los menús contextuales, por lo que esas variantes quedan condicionales de forma explícita.

### Correspondencia de documentación y alcance del inventario

La documentación oficial consultada enumera explícitamente las secciones de la barra de control y las vistas, los controles de mezcla, el navegador, la navegación accesible y las páginas de preferencias. Las notas oficiales de Live 12 aportan además opciones incorporadas al menú Navegación, accesibilidad, preescucha, entrada por pasos y acciones de edición. El catálogo precedente las agrupa por menú y contexto, y deja claro cuáles dependen de la selección o de una integración.

Fuentes oficiales consultadas para verificar las familias, nombres y opciones descritos:

- [Conceptos de Live: barra de control, Session/Arrangement, pistas, mezcla y señal](https://www.ableton.com/en/live-manual/12/live-concepts/).
- [Manual de referencia de Live 12 en PDF, edición publicada el 2026-04-30](https://cdn-resources.ableton.com/resources/pdfs/live-manual/12/2026-04-30/live12-manual-en.pdf), capítulos 2.3, 3–8, 17–25 y 41–42. La ayuda oficial indica que el manual está disponible en español dentro de Live; el PDF público descargado para esta revisión está en inglés.
- [Navegación accesible y teclado](https://www.ableton.com/en/live-manual/12/accessibility-and-keyboard-navigation/).
- [Atajos y menús contextuales de Live 12](https://www.ableton.com/en/manual/live-keyboard-shortcuts/), incluidos acceso a menús y comandos que sólo aparecen en contextos.
- [Ajustes y ubicación por plataforma](https://www.ableton.com/en/live-manual/12/first-steps/).
- [Preguntas frecuentes oficiales en español sobre navegación, mezclador y pantalla de Live 12](https://help.ableton.com/hc/es/articles/12243771208092-Preguntas-frecuentes-sobre-navegaci%C3%B3n-y-opciones-de-visualizaci%C3%B3n-en-Live-12).
- [Navegador y su menú de contenido](https://www.ableton.com/en/live-manual/12/working-with-the-browser/).
- [Navegador de Live 12: ayuda oficial en español](https://help.ableton.com/hc/es/articles/12927340213660-El-navegador-Live-12).
- [Mezclador y controles visibles](https://www.ableton.com/en/manual/mixing/) y [ruteo](https://www.ableton.com/en/manual/routing-and-i-o/).
- [Edición MIDI](https://www.ableton.com/en/manual/editing-midi/), [Arrangement](https://www.ableton.com/en/manual/arrangement-view/) y [administración de conjuntos/archivos](https://www.ableton.com/en/manual/managing-files-and-sets/).
- [Notas oficiales de lanzamiento de Live 12](https://www.ableton.com/en/release-notes/live-12/) (12.4.6, publicada el 2026-09-15) para cambios de menús, vista, reproducción, navegador y accesibilidad.
- [Novedades de Live 12.4: ayuda oficial en español](https://www.ableton.com/es/blog/live-12-4-is-coming/) y [notas oficiales detalladas de Live 12.4](https://help.ableton.com/hc/en-us/articles/26963870210076-What-s-new-in-Live-12-4).
- [Comparación oficial de ediciones Live 12](https://www.ableton.com/es/live/compare-editions/) para no atribuir opciones Suite a Intro/Standard.

El fabricante distribuye las opciones entre el manual, las notas de versiones y distintas guías; ninguna fuente reúne todas las combinaciones dinámicas en una tabla única. Por eso este registro describe las opciones publicadas y marca las que varían según sistema operativo, edición, selección o dispositivo. No se presupone que Live pueda ejecutarse en Linux ni se exige esa comprobación para avanzar. La matriz anterior cierra el cotejo de las opciones estáticas pertinentes con las acciones y tareas previstas para Estudio DAW.

Este catálogo y su matriz de trazabilidad constituyen la evidencia de cierre documental de la tarea 0.2, sin afirmar paridad ni implementación. Las opciones contextuales dependen de selección, plataforma, edición y dispositivos disponibles; el manual advierte que algunas sólo aparecen en menús contextuales y que éstos pueden exponer ajustes generales. Las variantes dinámicas no publicadas se mantienen identificadas como condicionales; una función no listada aquí no se infiere como requisito de Ableton.

El principio clave de Ableton es que Session y Arrangement no son proyectos separados: son dos maneras de interpretar y editar las mismas pistas. La matriz Session se puede tocar de inmediato y las ideas pueden capturarse/ordenarse en la línea de tiempo sin cambiar la identidad de pistas ni el estado del mezclador.

## Auditoría técnica secundaria: Ardour (no es referencia de experiencia de uso)

Ardour no es una referencia visual ni de flujo creativo para este producto. Consultar su manual sólo para contrastar conceptos técnicos de producción de audio cuando sea útil: propiedad de fuentes frente a listas/regiones, configuración de canales, señal entrada/procesador/panorama/ganancia/medidor/salida, captura/monitorización y ruteo. Sus menús (**Session** [Sesión], **Transport** [Transporte], **Edit** [Edición], **Region** [Región], **Track** [Pista], **View** [Vista], **Window** [Ventana] y **Help** [Ayuda]) se registran como contexto técnico, no como distribución de menús ni pauta de interacción que Estudio DAW deba imitar. Ableton Live 12 sigue siendo la referencia principal cuando difieran.

| Menú / superficie | Opciones de referencia y comportamiento que debemos considerar |
| --- | --- |
| **Encabezado de pista de audio y canal del mezclador** | Sólo contraste técnico: nombre/color/alto de pista, selección de entrada, cadena de procesadores, panoramizador, grabación/monitorización, silencio/solo, ganancia/fader y medidor de salida, automatización/grupo/punto de medición, ruteo de salida y comentarios. Describe la señal, no prescribe nuestro diseño de pantalla. |
| **Modelo de medios y edición** | Sólo contraste técnico: la sesión posee las fuentes; las pistas reproducen listas ordenadas de regiones no destructivas. La importación puede crear pistas o ubicar medios en el punto de edición/cursor/inicio, copiarlos a la sesión, conservar asignación de canales y permitir escucha previa. Pista y lista de reproducción son conceptos separados; las regiones referencian medios, no son el archivo fuente. |
| **Matriz de disparos** (Cue Grid) | Confirma de manera secundaria que casillas de audio/MIDI pueden compartir pistas/escenas, estado de lanzamiento, disparo por fila e integración con el mezclador. La vista Session de Ableton sigue siendo la referencia principal para el comportamiento y sensación de esta superficie creativa. |

## Política de menús y comandos del producto

- Estudio DAW debe alcanzar como mínimo la amplitud de referencia representada por estas familias de menús y añadir los flujos de análisis musical, código, notación y edición asistida previstos en la visión del proyecto.
- Es una meta de producto por etapas, no una obligación de entregar todos los comandos avanzados en un solo flujo. Cada etapa debe enumerar explícitamente los comandos implementados y las familias de menús diferidas.
- Cada opción visible de menú, botón de barra, acción contextual, atajo y control corresponde a un comando tipado de aplicación/dominio, un cambio real de vista o una opción claramente deshabilitada con motivo específico. No hay botones decorativos/falsos ni controles de «próximamente» presentados como acciones.
- Los comandos que cambian contenido del proyecto pasan por el bus de comandos y participan en deshacer/rehacer cuando la operación es reversible. Las operaciones de plataforma (configurar dispositivos, decodificar medios, diálogos de archivos) permanecen en adaptadores/procesos auxiliares.
- Los registros de menús y atajos son la única fuente de etiquetas, habilitación, combinaciones de teclas, nombres accesibles y ejecución. Los menús contextuales filtran ese registro según selección y tipo de pista; no crean mutaciones alternativas sin documentar.

## Matriz de implementación actual (revisión del código fuente)

| Área | Presente en el código | Pendiente para el contrato aprobado de estación de trabajo |
| --- | --- | --- |
| Interfaz/IPC del proyecto | Tauri permite nuevo/demo/abrir/guardar/guardar como, resumen de proyecto, deshacer/rehacer, transporte, ajustes de audio e importación de medios; editar regiones, mezclar pistas y administrar escenas/casillas se conecta por comandos reversibles. Los cambios de mezcla recompilan el plan al estar en Play y se aplican al reanudar desde pausa. | Conectar lanzamiento cuantizado Session; registro general de menús/acciones y menús contextuales atentos a la selección. |
| Modelo de proyecto | Marcador JSON actual `estudio-daw.project.v5` (revisión interna del formato, no versión del producto); usa `estudio-daw-midi-types` para eventos/tomas serializables y no depende de ALSA, PipeWire, motor de audio, diagnósticos de dispositivos, `ffmpeg` ni `ffprobe`. Conserva tipos MIDI/audio y roles MIDI/instrumento/audio/bus/retorno/salida principal; estado compartido del mezclador; fuentes con ruta/firma/hash; regiones con selección opcional de canales de origen, listas, escenas y casillas referenciadas sin duplicar contenido. `Project::validate_persisted_contracts` comprueba propiedad y referencias. La conversión de contenedores DAWproject recibe/devuelve bytes en memoria; la CLI y los adaptadores de aplicación realizan el acceso a archivos. | Mapas de entrada/salida específicos del dispositivo y ruteo en ejecución. |
| Comandos de proyecto | `ProjectCommand` permite agregar/duplicar/renombrar/reordenar/quitar/activar pistas; asignar estado de mezcla, destino interno, entrada física y armado de grabación; crear/renombrar/reordenar/quitar escenas; crear/quitar casillas; y agregar/recortar/mover/quitar/ajustar ganancia/desvanecimientos de regiones de audio. Las transacciones validan referencias, impiden ciclos de ruteo y se pueden deshacer/rehacer. Quitar una región la elimina de su lista, pero conserva su fuente; quitar una pista reasigna sus entradas al destino que tenía y conserva los archivos externos. | Conectar lanzamiento de clips a Tauri; ampliar edición (división, edición MIDI y más), salidas físicas y envíos. La presencia de comandos del dominio no significa que haya controles gráficos para ellos. |
| Adaptadores de medios/MIDI | `estudio-daw-media-adapter` inspecciona firma/hash, consulta metadatos técnicos y produce waveform min/max acotada con `ffmpeg`/`ffprobe`; también genera/valida proxies con publicación temporal y manifiesto. `estudio-daw-midi-types` separa los datos serializables de tomas/eventos del acceso a dispositivos ALSA. La interfaz puede inspeccionar metadatos y preescuchar hasta 30 segundos antes de confirmar importación, elegir copia/vínculo, selección mono/estéreo de canales de fuente y colocar una región mediante un comando reversible. Un decodificador `ffmpeg` transmite PCM al worker de reproducción; sólo metadatos y waveform/preview cruzan Tauri. Las regiones importadas conservan la preescucha Opus/WebAudio independiente del transporte. | Faltan búsqueda de bibliotecas generales/instaladas y ubicaciones de usuario. Las consultas de waveform/preview ejecutan procesos síncronos de medios desde comandos Tauri, fuera del callback de audio. |
| Instantáneas/interfaz | Los resúmenes de pista llevan valores de mezcla, color, entrada y destino. Tauri crea pistas MIDI/audio/bus, cambia ruteo interno mediante comandos reversibles y recompila el plan; Session, Arrangement y Mezclador muestran controles, medidores y selección compartida. El Mezclador muestra la ruta efectiva; Arrangement muestra waveform y permite mover/recortar/quitar regiones con comandos reversibles. La rejilla elegible (1/16, 1/8, negra, compás o libre) se aplica a mover y recortar audio; recortar el inicio actualiza la posición de fuente. Los clips MIDI se seleccionan y mueven con la rejilla elegida, usando el PPQ de cada clip y el comando reversible `MoveMidiClip`; se duplican consecutivamente y se dividen en el cursor mediante comandos reversibles. La división cierra/rearticula notas y restaura el estado CC, pitch bend, presión y programa en el segundo fragmento. El inspector presenta propiedades del clip MIDI y un piano roll de 36 teclas que muestra/inserta corcheas y permite seleccionar, editar tono/posición/duración/velocidad, borrar, mover y redimensionar notas mediante comandos reversibles que validan los eventos objetivo. | Entrada/salida de dispositivo por pista completa, cursor Session/Arrangement compartido, edición contextual MIDI/audio completa y lanzamiento Session; expresión MIDI, marcadores adicionales y QA visual/funcional en Arrangement. |
| Motor | PipeWire mezcla fuentes por pista, aplica controles/medidor en cada etapa y enruta a destinos internos siguiendo orden aguas abajo; los buffers por pista y fuente se reservan antes del callback. La suma llega a Master y a la salida, sin crear buffers ni resolver topología en tiempo real. Decodificación MIDI/audio y escritura de WAV usan workers/rings acotados; el reloj sigue los frames procesados y cambios en Play recompilan el plan, en pausa se aplican al reanudar. También hay loop A/B, restauración de estados MIDI, pánico y metrónomo. | Precisión sample-accurate del scheduler MIDI, QA funcional/acústica, lanzamiento de clips/escenas, cuenta previa, tomas por secciones, salidas físicas por pista y envíos/retornos/procesadores. |
| Interfaz Tauri | Acciones de proyecto, transporte, historial, perfiles de audio, importación/preescucha, grabación WAV y edición reversible de regiones; vistas principales Arreglo/Session con panel inferior contextual de clip o mezclador, controles de mezcla, medidores, armado, alta de bus, entrada PipeWire y salida interna a audio/bus/Master. Session muestra la matriz compartida, permite administrar escenas y asignar clips existentes; Arreglo permite mover/recortar regiones de audio con rejilla, editar posición/ganancia/desvanecimientos y mover, duplicar, cuantizar o dividir clips MIDI desde el menú contextual. El navegador lista y filtra regiones de audio importadas sólo cuando existen; preescucha y enfoca su región. La interfaz alterna Arreglo/Session con Tab, oculta el navegador con Ctrl+Alt+B y el detalle del clip con Mayús+Tab; el Mezclador se abre sin abandonar la vista de composición. Los clips MIDI pueden moverse, duplicarse, cuantizarse y dividirse en el cursor mediante comandos reversibles; la división conserva el estado MIDI de canal recuperable y rearticula notas sostenidas en el segundo fragmento. El piano roll MIDI inserta notas de 1/8 de forma reversible. | Completar familias de menús, registro tipado común con el dominio, edición de atajos, categorías creativas del navegador, edición contextual completa de notas y dispositivos, historial/favoritos/bibliotecas generales del navegador, lanzamiento de clips y QA visual/manual del flujo en tamaños de ventana distintos. |

Esta matriz se basa en `crates/ui-shell/src/main.rs`, `crates/ui-shell/src/audio_runtime.rs`, `crates/project-model/src/lib.rs`, `crates/command-bus/src/lib.rs` y `crates/ui-shell/frontend/{index.html,main.js}`. Es sólo una inspección del código: ninguna capacidad de interfaz de la columna pendiente se considera completa por existir metadatos de modelo o un comando desconectado.

## Objetivos y exclusiones de este cambio

**Objetivos:**

- Entregar un primer corte real y coherente de estación de trabajo con superficies Session y Arrangement sobre un único conjunto ordenado de pistas.
- Establecer contratos explícitos de pistas MIDI/audio, incluido flujo de señal, relación clip/medio, estado del mezclador y acciones admitidas.
- Hacer funcionales y respaldados por comandos los menús y controles principales visibles; no afirmar paridad de funciones avanzadas diferidas.
- Usar una jerarquía visual densa y legible orientada a instrumentos: barra de transporte/control, navegador por categorías, matriz de pistas/escenas o línea de tiempo, encabezados/mezclador y editor contextual inferior.
- Conservar archivos de proyecto existentes mediante valores predeterminados de serde/migración del formato, preservando la procedencia no destructiva de medios.

**Exclusiones del primer corte:**

- Implementar todos los efectos, formatos de complemento, grafos de ruteo, superficies de control, interfaces de programación, herramientas de análisis o ajustes de exportación de las referencias maduras.
- Reproducir recursos, ilustraciones exactas o código propietario de Ableton o Ardour.
- Mostrar como funcionales el armado, la monitorización de entrada, el ruteo de dispositivos, la inserción de complementos, la reproducción de audio o la grabación antes de que exista y se verifique su ruta de ejecución.

## Decisiones

### Un modelo compartido de pistas y dos vistas creativas

Session es una matriz con columnas de pistas y filas de escenas. Arrangement es una línea de tiempo musical con las mismas pistas ordenadas. Ambas leen las mismas entidades del proyecto y estado del mezclador. Cambiar de superficie no duplica contenido ni reinicia la reproducción. Crear, renombrar, reordenar o borrar pistas actualiza ambas vistas mediante comandos reversibles.

Alternativa considerada: mantener la línea de tiempo actual y añadirle una etiqueta Session. Se descartó porque no ofrece el flujo no lineal de lanzamiento de clips solicitado.

### Arrangement y Session conservan el foco creativo

La jerarquía de interacción sigue Live: una barra concentra transporte, tempo y
posición; los controles de vista mantienen Arrangement/Session como los dos
espacios de composición; el navegador del proyecto se puede ocultar; el detalle
de clip se abre y cierra sin perder la selección; y el
mezclador se consulta como panel del espacio de trabajo, sin sustituir
Arrangement/Session por una página independiente. En Estudio DAW, Tab alterna
Arrangement/Session, Mayús+Tab muestra u oculta el detalle del clip y
Ctrl+Alt+B muestra u oculta el navegador. El botón Mezclador abre el panel
inferior contextual. Se evita duplicar la navegación de vistas en una barra
lateral, mantener resúmenes métricos en el navegador o dar a acciones de demo
el mismo peso visual que al trabajo musical.

Este corte aplica el flujo de la referencia, no afirma paridad visual ni
funcional. La tarea 3.1 sigue abierta hasta que el navegador tenga categorías
creativas respaldadas por contenido real y la superficie pase revisión visual
con proyectos cargados en tamaños de ventana estándar y estrecho.

### Las pistas de audio modelan fuentes, regiones, listas y flujo de señal

Una pista de audio es una entidad del proyecto con nombre/orden/configuración de canales persistentes, estado activa/silencio/solo, ganancia/panorama, destino de salida, fuentes de medios y lista ordenada no destructiva de clips/regiones. Un clip apunta a medios fuente y guarda desplazamiento de fuente, posición temporal, duración, desvanecimientos y ganancia. La ruta ejecutada es fuente/regiones → mezcla de pista → medidor → destino; un bus suma las pistas que lo enrutan, aplica sus controles propios y pasa su señal a otro destino. El destino predeterminado es Master. Las rutas internas se validan sin ciclos y el plan calcula su orden fuera del callback; entradas de hardware, envíos, retornos y procesadores se incorporarán sólo cuando su flujo esté ejecutable. Las pistas MIDI/instrumento conservan identidad propia de clip/instrumento.

La implementación reconoce como cortes separados la importación/ubicación, la preescucha, la edición de límites, la reproducción, la grabación/monitorización, el ruteo/procesamiento y los buses. La importación ofrece preescucha previa y selección persistida de canal mono o pareja estéreo, que el decodificador aplica. El plan compila rutas internas y aplica mezcla/medidores por pista; también captura entradas PipeWire por pista y graba las pistas armadas a WAV para crear regiones al detener. Siguen pendientes selección de salidas físicas por pista, cuenta previa, tomas por secciones, procesadores y QA visual/auditiva.

### Los menús describen capacidades del producto, no adornos de implementación

La organización principal de menús/vistas sigue Archivo, Edición, Crear, Vista, Opciones y Ayuda de Ableton Live, la ubicación de ajustes Live/Opciones específica de cada plataforma, además de su barra de control, navegador, Session, Arrangement, vistas Clip/Dispositivo y mezclador. Se pueden añadir acciones técnicas propias de Estudio, como configuración de audio/MIDI. Ardour aporta sólo conceptos técnicos de audio; la distribución de sus menús y organización visual no son objetivos. Una opción de menú se incorpora sólo junto con su comando, condición de disponibilidad, política de deshacer, atajo cuando corresponda, información de estado y verificación determinista. El catálogo de comandos es más amplio que el menú habilitado inicial y se entrega por etapas en tareas OpenSpec.

### Los controles de mezcla son coherentes entre editor y mezclador

Ganancia, panorama, silencio, solo y estado activo de la pista son estado compartido del proyecto y se muestran de forma coherente en Session, Arrangement y Mezclador. El armado/monitorización de entrada sólo aparece habilitado cuando funcionan la ruta de grabación y la entrada de hardware elegida. Los canales del mezclador comunican dirección de entrada, procesamiento y salida; los medidores muestran valores medidos, nunca niveles simulados.

### El lanzamiento de clips en tiempo real es una función del motor

El lanzamiento de clips/escenas usará el tempo/cuantización de sesión, mensajes de control acotados a un planificador fuera de la devolución de audio y estado de reproducción por pista. La devolución no asigna memoria ni reconstruye el plan de dispositivo/renderizado. Los eventos MIDI se detienen correctamente al reemplazar/detener. Arrangement ya mezcla regiones de audio mediante decodificación/almacenamiento temporal acotados; integrarlas al lanzamiento de Session requiere completar las reglas de lanzamiento y repetición.

### El navegador de referencias es útil y veraz

El navegador ofrece pistas/clips reales del proyecto y ubicaciones de medios elegidas por la persona, con búsqueda, historial, filtros y escucha previa cuando exista el servicio correspondiente de fuente/preescucha. No muestra sonidos de fábrica, bibliotecas licenciadas ni complementos ficticios. El contenido SoundFont/Analog Lab del usuario sólo aparece mediante una integración real con dispositivo/medios instalados.

## Migración de datos

Los campos aditivos usan valores predeterminados de serde y migración del formato. Se pueden seguir leyendo `TrackKind::Midi`/`Audio`, clips MIDI, procedencia de clips de audio y proyectos JSON actuales. La identidad de escenas/casillas y los campos nuevos del mezclador reciben valores iniciales explícitos. Una prueba de migración debe demostrar que los ejemplos antiguos se abren con identidad/orden estable de pistas/clips y se pueden volver a guardar.

## Riesgos y compromisos

- **La referencia abarca muchas funciones** → implementar cortes útiles por etapas; conservar el inventario completo como hoja de ruta y nunca afirmar falsamente paridad total.
- **La semántica de pistas de audio abarca varias capas** → definir el modelo serializable de medios/regiones/listas y sus contratos de comandos antes de pulir la interfaz.
- **Los lanzamientos Session requieren planificación en vivo** → aislar el controlador del planificador de la devolución PipeWire y evitar reiniciar dispositivos al cambiar clips.
- **Las superficies densas pueden abrumar** → mantener contextual el detalle específico de selección y permitir mostrar/ocultar navegador/mezclador/detalle sin perder jerarquía visual clara.
- **Las entradas generadas del navegador pueden sugerir licencias/capacidades** → enumerar sólo contenido de usuario instalado, indexado y con preescucha real, junto con su procedencia.

## Plan de migración y verificación

1. Conciliar estos requisitos/tareas con el modelo y el bus de comandos; añadir campos del modelo migrables y comandos reversibles.
2. Añadir operaciones de aplicación/Tauri y soporte coherente del motor para el corte vertical real.
3. Construir Session, Arrangement, navegador, editor contextual y mezclador a partir de instantáneas/resultados de comandos.
4. Verificar todas las rutas habilitadas de menús/controles, persistencia/migración, deshacer/rehacer, ubicación musical y comportamiento del motor de audio mediante comprobaciones deterministas. Ejecutar las comprobaciones del espacio de trabajo cuando compile el corte; reservar las comprobaciones de hardware para capacidades que requieran dispositivos reales.
5. Revisar la interfaz en ejecución en tamaños estándar y mínimo. Registrar tareas completas exactas, evidencia, menús diferidos conocidos y límites visibles para el usuario. Ninguna tarea se completa sólo por revisar el código de interfaz.

## Delta de planificación UI para Luna — 2026-09-29

### Objetivo y estado de esta revisión

Plan solicitado por la persona usuaria para ejecutar posteriormente con Luna.
Base inspeccionada: `052357de56fb390e1f7848f31799ea90033a10f3`, árbol limpio al
iniciar. Esta revisión no implementa frontend, no ejecuta la aplicación y no
significa aceptación visual. Conserva el historial y concreta la tarea 3.1.
Las demás funciones permanecen en pausa hasta feedback explícito del usuario.

Este delta sustituye, para la próxima implementación, la decisión de hacer
excluyentes Mezclador y detalle en el mismo panel inferior. La composición
anterior se conserva arriba como evidencia del corte publicado. El objetivo
ahora separa mezcla integrada en la superficie y detalle contextual inferior.

La propuesta y los requisitos generales siguen vigentes. Las tablas históricas
de capacidades anteriores no son un inventario actualizado: Session ya tiene
manejadores de lanzamiento y el adaptador expone retorno al Arreglo. Este plan
no pide implementar nuevamente esos comportamientos.

### Evidencia de partida y límites

- `index.html` coloca transporte, vistas, historial, rango, rejilla y paneles
  en `.transport-bar`. El CSS final le aplica desplazamiento horizontal con
  barra invisible: puede esconder acciones sin una señal clara.
- `styles.css` acumula varias redefiniciones de `.editor` y sus estados, con
  alturas y mínimos diferentes. Reorganizar las reglas por componente/estado;
  no resolver el rediseño añadiendo otra capa de excepciones al final.
- `renderSessionSurface` usa escenas a la izquierda y cabeceras con controles;
  `renderMixerSurface` dibuja canales en otro contenedor. `updateWorkspaceLayout`
  oculta el inspector al abrir el mezclador.
- `renderProjectMedia` usa `snapshot.audioClips`; la Demo MIDI no puebla esa
  lista. `snapshot.midiClips` y `snapshot.tracks` permiten navegación musical
  real sin inventar una biblioteca instalada.
- Revisar el cierre de `browser-project-panel`: el HTML fuente abre `section`
  y cierra `div`. Corregir la estructura al reorganizarla y verificar el DOM
  resultante; no asumir que el navegador lo repara con el árbol deseado.
- El índice `estudio-daw` de codebase-memory estaba fechado el 26 de septiembre
  y reportó `metadata_changed` en los cuatro archivos frontend. Las referencias
  anteriores proceden de lectura directa del código vigente, no de ese grafo.
- No hay captura nueva ni QA en ejecución en esta planificación. La percepción
  de que la UI sigue lejos del objetivo proviene del feedback del handoff.
  Luna debe capturar una línea base antes de modificar la aplicación.

### Composición propuesta

Medidas iniciales en píxeles CSS, área interior útil del WebView y zoom 100 %;
son objetivos de Estudio DAW, no medidas atribuidas a Ableton. Si una captura
incluye decoraciones del sistema, registrar también el tamaño interior real.

```text
ARREGLO · 1920 × 1080
┌─────────────────────────────────────────────────────────────────────┐
│ Menús · nombre del proyecto                                   28 px │
├─────────────────────────────────────────────────────────────────────┤
│ Tempo/métrica │ Stop Play Pause Rec · posición │ loop · vistas 40 px │
├──────────────┬───────────────────────────────────────┬───────────────┤
│ NAVEGADOR    │ Herramientas locales · overview/regla │ CABECERAS     │
│ 280 px       ├───────────────────────────────────────┤ 232 px        │
│ búsqueda    │ clips / waveform / notas             │ pista y mezcla│
│ categorías  │ lienzo temporal: 1408 px              │ compacta      │
│ resultados  │ superficie central total: 690 px      │               │
│             ├───────────────────────────────────────┴───────────────┤
│             │ Clip / Dispositivo · detalle: 300 px                  │
├──────────────┴───────────────────────────────────────────────────────┤
│ Estado y ayuda contextual                                     22 px │
└─────────────────────────────────────────────────────────────────────┘

SESSION · mismo marco
┌──────────────┬───────────────────────────────────────┬───────────────┐
│ NAVEGADOR    │ Pista 1 │ Pista 2 │ Pista 3 │ …         │ ESCENAS       │
│              │ clip    │ clip    │ vacío   │           │ lanzar fila   │
│              │ vacío   │ clip    │ clip    │           │ …             │
│              │ parada y estado por pista              │               │
│              ├───────────────────────────────────────┤               │
│              │ mezcla alineada con cada columna       │ buses / Master│
│              │ ganancia · panorama · M/S · medidor     │ reales        │
│              ├───────────────────────────────────────┴───────────────┤
│              │ Clip / Dispositivo, independiente del mezclador       │
└──────────────┴───────────────────────────────────────────────────────┘
```

| Zona | 1920×1080 | 1280×720 |
| --- | --- | --- |
| Menú / control global / estado | 28 / 40 / 22 px | 28 / 40 / 22 px |
| Navegador expandido | 280 px | 224 px |
| Cabecera de pista de Arreglo | 232 px | 208 px |
| Lienzo temporal restante | 1408 px | 848 px |
| Superficie central con detalle abierto | 690 px | 420 px |
| Detalle inicial | 300 px | 210 px |
| Overview + regla + herramientas locales | hasta 64 px | hasta 56 px |
| Altura inicial de pista de Arreglo | 64 px | 56 px |
| Columna Session / fila de clip | 128 / 30 px | 112 / 28 px |
| Mezcla integrada de Session | 180 px | 140 px |

En 1280×720 el esquema conserva las mismas zonas: 224 px de navegador y
1056 px de área de trabajo; dentro del Arreglo quedan 848 px de lienzo y
208 px de cabeceras. Session reserva 112 px a escenas/Master y distribuye el
resto en columnas de 112 px con desplazamiento horizontal local. Las pistas
adicionales se desplazan dentro de su superficie; nunca todo el documento.
No se reduce el zoom automáticamente para cumplir estas medidas.

El navegador se puede ocultar; el detalle se redimensiona mediante el separador
existente. Aplicar límites según espacio disponible, evitando mínimos de CSS
que obliguen a sacar el transporte o la barra de estado de la ventana. Con
detalle abierto deben caber al menos seis filas de 56 px en Arreglo a 1280×720;
en Session, seis filas de clips y mezcla de 140 px. No introducir pistas ni
clips ficticios para llenar esos espacios.

### Decisiones de interacción y presentación

1. **Control global estable.** Menús y nombre en una fila discreta; transporte,
   posición, tempo/métrica, metrónomo, rango A/B y Arreglo/Session en la siguiente.
   Tempo y métrica conservan su disponibilidad real; no convertir lecturas en
   editores si no existe una operación conectada. Guardar/deshacer permanecen
   accesibles por menú/atajos. Demo MIDI pasa a Proyecto y al estado inicial.
   Rejilla/zoom temporal pertenecen a la cabecera de la superficie. Preferencias
   y diagnóstico abren un panel/diálogo, no ocupan espacio permanente del editor.
   En anchuras menores, usar un menú visible para acciones secundarias; Play,
   Stop, posición y cambio de vista no pueden depender de scroll oculto.
2. **Navegador de contenido.** Sustituir Proyecto/Crear por búsqueda y categorías
   de contenido del proyecto: Audio, Clips MIDI e Instrumentos asignados, sólo
   cuando tengan elementos reales. Derivar las dos primeras de los snapshots;
   la tercera sólo de asignaciones reales de pistas, sin afirmar que el plugin
   está cargado ni enumerar instalaciones del sistema. Un resultado enfoca su
   pista/clip; Audio conserva la preescucha y sus restricciones actuales. Los
   clips MIDI no reciben preescucha simulada. Crear pista queda en Crear y en un
   botón compacto junto a las pistas. Importar abre el formulario existente en
   un panel invocado, preservando copia/vínculo, canales, destino y cursor.
   Un proyecto vacío ofrece Nuevo/Abrir/Demo/Crear/Importar según disponibilidad,
   con explicación breve, sin catálogo de categorías vacías.
3. **Arreglo como superficie musical.** Overview y regla alineados con los
   clips; cabeceras fijas a la derecha, scroll vertical sincronizado y anchuras
   coherentes al ocultar navegador. Nombre/color, activa/M/S, armado cuando
   corresponda y medidor real forman el encabezado compacto. Ganancia/panorama
   accesibles sin abrir una página; ruteo, identidad detallada y configuración
   de instrumento pasan a un desplegable contextual. Quitar/reordenar/duplicar
   conservan sus acciones de menú. La altura inicial de las pistas es 64 px
   (56 px en ventanas compactas); un separador por pista permite ajustarla con
   ratón, flechas o Home/End dentro de 56–320 px. Cabecera y carril comparten
   altura y los clips se adaptan. La preferencia se guarda localmente por
   proyecto y no altera el modelo musical ni su historial. No ocultar funciones
   sin dejar acceso visible.
4. **Session integrada.** Escenas a la derecha, nombres de pista arriba,
   botones de lanzamiento/parada distinguibles de selección y asignación.
   Las celdas no son formularios permanentes: mostrar nombre y estado;
   asignación, cuantización y modo se abren desde contexto/detalle de casilla.
   Mezcla debajo de cada columna con el mismo ancho y scroll horizontal.
   Buses/retornos existentes se muestran como canales sin casillas lanzables;
   Master queda al extremo derecho y conserva el caso legado informativo.
   No crear buses ni retornos nuevos por esta distribución. «Volver a Arreglo»
   conserva el estado `sessionOverrideActive` y su acción real; cambiar de vista
   no equivale a devolver el control musical al Arreglo.
5. **Detalle Clip/Dispositivo.** Seleccionar un clip muestra su editor real;
   seleccionar la pestaña Dispositivo muestra la asignación de la pista enfocada
   y reutiliza `createTrackInstrumentControl`. No dibujar una cadena de efectos
   o parámetros ficticios. Separar foco de pista para el detalle de la selección
   múltiple usada por grupos. Piano roll: propiedades a la izquierda, teclado,
   regla, notas y velocidades en el resto; audio conserva controles de región.
   El dispositivo sin asignación ofrece únicamente acciones de carga existentes.
6. **Mezcla y atajos.** Ctrl+3 muestra/oculta mezcla integrada sin cerrar el
   detalle. En Session es la banda alineada bajo las casillas; en Arreglo abre
   una banda de canales sobre el detalle, inicialmente de 160 px (120 px en
   ventana estrecha), reducible. En ese estado adicional se permite reducir
   las filas visibles, pero quedan al menos tres pistas. No duplicar controles
   DOM activos de una misma banda al cambiar de vista. Tab conserva el cambio
   Arreglo/Session; Mayús+Tab conserva mostrar/ocultar detalle; Clip/Dispositivo
   se elige con botones visibles. Ctrl+Alt+B y los atajos de proyecto se conservan.
   Respetar campos editables y navegación por teclado de menús/diálogos; no
   interceptar Tab dentro de ellos. Escape cierra overlays y devuelve el foco.
7. **Lenguaje visual.** Superficies continuas, divisores finos, sin sombras de
   tarjetas; jerarquía por posición y contraste. Base gris neutra oscura,
   acento cálido para selección, colores musicales del proyecto en pistas/clips.
   Texto habitual 12–13 px, secundario no menor de 11 px al 100 %, controles
   compactos de 24–28 px y foco visible. Iconos simples con nombre accesible;
   los estados seleccionado/en cola/reproduciendo no dependen sólo del color.
   Estas medidas son punto de partida a contrastar en ejecución, no aceptación.

### Entregas secuenciales para Luna

No ejecutar tareas pendientes ajenas a esta secuencia aunque OpenSpec Apply las
enumere antes. Cada etapa entrega evidencia y deja una base utilizable; no hace
falta solicitar autorización para cada decisión rutinaria dentro del alcance.

| Etapa | Archivos y puntos de intervención | Resultado y comprobación de salida |
| --- | --- | --- |
| L1 · línea base y mapa de controles | Leer `frontend/{index.html,styles.css,main.js,platform-tauri.js}`, especialmente `elements`, `UI_ACTIONS`, `updateWorkspaceLayout`, `selectSurface` | Capturar Demo MIDI en Arreglo y Session a ambos tamaños; inventariar cada control que se mueve, su destino, ID/manejador y condición de disponibilidad. Registrar dimensiones y acceso a runtime. Si no hay control visual disponible, registrar el bloqueo de QA sin inventar capturas. |
| L2 · marco y jerarquía | Editar `index.html`, `styles.css`; sólo enlaces/estado de layout en `main.js` | Aplicar las zonas y medidas, corregir etiquetas HTML, consolidar CSS, mover preferencias/importación y acciones secundarias. Transporte siempre visible, detalle redimensionable y ningún ID perdido o duplicado. Capturas comparables con L1. |
| L3 · navegador y Arreglo | `renderProjectMedia`, `renderSnapshot`, `renderTimelineRuler`, `renderArrangementOverview`, enlaces de selección/edición; CSS de pistas | Navegador útil con Demo MIDI, categorías reales y selección por ID; cabeceras compactas y regla/lienzo alineados. Conservar PPQ, cursor, seek, waveform y edición reversible. Mover un clip y deshacer debe conservar su identidad y posición original. |
| L4 · Session y mezcla integrada | `renderSessionSurface`, `renderMixerSurface`, `createTrackMixerControls`, `createTrackMeter`, `updateWorkspaceLayout`, `selectSurface` | Escenas a la derecha, mezcla alineada, canales sin casillas para buses/Master y desplazamiento sincronizado. Conservar selección, estados, cuantización, modo y retorno a Arreglo; menús sustituyen selectores permanentes. Verificar que un gesto llama una sola vez al manejador existente. |
| L5 · detalle y continuidad | `renderClipInspector`, `createTrackInstrumentControl`, selección y atajos; CSS del panel inferior | Clip/Dispositivo contextual, mezcla y editor simultáneos, estado visual local sin mutar proyecto. Cambiar de vista/panel no pierde clip, foco, scroll, zoom ni transporte. Controles de VST/standalone mantienen las aprobaciones locales existentes. |
| L6 · revisión completa | Documentación de resultados y archivos frontend que requieran corrección | Matriz visual/funcional siguiente, corrección de regresiones, actualización factual de README/guía/handoff y bitácora. Presentar antes/después al usuario. Mantener 3.1 abierta hasta aceptación explícita; no retomar funciones automáticamente. |

Conservar el frontend actual sin migración a React, framework, bundler ni nuevo
sistema de componentes. Se permiten helpers pequeños para reutilizar renderizado;
no dividir todo `main.js` como trabajo previo. `platform-tauri.js` es contrato
a conservar: no agregar IPC ni editar Rust para resolver esta UI. Si falta una
capacidad de dominio, registrar un pendiente separado y continuar lo independiente.

Al mover botones, recordar que `UI_ACTIONS` deriva ejecución/disponibilidad de
`elements`: no dejar menús conectados a nodos eliminados ni crear botones ocultos
como puente permanente. Reutilizar manejadores para botón, menú y atajo. Los
snapshots pueden reconstruir DOM; preservar foco, borradores, selección y scroll
sin duplicar listeners, temporizadores, llamadas de preview ni suscripciones.

### Matriz de revisión y definición de terminado

- Capturas antes/después con el mismo proyecto, ventana, zoom, posición y paneles:
  Arreglo con clip MIDI, Session con mezcla, detalle Dispositivo y estado vacío;
  1920×1080 y 1280×720. Añadir audio real autorizado para revisar waveform,
  importación y detalle de audio; no presentar una captura MIDI como QA de audio.
- Recorrer abrir/Demo → seleccionar clip → editar nota → deshacer/rehacer →
  Session → seleccionar/asignar casilla → lanzar/parar → volver a Arreglo →
  cambiar mezcla → guardar/reabrir una copia temporal. Separar lo observado en
  UI de lo verificado en motor; no afirmar escucha ni latencia física por capturas.
- Sin scroll de página ni transporte desplazable oculto; clips y controles no
  se solapan. Nombres largos se truncan con acceso al nombre completo. Filas,
  reglas y columnas de mezcla conservan alineación al desplazar/redimensionar.
- Navegador y detalle se ocultan/reabren sin perder selección; mezcla no cierra
  el piano roll. Verificar zoom de UI 80/100/150 % para accesibilidad, además de
  las capturas de referencia al 100 %. En zoom alto se admiten paneles plegados
  y scroll local, no controles globales inaccesibles.
- Confirmar nombres accesibles, foco visible, teclado en menús/diálogos y que los
  atajos no roban escritura. No introducir contenido, medidores ni estados falsos.
- Tras editar JS: `node --check crates/ui-shell/frontend/main.js` y
  `node --check crates/ui-shell/frontend/platform-tauri.js`; al cerrar:
  `git diff --check` y
  `openspec validate workstation-arrangement-surface-v2 --strict`.
  Ejecutar Tauri para QA; la sintaxis o compilación no sustituyen la revisión
  visual. No ejecutar suites Rust salvo instrucción del usuario; este corte
  no modifica Rust. Verificar interacciones con herramientas disponibles y
  registrar por separado lo que no se pudo ejecutar.
- La aceptación funcional/visual de esta secuencia no cierra automáticamente
  2.2, 2.3, 3.2–3.6, 4.x ni 5.x. La aceptación del usuario de la dirección visual
  es necesaria para cerrar 3.1; si falta parte de su alcance, sigue parcial.

### Prompt de ejecución para Luna

```text
Implementa sólo el delta «Delta de planificación UI para Luna — 2026-09-29»
en design.md de workstation-arrangement-surface-v2, etapas L1–L6 y subtareas
3.1.1–3.1.6. Lee AGENTS.md y las fuentes canónicas en su orden antes de editar.
Comprueba git status/log; la base de planificación fue 052357d y puede haber
avances posteriores que debes preservar. Usa los símbolos del plan como puntos
de entrada, no números de línea fijos; verifica el código vigente.

Prioridad: transformar la distribución completa, con contenido musical visible,
Browser real, transporte estable, Arreglo amplio, Session con mezclador alineado
y detalle Clip/Dispositivo independiente. Sigue las medidas, interacciones y
criterios del delta. Captura primero Demo MIDI y compara los mismos estados al
terminar. No basta cambiar colores, márgenes o tamaños de fuente.

Limita la implementación al frontend y a documentación factual. Conserva los
comandos/IPC, permisos de seguridad, motor, formato de proyecto, operaciones
musicales y funciones ya existentes. No agregues backend, bibliotecas falsas,
plugins simulados ni nuevas funciones audio/MIDI. No avances otras tareas de
OpenSpec. Si falta acceso visual, registra el bloqueo de QA y continúa el trabajo
independiente, sin declarar la revisión aprobada.

Trabaja por etapas y sin confirmaciones rutinarias. Registra evidencia y límites
en la bitácora append-only. No marques 3.1 completa sin revisión visual real y
aceptación explícita del usuario; no hagas commit/push sin autorización vigente.
```

Referencias de esta revisión: [Session View](https://www.ableton.com/en/manual/session-view/)
documenta pistas por columnas, escenas a la derecha y la diferencia entre selección,
lanzamiento y retorno al arreglo; [Arrangement View](https://www.ableton.com/en/manual/arrangement-view/)
se usa como referencia de línea temporal. Las dimensiones, etapas y composición
propuestas aquí son decisiones de Estudio DAW. No se copian código ni activos.

## Fuentes oficiales consultadas

Revisado el 2026-09-27:

- Ableton Live 12: [Conceptos de Live y barra de control](https://www.ableton.com/en/manual/live-concepts/), [Vista Session](https://www.ableton.com/en/manual/session-view/), [Vista Arrangement](https://www.ableton.com/en/manual/arrangement-view/), [lanzamiento de clips](https://www.ableton.com/en/manual/launching-clips/), [mezcla](https://www.ableton.com/en/manual/mixing/), [navegador](https://www.ableton.com/en/live-manual/12/working-with-the-browser/), [ajustes](https://www.ableton.com/en/live-manual/12/first-steps/).
- Ardour: [índice del menú principal](https://manual.ardour.org/ardours-interface/main-menu/), [menú Session](https://manual.ardour.org/ardours-interface/main-menu/Session-menu/), [menú Transporte](https://manual.ardour.org/ardours-interface/main-menu/Transport-menu/), [menú Edición](https://manual.ardour.org/ardours-interface/main-menu/Edit-menu/), [menú Región](https://manual.ardour.org/ardours-interface/main-menu/Region-menu/), [menú Pista](https://manual.ardour.org/ardours-interface/main-menu/Track-menu/), [menú Vista](https://manual.ardour.org/ardours-interface/main-menu/View-menu/), [menú Ventana](https://manual.ardour.org/ardours-interface/main-menu/Window-menu/), [controles de pista de audio](https://manual.ardour.org/working-with-tracks/audio-track-controls/), [canales del mezclador](https://manual.ardour.org/ardours-interface/audio-midi-mixer-strips/), [sesiones y pistas](https://manual.ardour.org/working-with-tracks/), [importación de medios](https://manual.ardour.org/adding-pre-existing-material/), [regiones](https://manual.ardour.org/working-with-regions/), [flujo de matriz de disparos](https://manual.ardour.org/cue/).

La lista de Ableton refleja superficies, ámbitos de menú/función y páginas de configuración documentadas para Live 12; la referencia de Ardour se limita a conceptos de flujo de señal y medios, no a la experiencia visual o creativa. Las opciones contextuales que dependen de dispositivos y selecciones se señalan como condicionales. La tarea 0.2 se cerró mediante cotejo documental de opciones publicadas; este documento no afirma paridad de funciones.

## Revisión de jerarquía orientada al músico — 2026-09-29

La persona usuaria indica que la interfaz actual dispersa acciones de pista y
dispositivos y cuesta encontrar opciones. El flujo se reorganiza alrededor de la
pista seleccionada: Browser ofrece contenido; Arrangement y Session muestran
identidad, estado y acciones inmediatas de la pista; Mixer se limita a ruteo y
mezcla; la vista inferior concentra edición contextual y cadena de dispositivos.
Los accesos en pistas sólo navegan a esa cadena. Los controles de agrupación
aparecen únicamente con dos o más pistas seleccionadas. La cadena presenta el
instrumento real y declara que los efectos todavía no existen en el motor; no
se habilitan espacios falsos ni editores duplicados en cada superficie.

Esta decisión toma como referencia funcional el Browser de dispositivos y la
Device View de pista descritos por
[Ableton, Conceptos de Live](https://www.ableton.com/en/manual/live-concepts/),
los controles de pista descritos en
[Arrangement View](https://www.ableton.com/en/manual/arrangement-view/) y el
orden compartido Editor/Mezclador de
[Ardour](https://manual.ardour.org/working-with-tracks/controlling-track-ordering/).
Guían la organización de Estudio DAW; no representan paridad ni autorizan
copiar código o activos.

El piano roll expone el rango MIDI completo (0–127), por lo que cualquier clip
puede recorrer más de cuatro octavas. La rueda desplaza alturas y Ctrl+rueda
ajusta la escala vertical entre 4 y 28 px por semitono, anclando el tono bajo el
puntero. La vista inicial enfoca las notas del clip y guarda su desplazamiento.
