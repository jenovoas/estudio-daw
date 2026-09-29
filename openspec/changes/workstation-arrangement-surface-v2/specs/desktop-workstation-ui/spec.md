# Especificación diferencial

## ADDED Requirements

### Requirement: la identidad visual de cada pista pertenece a la persona

Cada pista DEBE (MUST) conservar color, marca visual y nota asignados por la persona en el proyecto. El color identifica mentalmente el instrumento o su papel musical; NO DEBE derivarse del orden de pista ni del formato MIDI/audio. La identidad DEBE mantenerse en Arreglo, Session, Mezclador, clips y editor de notas. Editarla DEBE usar un comando reversible y guardar el cambio con el proyecto. Las marcas y notas DEBEN poder editarse desde el encabezado de pista y tener nombres accesibles; el tipo de medio puede indicarse aparte sin reemplazar la identidad musical.

#### Scenario: asignar identidad musical a una pista

- **CUANDO** la persona cambia el color, elige una marca para reconocer el instrumento o escribe una nota sobre el papel/toma
- **ENTONCES** los encabezados, clips y editor de notas reflejan el color y la marca en todas las vistas conectadas
- **Y** la nota queda asociada a esa pista y disponible como referencia al volver a ella
- **Y** deshacer y rehacer restauran el conjunto de valores

#### Scenario: conservar la identidad al reorganizar pistas

- **CUANDO** se mueve o cambia de vista una pista MIDI o de audio
- **ENTONCES** conserva su color, marca y nota sin importar su posición o formato de medio
- **Y** sus clips mantienen el mismo color que el encabezado de pista

### Requirement: los instrumentos y complementos aparecen sólo cuando están cargados de verdad

La asignación de instrumento a una pista DEBE (MUST) representar una instancia ejecutable y persistir su identidad y estado recuperable. La persona DEBE poder cargarla desde el flujo de pista y volver a abrir sus controles. Analog Lab y sus VST instalados junto con él en el prefijo Wine de Arch son un objetivo explícito. Estudio DAW DEBE permitir cargar el VST real en una pista con su GUI, MIDI, audio y estado; también DEBE conservar la opción de abrir/reutilizar Analog Lab standalone y conectar sus puertos MIDI/audio. La persona usuaria confirma que la aplicación independiente ya funciona sin latencia perceptible. La integración VST debe encontrar un adaptador Wine/bridge compatible con los plugins instalados; no se exige soporte nativo del fabricante para Linux. Ninguna de las dos rutas puede presentarse mediante nombres, presets o controles simulados. El lanzamiento, bridge, comunicación y ruteo NO DEBEN bloquear ni hacer I/O desde el callback de audio; se deben medir latencia y estabilidad por ruta.

#### Scenario: cargar el VST de Analog Lab en una pista

- **CUANDO** la persona elige el VST de Analog Lab instalado en Wine desde el flujo de instrumento
- **ENTONCES** se crea una instancia real en la pista, se procesa MIDI/audio y se conserva su estado recuperable
- **Y** se puede abrir la GUI nativa y automatizar o editar sus controles disponibles
- **Y** cualquier error de host/bridge aparece como diagnóstico concreto, sin sustitución por otro sintetizador

#### Scenario: abrir Analog Lab standalone como instrumento externo

- **CUANDO** la persona asigna Analog Lab a una pista MIDI y elige sus puertos disponibles
- **ENTONCES** el DAW abre o reutiliza Analog Lab en Wine, persiste su asociación y destino MIDI, envía los eventos de esa pista y mezcla el retorno de audio en la misma pista
- **Y** puede abrir la ventana real de Analog Lab desde los controles de instrumento de la pista
- **Y** presenta un diagnóstico accionable si no existe la aplicación o alguno de los puertos seleccionados

#### Scenario: recuperar una asignación de Analog Lab o su VST

- **CUANDO** se vuelve a abrir un proyecto con una pista asignada a Analog Lab
- **ENTONCES** se restaura la instancia alojada o la aplicación/prefijo y asociación de puertos de la ruta standalone
- **Y** se valida la disponibilidad del instrumento/bridge o aplicación/puertos antes de iniciar reproducción
- **Y** no se reemplaza silenciosamente la instancia por SineSynth, FluidSynth u otra fuente
- **Y** el estado de la aplicación y de sus puertos refleja la conexión externa real

