# Diseño

## Contexto

Este cambio corrige una pasada anterior de interfaz que trató un DAW como un panel oscuro más una línea de tiempo de sólo lectura. Aquella pasada no satisfizo el flujo creativo solicitado de Ableton Live 12, no entregó un sistema utilizable de pistas de audio y marcó tareas como completas antes de tiempo. Esas tareas se reabren aquí. Los manuales oficiales de Ableton Live 12 y Ardour se revisaron el 2026-09-27; este documento registra las vistas principales, familias de menús de primer nivel y ámbitos de opciones que orientan el producto. Es una referencia de producto, no una afirmación de que la ventana actual implemente esas funciones. Otra tarea pendiente exige el inventario completo opción por opción y de menús contextuales antes de afirmar paridad de menús.

La implementación actual es una ventana Tauri con apertura/guardado de proyectos, reproducción MIDI, transporte, historial y ajustes de perfil de audio. El modelo de proyecto distingue pistas MIDI y de audio y permite operaciones no destructivas sobre metadatos de clips de audio, pero la ventana aún no tiene un flujo completo de pista de audio, reproducción de clips de audio, grabación, matriz real de clips Session ni mezclador funcional. El código fuente y las comprobaciones ejecutables, no este documento, determinan las capacidades presentes.

## Auditoría de referencia: Ableton Live 12

El diseño toma de Live el flujo creativo y la jerarquía de superficies, sin copiar recursos gráficos ni marca protegidos.

| Superficie / menú | Opciones de referencia y comportamiento que debemos considerar |
| --- | --- |
| **Barra de control / Transporte** | Reproducir/detener/grabar, tempo y compás, metrónomo, cuantización global de lanzamiento, cambio Session/Arrangement, estado/uso de CPU y visibilidad de vistas/navegador. Mantiene accesibles los controles frecuentes y deja espacio para editar. |
| **Navegador** | Búsqueda; historial atrás/adelante; colecciones; categorías de biblioteca: Todo, Sonidos, Baterías, Instrumentos, Efectos de audio, Efectos MIDI, Moduladores, Max for Live, Complementos, Clips, Muestras, Grooves y Plantillas; ubicaciones: Proyecto actual, Biblioteca de usuario y carpetas del usuario; filtros/etiquetas, escucha previa y arrastre al conjunto. Los paquetes e integraciones externos sólo aparecen cuando existen. |
| **Vista Session** | Columnas de pistas × filas de escenas; lanzamiento/parada/selección de clips por casilla y estado reproduciendo/en cola; lanzamiento de escena entre pistas; casillas vacías; un clip activo por pista; cuantización, modo de lanzamiento, legato, velocidad, desplazamiento/empujón de clip, bucle y acciones posteriores; sección de mezclador y controles de escena. En su pista, un clip Session tiene precedencia sobre la reproducción de Arrangement. |
| **Vista Arrangement** | Las mismas pistas en vertical contra compases/pulsos; formas de onda y regiones MIDI; mover/cambiar tamaño/dividir/duplicar clips; bucle y selección temporal; carriles de tomas y compilación; controles de pista; reglas/marcadores; mezclador. Cambiar de vista conserva reproducción e identidad compartida de pistas. |
| **Detalle de clip/dispositivo** | El panel inferior contextual sigue la selección. Los clips MIDI ofrecen edición de notas y controles de tiempo/bucle; los clips de audio, muestra/forma de onda y deformación temporal; los dispositivos, sus parámetros. El panel cambia con la selección y no se queda como tarjeta genérica de ajustes. |
| **Mezclador / controles de pista** | Valores compartidos entre Session y Arrangement; activador, solo, armado de grabación si está admitido, panorama, volumen, medidores, entrada/salida y controles configurables del mezclador. Envíos/retornos/master y escucha de referencia son conceptos reales de señal, no botones decorativos. |
| **Menús de aplicación** | Archivo: ciclo de vida del conjunto/proyecto, reunir/administrar archivos, importar/exportar. Edición: deshacer/rehacer, portapapeles, selección y edición. Crear: pistas, clips, escenas, tiempo/compás/marcadores. Vista: Session/Arrangement, navegador, panel de detalle, mezclador y controles seleccionables de pista/mezclador. Opciones: ajustes de audio, MIDI, Link, pantalla, biblioteca, complementos, archivos y Grabación/Deformación/Lanzamiento. Ayuda: aprendizaje, referencia e información. Los menús contextuales amplían estas acciones según pista, clip, escena, elemento del navegador o dispositivo seleccionado. Cada opción debe corresponder a un comando identificable e informar con honestidad cuando no esté disponible. |