#### Scenario: Analog Lab o alguno de sus puertos no está disponible

- **CUANDO** el proyecto se abre sin Analog Lab, sin su destino MIDI o sin su retorno de audio
- **ENTONCES** la pista conserva su identidad musical y muestra qué parte de la ruta externa falta
- **Y** el sistema no reemplaza Analog Lab silenciosamente ni simula sus controles o su audio

### Requirement: los menús de la estación de trabajo exponen el mapa completo de comandos

La estación DEBE (MUST) organizar las acciones admitidas en familias localizables: Proyecto/Sesión, Edición, Crear, Vista/Ventana, Pista, Clip/Región, Transporte, Audio/MIDI, Ajustes/Opciones y Ayuda. DEBE ofrecer menús contextuales y atajos para flujos frecuentes. Las familias de comandos auditadas en `../../design.md` son el inventario mínimo del producto; la hoja de ruta DEBE contabilizar cada familia y PUEDE ampliarla con análisis musical, notación, programación y edición asistida propias de Estudio DAW. Cada opción DEBE corresponder a un comando tipado, una acción real de vista/estado o un adaptador de plataforma. Cada opción DEBE tener una condición de disponibilidad correcta, política de deshacer si modifica el proyecto, etiqueta accesible e información veraz de resultado/error. Una acción no implementada NO DEBE mostrarse habilitada ni completada.

#### Scenario: revisar los menús del producto

- **CUANDO** la persona abre un menú principal o contextual
- **ENTONCES** los comandos admitidos se agrupan según el flujo de proyecto, edición, pista, clip, transporte, vista o dispositivo
- **Y** las acciones deshabilitadas por la selección, el proyecto o el dispositivo indican el motivo concreto
- **Y** ningún control decorativo aparenta ejecutar una capacidad inexistente

#### Scenario: ejecutar una acción de menú y su atajo

- **CUANDO** la persona elige una opción habilitada o su atajo mostrado
- **ENTONCES** ambos ejecutan la misma operación tipada
- **Y** el resultado, el estado del proyecto y el estado de deshacer/rehacer coinciden

### Requirement: las pistas de audio son entidades completas de primer nivel

El proyecto DEBE (MUST) modelar las pistas de audio por separado de las pistas MIDI/instrumento, con identidad/orden estables, nombre/color, configuración de canales, estado activa/silencio/solo, ganancia/panorama, estado de ruteo de entrada/salida y una lista ordenada no destructiva de regiones de audio. Las regiones DEBEN referenciar medios fuente y conservar el desplazamiento dentro de la fuente, la posición temporal, la duración, los desvanecimientos y la ganancia de región. Quitar, mover o recortar una región NO DEBE borrar ni alterar silenciosamente el archivo fuente. Session, Arrangement y Mezclador DEBEN mostrar la misma identidad y estado de pista. La interfaz NO DEBE llamar «audio» a una pista sólo por su aspecto si falta la ruta de importación/reproducción.

#### Scenario: crear y administrar una pista de audio

- **CUANDO** la persona crea, nombra, reordena, duplica, silencia, activa, pone en solo o quita una pista de audio
- **ENTONCES** el bus de comandos del proyecto valida la operación y permite deshacerla/rehacerla
- **Y** Session, Arrangement y Mezclador muestran el mismo orden y estado
- **Y** no se permite añadir a esa pista contenido exclusivo de MIDI

#### Scenario: importar y editar medios de audio

- **CUANDO** la persona importa audio tras elegir si copia o vincula el archivo y cómo asigna sus canales
- **ENTONCES** se conserva la procedencia de la fuente y sus metadatos técnicos
- **Y** aparece una región no destructiva en la posición musical/temporal elegida, con forma de onda real y duración/desplazamiento de fuente editables
- **Y** deshacer la edición de región deja intacto el archivo fuente

#### Scenario: reproducir y rutear una pista de audio

- **CUANDO** el transporte reproduce un proyecto con regiones de audio importadas
- **ENTONCES** cada región se procesa sólo durante su duración en la línea de tiempo, con su desplazamiento de fuente, desvanecimientos y ganancia, y llega a la salida real seleccionada a través de la pista/mezclador
- **Y** la devolución de audio permanece acotada y no asigna memoria
- **Y** seleccionar una entrada, armar o monitorizar una pista sólo está habilitado cuando funciona la ruta de captura/dispositivo correspondiente

### Requirement: Session y Arrangement son vistas conectadas sobre pistas compartidas

La estación DEBE (MUST) ofrecer una vista Session de clips/escenas y una vista Arrangement de línea de tiempo sobre las mismas pistas ordenadas, medios, estado de mezcla y transporte del proyecto. Session organiza columnas de pistas y filas de escenas; Arrangement organiza las mismas pistas verticalmente contra el tiempo musical. Cambiar de vista DEBE conservar el contenido del proyecto y el estado de reproducción. Ableton Live 12 DEBE ser la referencia visual y de interacción principal para la jerarquía creativa, densidad y relación entre ambas vistas. Ardour PUEDE consultarse únicamente para detalles técnicos de ingeniería de audio; NO DEBE definir la jerarquía visual ni el flujo creativo. El estilo visual por sí solo no satisface este requisito.

La selección actual de pistas DEBE permanecer coherente al alternar entre Session, Arrangement y Mezclador. La pertenencia a un grupo organizativo DEBE persistir en el proyecto y poder asignarse o quitarse mediante una operación reversible. Agrupar pistas no DEBE enlazar ni propagar activa/silencio/solo/ganancia/panorama: cada pista conserva su propio estado de mezcla.

En Arrangement, la persona DEBE poder ajustar la altura de cada carril con el ratón y el teclado. El encabezado y el carril temporal DEBEN conservar la misma altura; esta preferencia local de interfaz DEBE sobrevivir a las actualizaciones del snapshot y a la reapertura del proyecto, sin modificar el modelo musical.

#### Scenario: ajustar la altura de un carril de Arreglo

- **DADO** un proyecto abierto con una o más pistas en Arrangement
- **CUANDO** la persona arrastra el separador de altura de una pista o lo ajusta con las flechas
- **ENTONCES** el encabezado y el carril de esa pista cambian juntos, sin mover los clips en el tiempo
- **Y** la altura respeta sus límites y se conserva al refrescar o volver a abrir el proyecto
- **Y** la operación no crea comandos ni cambios en el historial del proyecto

#### Scenario: abrir un proyecto en Session

- **CUANDO** un proyecto aparece en la vista creativa principal
- **ENTONCES** las columnas de pistas y filas de escenas muestran casillas tipadas, estado reproduciendo/en cola y controles de lanzamiento de escena
- **Y** cada casilla ofrece acciones acordes con su tipo de pista y clip
- **Y** las casillas vacías ofrecen acciones reales de creación/importación o un estado vacío auténtico

#### Scenario: alternar entre Session y Arrangement

- **CUANDO** la persona cambia de vista durante un proyecto
- **ENTONCES** ambas representan las mismas pistas, clips, referencias de fuentes, controles de pista y cursor de reproducción
- **Y** la reproducción no se detiene ni reinicia sólo por cambiar de vista

#### Scenario: seleccionar y agrupar pistas manteniendo mezclas independientes

- **CUANDO** la persona selecciona pistas en cualquiera de las superficies y les asigna o quita un grupo
- **ENTONCES** la selección visible se mantiene al alternar entre Session, Arrangement y Mezclador
- **Y** el nombre de grupo se guarda para cada pista seleccionada y la operación puede deshacerse
- **Y** cambiar la mezcla de una pista no modifica los valores de mezcla de las demás pistas del grupo

#### Scenario: revisar audio y MIDI en Arrangement

- **CUANDO** una pista de audio o MIDI contiene regiones
- **ENTONCES** Arrangement muestra la forma de onda real o el contenido de notas MIDI en su posición musical
- **Y** selección, movimiento, recorte, división, duplicación, bucle y ajuste a rejilla actualizan el proyecto mediante comandos y se pueden deshacer

#### Scenario: dividir un clip MIDI en el cursor

- **CUANDO** el cursor de edición cae estrictamente dentro de un clip MIDI y la persona elige «Dividir en cursor»
- **ENTONCES** el sistema crea dos clips contiguos mediante una transacción reversible, conservando pista, PPQ, tempo y los eventos de cada lado
- **Y** las notas activas se cierran al final del primero y se rearticulan al inicio del segundo; el estado previo recuperable de CC, pitch bend, presión y programa se restablece en el segundo
- **Y** un Note Off que coincide con el corte cierra la nota del primer clip, y SysEx anterior al corte no se vuelve a emitir desde el segundo
- **Y** el comando no se ofrece cuando el cursor está en un extremo o fuera del clip