### Inventario de menús y acciones contextuales de Ableton Live

Este inventario describe la aplicación principal y las superficies creativas. Incluye comandos de Live 12 pertinentes a creación de proyecto, interpretación, edición, mezcla y gestión de medios. Se contrastaron las superficies de la barra de control y las vistas, navegación por teclado, preferencias, navegador, mezcla, edición MIDI/Arrangement y administración de archivos en el manual oficial. Los menús específicos de cada dispositivo/complemento los proporciona el dispositivo real; Estudio DAW no los inventa. Las opciones enumeradas son las documentadas y observables en esas secciones, pero aún no se ha cotejado cada opción condicional contra todos los estados de selección y ediciones de Live; por eso la tarea 0.2 no se da por completa.

| Menú / contexto | Opciones de Live 12 observadas en la referencia oficial | Cobertura prevista para Estudio DAW |
| --- | --- | --- |
| **Archivo** | Conjunto nuevo; abrir; lista de recientes; guardar; guardar como; guardar una copia; guardar como plantilla; recopilar todo y guardar; administrar archivos; exportar audio/vídeo y clip MIDI; cerrar conjunto y salir. El administrador busca archivos ausentes, reúne medios externos, muestra archivos sin uso y empaqueta proyectos. | Crear/abrir/recientes/guardar/guardar como/plantilla; reunir y volver a vincular medios; exportar audio/pistas separadas/MIDI; cierre/salida seguros. |
| **Edición** | Deshacer/rehacer; cortar/copiar/pegar/duplicar/eliminar; renombrar y editar texto informativo; seleccionar todo/quitar selección/invertir selección; seleccionar bucle; volver al valor predeterminado; cuantizar; dividir/consolidar según contexto; marcadores de inicio, final y bucle de clips MIDI; herramientas de nota dependientes del editor y la selección. | Bus de comandos reversible para mutaciones; modelo de selección/portapapeles; edición MIDI/audio; disponibilidad contextual e historial de edición. |
| **Crear** | Insertar pista de audio/MIDI/retorno; insertar escena; capturar e insertar escena; insertar clip MIDI vacío; importar archivo MIDI; consolidar el tiempo seleccionado en una escena nueva. El conjunto disponible varía con la vista y selección. | Crear pistas/escenas/clips tipados; capturar material de sesión activo sin interrumpirlo; importar MIDI/audio a pista/casilla elegida; convertir selecciones temporales de Arrangement en escenas. |
| **Vista** | Cambiar entre Session/Arrangement; mostrar/ocultar navegador, vista de ayuda, Clip View, Device View, mezclador y editor de envolventes; apilar vistas de clip y dispositivo; configurar secciones de mezclador: entrada/salida, envíos, retornos, volumen, opciones/retardo de pista, fundido cruzado e impacto de rendimiento; plegar/desplegar pistas; mostrar controles de pista de Arrangement, colección de grooves, administrador de archivos y pantalla completa. La disponibilidad cambia según edición, versión, plataforma y selección. | Cambio inmediato entre vistas; paneles opcionales persistentes de navegador/detalle/mezclador; canales configurables y encabezados; acercamiento/plegado/resumen; preferencias de accesibilidad y distribución guardada. |
| **Opciones (Windows) / menú Live (macOS)** | Activar/desactivar teclado MIDI de computadora; modos de asignación de teclas y MIDI; modo de dibujo; armado/reactivación de automatización; entrada por pasos; preescucha de archivos del navegador y notas del editor MIDI; accesibilidad. La ventana Ajustes/Preferencias se abre desde Opciones en Windows y desde Live en macOS. Sus páginas incluyen Pantalla y entrada, Tema y colores, Audio, Link/Tempo/MIDI, Archivo y carpeta, Biblioteca, Complementos, Grabación/Deformación/Lanzamiento y Licencias/actualizaciones. | Dispositivo/frecuencia/búfer de audio de ámbito de aplicación, puertos/asignación MIDI, pantalla/tema/teclado, rutas de medios/biblioteca, comportamiento de lanzamiento/grabación/deformación y accesibilidad. Distinguir ajustes aplicados de los que requieren reinicio. |
| **Navegación** | En Live 12 existe un menú Navegación para activar el uso de Tab como cambio de foco y llevar el foco a la barra de control, Session, Arrangement, Clip View, Device View, navegador, colección de grooves, vista de ayuda y mezclador; incluye navegación al control equivalente de pistas vecinas. | Navegar entre vistas y controles con atajos visibles; no interceptar escritura en campos; foco accesible y comportamiento documentado. |
| **Ayuda** | Vista de ayuda contextual; lecciones/recursos integrados; abrir el manual de la versión instalada; ir a recursos de ayuda; información de paquetes y Acerca de. Las notas de Live 12 documentan también la carga del conjunto de demostración desde Ayuda. La ubicación de información del producto puede variar por sistema. | Ayuda contextual, atajos/referencia, diagnósticos/registros del motor e información del producto/licencia; enlaces y contenido sólo cuando estén disponibles. |
| **Contexto del encabezado de pista** | Renombrar/información/color; insertar, duplicar, eliminar, desactivar, congelar, plegar/desplegar; ajustar alto; agrupar/vincular; carriles de tomas; armar/solo/silenciar; mostrar controles de mezclador y acciones de dispositivo/ruteo según pista. | Agregar/duplicar/renombrar/reordenar/quitar/activar, tipo/color, plegado/alto, grupo y mezcla/grabación admitidas; filtrar según tipo y rutas disponibles. |
| **Contexto de casilla/escena Session** | Lanzar/detener/seleccionar clip; insertar/capturar escena; renombrar y elegir color; duplicar/copiar/pegar/eliminar; modo de lanzamiento, legato, velocidad, cuantización, desplazamiento, bucle y acciones posteriores; lanzar escena y ajustar tempo/compás; añadir/quitar botones de parada. | Funciones creativas con estado persistido, sustitución por pista, lanzamiento de escena y reproducción cuantizada real; hacer visible cada estado de lanzamiento. |
| **Contexto de clip/selección Arrangement** | Seleccionar/dividir/duplicar/mover/cambiar tamaño/recortar/invertir; bucle; consolidar; desvanecimientos/ganancia; controles de deformación/muestra; recortar/insertar/borrar tiempo; acciones de selección/rango; notas MIDI y automatización. | Edición MIDI/audio no destructiva, forma de onda/notas, desplazamientos de fuente, desvanecimientos, ajuste y edición reversible. Omitir transformaciones no admitidas. |
| **Contexto de elemento del navegador** | Cargar/insertar/escuchar; navegar atrás/adelante; buscar/filtrar/etiquetar/coleccionar; renombrar/quitar cuando corresponda; mostrar extensiones y elegir columnas; sustituir/localizar medios ausentes; administrar proyecto/biblioteca y reunir archivos. | Búsqueda/ubicaciones/favoritos/filtros, escucha veraz, arrastrar y soltar, procedencia, revinculación y diagnóstico. Mostrar contenido licenciado instalado sólo tras descubrirlo mediante una integración real. |
| **Controles de mezclador/pista** | Activador de pista, solo/escucha de referencia, armado, panorama, volumen/medidor; selectores de entrada/salida; envíos/retornos/salida principal; retardo de pista; asignación/curva de fundido cruzado; indicadores de rendimiento; mostrar/ocultar secciones configurables. | Los mismos controles centrales en Session y Arrangement sobre valores compartidos. Habilitar ruteo/escucha/envíos/retardo/complementos/grabación sólo con soporte real de comandos y motor. |
| **Contexto del editor de notas MIDI** | Seleccionar/mover/cambiar tamaño/velocidad/probabilidad de nota; cuantizar/rejilla; agrupar/reproducir una nota; herramientas de transposición/transformación/generación; marcadores de inicio/final/bucle de clip; preescucha de nota y modos de envolventes/MPE. | Primero las funciones básicas de piano roll y luego expresión/transformaciones admitidas; las ediciones se guardan y se deshacen. Los generadores avanzados se especifican por separado. |