### Requirement: las casillas y escenas de Session producen reproducción real

El lanzamiento de clips/escenas en Session DEBE (MUST) compartir el reloj de transporte, tempo y compás del proyecto. La cuantización, sustitución/parada por pista y lanzamiento de escena completa DEBEN planificarse fuera de la devolución de audio mediante mensajes de control acotados; cambiar de casilla NO DEBE reconstruir ni reiniciar el dispositivo. Las notas MIDI y regiones de audio DEBEN detenerse/liberarse correctamente al terminar el clip, al sustituirlo, pausar o detener el transporte. Los ajustes de lanzamiento —cuantización, modo de repetición/lanzamiento y acciones posteriores— sólo se muestran si su comportamiento está implementado con exactitud.

#### Scenario: lanzar un clip MIDI o de audio

- **CUANDO** la persona lanza un clip admitido con ajuste de cuantización
- **ENTONCES** comienza en el límite esperado del transporte y se repite/termina según su duración persistida y ajustes de lanzamiento
- **Y** reemplazarlo detiene/libera el clip anterior de esa pista sin afectar otras pistas ni reabrir el dispositivo

#### Scenario: lanzar una escena

- **CUANDO** la persona lanza una escena
- **ENTONCES** las casillas compatibles y ocupadas comienzan juntas en el límite de cuantización compartido
- **Y** las casillas vacías siguen su comportamiento explícito de detener/continuar, sin inventar contenido

### Requirement: los encabezados de pista y canales del mezclador representan el flujo real de señal

Los encabezados de pistas de audio y MIDI/instrumento DEBEN (MUST) identificar el tipo de pista y ofrecer sólo controles implementados. El mezclador y los controles DEBEN seguir el modelo compartido y configurable de Session/Arrangement de Live, mostrando valores y actividad reales, no controles o medidores simulados. Para la corrección técnica de pistas de audio, entrada, cadena de procesadores, panorama/ganancia, medidor y salida DEBEN formar un flujo legible; puede contrastarse con la descripción de canales de Ardour cuando sea útil. Los valores de ganancia/panorama/silencio/solo/activa DEBEN coincidir en todas las vistas. El armado de grabación, monitorización de entrada, ruteo, envíos, buses y procesadores sólo DEBEN habilitarse cuando esté disponible su ruta completa de ejecución.

#### Scenario: cambiar estado compartido del mezclador

- **CUANDO** la persona cambia ganancia, panorama, silencio, solo o estado activo desde una vista admitida
- **ENTONCES** el valor se guarda y coincide inmediatamente en Session, Arrangement y Mezclador
- **Y** el comando del proyecto participa en deshacer/rehacer

#### Scenario: abrir el menú contextual de una pista

- **CUANDO** la persona abre el menú contextual de una pista MIDI, audio, bus o master
- **ENTONCES** aparecen las acciones aplicables de creación, nombre, orden, edición y mezcla según el tipo
- **Y** las acciones no admitidas de ruteo, grabación o procesadores se omiten o explican por qué están deshabilitadas

### Requirement: el navegador y el editor inferior son herramientas creativas contextuales

El navegador DEBE (MUST) ofrecer búsqueda y navegación de medios reales del proyecto/usuario y de instrumentos/dispositivos instalados, filtros de metadatos, escucha previa y arrastrar/soltar sólo cuando el motor lo admita. DEBE conservar la procedencia y nunca afirmar acceso a contenido sólo por su etiqueta visual. El panel contextual inferior DEBE cambiar según la selección: editor de notas MIDI para clips MIDI, controles de región/fuente para audio o parámetros reales para dispositivos cargados.

#### Scenario: buscar y escuchar medios

- **CUANDO** la persona busca o escucha un elemento de biblioteca o un archivo de audio
- **ENTONCES** los resultados provienen de contenido indexado del proyecto/usuario/instalación, con tipo y procedencia
- **Y** la escucha previa se oye y se distingue del transporte de sesión cuando se ofrece
- **Y** soltar un resultado en una pista crea una entidad de proyecto compatible a través de la ruta de comandos de aplicación

### Requirement: transporte y ajustes informan el estado real del motor

El transporte DEBE (MUST) exponer reproducir/pausar/detener, posición musical, tempo/compás y las operaciones implementadas de bucle, metrónomo, navegación y grabación. Posición y reproducción DEBEN provenir del reloj de transporte de sesión y de eventos del motor. Los ajustes de audio DEBEN distinguir preferencias guardadas de ajustes aplicados al flujo activo, incluido si hace falta reiniciar. La interfaz DEBE informar errores de dispositivo en vez de indicar éxito cuando falle un comando de dominio o el inicio del motor.

El rango de repetición del transporte, cuando se define, DEBE persistir en ticks de transporte, validar que el final sea posterior al inicio y admitir deshacer/rehacer. Guardar el rango no implica que la reproducción en bucle esté activa; el control de loop sólo se presenta como funcional cuando el motor coordina el salto de audio y MIDI.

#### Scenario: definir un rango de repetición

- **CUANDO** la persona fija los puntos A y B desde el cursor del Arreglo
- **ENTONCES** el rango se guarda en el proyecto a 960 ticks por negra y puede deshacerse
- **Y** el sistema rechaza un rango cuyo final no sigue al inicio
- **Y** la interfaz no anuncia repetición activa hasta que el motor ejecute ambos medios en bucle

#### Scenario: pausar y reanudar

- **CUANDO** la persona pausa durante la reproducción y luego la reanuda
- **ENTONCES** posición del transporte, planificador de eventos, voces y consumo de audio permanecen alineados
- **Y** el tiempo de pared transcurrido durante la pausa no avanza el contenido musical

### Requirement: la fidelidad de interfaz se verifica como flujo de producción musical

La interfaz DEBE (MUST) tener una jerarquía de estación de trabajo con superficie creativa utilizable de inmediato, organización clara de pistas/escenas, forma de onda y notas MIDI legibles, transporte compacto y persistente, navegador/editor contextual y mezclador integrado. La validación visual DEBE inspeccionar una ventana en ejecución en tamaños estándar y mínimo y recorrer los flujos principales; los colores, capturas del código fuente o una compilación no demuestran fidelidad del producto.

#### Scenario: revisar una sesión estándar

- **CUANDO** se revisa un proyecto MIDI/audio mixto en el tamaño de escritorio admitido
- **ENTONCES** Session, Arrangement, Navegador, controles de pista y Mezclador se leen como una estación conectada
- **Y** las superficies musicales dominan el espacio disponible, en vez de tarjetas de tablero o secciones genéricas de página web

#### Scenario: revisar el tamaño mínimo de ventana

- **CUANDO** la ventana está en sus dimensiones mínimas admitidas
- **ENTONCES** transporte, identidad de pista, vista seleccionada y acciones esenciales de edición/lanzamiento siguen accesibles sin superponerse
- **Y** los paneles opcionales se contraen o desplazan sin ocultar la superficie activa de pistas

#### Scenario: revisar la distribución propuesta para Luna

- **CUANDO** se ejecuta el rediseño de 3.1 con un proyecto cargado en áreas interiores de 1920×1080 y 1280×720 a zoom 100 %
- **ENTONCES** transporte, posición y cambio Arreglo/Session permanecen visibles sin desplazamiento horizontal oculto ni scroll de página
- **Y** Arreglo conserva cabeceras, regla y clips alineados, y Session presenta escenas a la derecha con mezcla alineada por pista
- **Y** el detalle Clip/Dispositivo puede coexistir con la mezcla integrada y conserva selección/foco al cambiar de superficie
- **Y** el navegador muestra contenido real del proyecto; una Demo MIDI permite navegar sus clips sin inventar bibliotecas ni preescucha MIDI
- **Y** los cambios de distribución conservan las operaciones, condiciones de disponibilidad y controles de seguridad existentes

#### Scenario: evaluar el cierre del rediseño

- **CUANDO** se presenta el resultado de 3.1 a la persona usuaria
- **ENTONCES** se comparan estados equivalentes antes/después en la aplicación en ejecución y se informa qué comprobaciones visuales y funcionales se realizaron
- **Y** la tarea sigue abierta si falta la aceptación explícita de la dirección visual o parte de su alcance
- **Y** ni una captura estática ni la aceptación estética se presentan como prueba de audio, latencia física o cierre de otras tareas funcionales