### Correspondencia de documentación y alcance del inventario

La documentación oficial consultada enumera explícitamente las secciones de la barra de control y las vistas, los controles de mezcla, el navegador, la navegación accesible y las páginas de preferencias. Las notas oficiales de Live 12 aportan además opciones incorporadas al menú Navegación, accesibilidad, preescucha, entrada por pasos y acciones de edición. El catálogo precedente las agrupa por menú y contexto, y deja claro cuáles dependen de la selección o de una integración.

Fuentes oficiales para verificar las familias y opciones descritas:

- [Conceptos de Live: barra de control, Session/Arrangement, pistas, mezcla y señal](https://www.ableton.com/en/live-manual/12/live-concepts/).
- [Navegación accesible y teclado](https://www.ableton.com/en/live-manual/12/accessibility-and-keyboard-navigation/).
- [Ajustes y ubicación por plataforma](https://www.ableton.com/en/live-manual/12/first-steps/).
- [Navegador y su menú de contenido](https://www.ableton.com/en/live-manual/12/working-with-the-browser/).
- [Mezclador y controles visibles](https://www.ableton.com/en/manual/mixing/) y [ruteo](https://www.ableton.com/en/manual/routing-and-i-o/).
- [Edición MIDI](https://www.ableton.com/en/manual/editing-midi/), [Arrangement](https://www.ableton.com/en/manual/arrangement-view/) y [administración de conjuntos/archivos](https://www.ableton.com/en/manual/managing-files-and-sets/).
- [Notas oficiales de lanzamiento de Live 12](https://www.ableton.com/en/release-notes/live-12/) para los cambios de menús de navegación, vista y accesibilidad.

La lista no se cuenta como exhaustiva mientras falte revisar dentro de Live cada menú principal y contextual según sistema operativo, edición de producto, tipo de pista/dispositivo y estado de selección. Las páginas públicas describen opciones por flujo, no publican una tabla única con todas esas permutaciones. La tarea 0.2 sigue abierta hasta terminar ese cotejo y asociar cada acción con el catálogo de comandos previsto.

Esto es un resumen de menús/contextos, no el inventario exhaustivo opción por opción solicitado en la tarea 0.2. Aún no establece paridad de menús. Los menús contextuales dependen de la selección y crecerán con las integraciones de dispositivos; cada comando integrado debe aparecer en el registro tipado de acciones, no copiarse como una lista de botones inertes.

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
| Interfaz/IPC del proyecto | Tauri permite nuevo/demo/abrir/guardar/guardar como, resumen de proyecto, deshacer/rehacer, transporte y ajustes guardados de perfil de audio. | Conectar por IPC los comandos tipados del bus para altas/bajas/cambios de pistas, escenas, casillas y regiones; registro general de menús/acciones y menús contextuales atentos a la selección. |
| Modelo de proyecto | Marcador JSON actual `estudio-daw.project.v5` (sólo revisión interna del formato); tipos de medio MIDI/audio y roles MIDI/instrumento/audio/bus/retorno/salida principal; estado compartido del mezclador; catálogo de fuentes de audio con ruta/firma/hash y metadatos opcionales de canales/frecuencia; regiones con ID de fuente, desplazamiento de fuente, pulso de línea de tiempo, duración en muestras, ganancia y desvanecimientos; listas ordenadas que referencian IDs de regiones; escenas/casillas que referencian clips MIDI/audio sin duplicar contenido. `Project::validate_persisted_contracts` comprueba propiedad y referencias de fuentes/listas/escenas/casillas. v0–v4 migran en memoria; las pruebas comprueban conservación de pistas y recuperación de medios/regiones v4→v5. | Mapas de canales y selección de ruta específicos del dispositivo, ruteo en ejecución para bus/retorno/salida principal, interfaz Session/Arrangement y comandos de escena/casilla, forma de onda/decodificación/reproducción. |
| Comandos de proyecto | `ProjectCommand` permite agregar/duplicar/renombrar/reordenar/quitar/activar pistas; asignar estado de mezcla; crear/renombrar/reordenar/quitar escenas; crear/quitar casillas; y agregar/recortar/mover/ajustar ganancia/desvanecimientos de regiones de audio. Las transacciones validan referencias y se pueden deshacer/rehacer. Quitar una pista elimina sus clips/lista/fuentes de proyecto y conserva archivos externos; duplicarla crea IDs nuevos sin copiar fuentes. | Conectar comandos a la interfaz Tauri; ampliar edición (división y más) y ruteo. La presencia de comandos del dominio no significa que haya controles gráficos para ellos. |
| Instantáneas/interfaz | Los resúmenes de pista llevan valores de mezcla compartidos y color. Tauri puede crear pistas MIDI/audio mediante comandos; las etiquetas del arreglo distinguen audio de cantidad de notas MIDI. | Resumen/forma de onda/estado de fuente de clips de audio, mezclador interactivo compartido, estado de entrada/salida/medidor, cursor Session/Arrangement compartido y detalle MIDI/audio editable. |
| Motor | La reproducción PipeWire crea fuentes de instrumentos MIDI y planifica eventos MIDI; existen reproducir/pausar/detener y planificación MIDI limitada por duración de clip. | Decodificación/reproducción/mezcla de regiones de audio, lanzamiento en vivo de clips/escenas por pista y cuantización, grabación/monitorización de entrada, posición determinista del transporte reflejada en Tauri. |
| Interfaz web | Acciones de proyecto, demostración MIDI, transporte de reproducción MIDI, historial, controles de perfil de audio y vista previa de sólo lectura del arreglo MIDI. | Vista Session real (hoy rotulada «PRONTO»), opciones/atajos de menú, creación/importación/reproducción de pistas de audio, mezclador, edición de clips/notas, navegador de medios del usuario y diseño aceptado visualmente inspirado en Ableton. |

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

### Las pistas de audio modelan fuentes, regiones, listas y flujo de señal

Una pista de audio es una entidad del proyecto con nombre/orden/configuración de canales persistentes, estado activa/silencio/solo, ganancia/panorama, fuentes de medios y lista ordenada no destructiva de clips/regiones. Un clip apunta a medios fuente y guarda desplazamiento de fuente, posición temporal, duración, desvanecimientos y ganancia. La ruta de señal de pista es entrada/fuente → cadena de procesamiento → panorama/ganancia → medidor → salida; cada nodo/operación se implementa sólo cuando existe su contrato de procesamiento/ruteo. Las pistas MIDI/instrumento tienen identidad propia de entrada/clip/instrumento/salida MIDI, no son una tarjeta de audio con pintura distinta.

La primera implementación debe reconocer que hoy faltan reproducción y grabación de audio. Importación/ubicación/reproducción de audio, grabación/monitorización, ruteo/procesamiento y buses son cortes verticales explícitos; no mostrar sus controles antes de que cada flujo funcione de extremo a extremo.

### Los menús describen capacidades del producto, no adornos de implementación

La organización principal de menús/vistas sigue Archivo, Edición, Crear, Vista, Opciones y Ayuda de Ableton Live, la ubicación de ajustes Live/Opciones específica de cada plataforma, además de su barra de control, navegador, Session, Arrangement, vistas Clip/Dispositivo y mezclador. Se pueden añadir acciones técnicas propias de Estudio, como configuración de audio/MIDI. Ardour aporta sólo conceptos técnicos de audio; la distribución de sus menús y organización visual no son objetivos. Una opción de menú se incorpora sólo junto con su comando, condición de disponibilidad, política de deshacer, atajo cuando corresponda, información de estado y verificación determinista. El catálogo de comandos es más amplio que el menú habilitado inicial y se entrega por etapas en tareas OpenSpec.

### Los controles de mezcla son coherentes entre editor y mezclador

Ganancia, panorama, silencio, solo y estado activo de la pista son estado compartido del proyecto y se muestran de forma coherente en Session, Arrangement y Mezclador. El armado/monitorización de entrada sólo aparece habilitado cuando funcionan la ruta de grabación y la entrada de hardware elegida. Los canales del mezclador comunican dirección de entrada, procesamiento y salida; los medidores muestran valores medidos, nunca niveles simulados.

### El lanzamiento de clips en tiempo real es una función del motor

El lanzamiento de clips/escenas usa el tempo/cuantización de sesión, mensajes de control acotados a un planificador fuera de la devolución de audio y estado de reproducción por pista. La devolución no asigna memoria ni reconstruye el plan de dispositivo/renderizado. Los eventos MIDI se detienen correctamente al reemplazar/detener. Los clips de audio se incorporarán al planificador sólo cuando exista una ruta acotada de decodificación/almacenamiento temporal.

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

## Fuentes oficiales consultadas

Revisado el 2026-09-27:

- Ableton Live 12: [Conceptos de Live y barra de control](https://www.ableton.com/en/manual/live-concepts/), [Vista Session](https://www.ableton.com/en/manual/session-view/), [Vista Arrangement](https://www.ableton.com/en/manual/arrangement-view/), [lanzamiento de clips](https://www.ableton.com/en/manual/launching-clips/), [mezcla](https://www.ableton.com/en/manual/mixing/), [navegador](https://www.ableton.com/en/live-manual/12/working-with-the-browser/), [ajustes](https://www.ableton.com/en/live-manual/12/first-steps/).
- Ardour: [índice del menú principal](https://manual.ardour.org/ardours-interface/main-menu/), [menú Session](https://manual.ardour.org/ardours-interface/main-menu/Session-menu/), [menú Transporte](https://manual.ardour.org/ardours-interface/main-menu/Transport-menu/), [menú Edición](https://manual.ardour.org/ardours-interface/main-menu/Edit-menu/), [menú Región](https://manual.ardour.org/ardours-interface/main-menu/Region-menu/), [menú Pista](https://manual.ardour.org/ardours-interface/main-menu/Track-menu/), [menú Vista](https://manual.ardour.org/ardours-interface/main-menu/View-menu/), [menú Ventana](https://manual.ardour.org/ardours-interface/main-menu/Window-menu/), [controles de pista de audio](https://manual.ardour.org/working-with-tracks/audio-track-controls/), [canales del mezclador](https://manual.ardour.org/ardours-interface/audio-midi-mixer-strips/), [sesiones y pistas](https://manual.ardour.org/working-with-tracks/), [importación de medios](https://manual.ardour.org/adding-pre-existing-material/), [regiones](https://manual.ardour.org/working-with-regions/), [flujo de matriz de disparos](https://manual.ardour.org/cue/).

La lista de Ableton refleja las superficies de aplicación, ámbitos de menú/función y páginas de configuración del manual Live 12; la lista de Ardour enumera cada familia de menú principal de primer nivel y agrupa arriba sus acciones documentadas por función. Aún no son un catálogo línea por línea de cada acción contextual según dispositivo, tipo de pista y estado de selección. La tarea 0.2 sigue abierta para completar ese catálogo; este documento de planificación no afirma que la auditoría de menús sea exhaustiva ni que la compilación actual tenga paridad de funciones.
