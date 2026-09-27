# Especificación diferencial

## ADDED Requirements

### Requirement: los menús de la estación de trabajo exponen el mapa completo de comandos

La estación DEBE organizar las acciones admitidas en familias localizables: Proyecto/Sesión, Edición, Crear, Vista/Ventana, Pista, Clip/Región, Transporte, Audio/MIDI, Ajustes/Opciones y Ayuda. DEBE ofrecer menús contextuales y atajos para flujos frecuentes. Las familias de comandos auditadas en `../../design.md` son el inventario mínimo del producto; la hoja de ruta DEBE contabilizar cada familia y PUEDE ampliarla con análisis musical, notación, programación y edición asistida propias de Estudio DAW. Cada opción DEBE corresponder a un comando tipado, una acción real de vista/estado o un adaptador de plataforma. Cada opción DEBE tener una condición de disponibilidad correcta, política de deshacer si modifica el proyecto, etiqueta accesible e información veraz de resultado/error. Una acción no implementada NO DEBE mostrarse habilitada ni completada.

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

El proyecto DEBE modelar las pistas de audio por separado de las pistas MIDI/instrumento, con identidad/orden estables, nombre/color, configuración de canales, estado activa/silencio/solo, ganancia/panorama, estado de ruteo de entrada/salida y una lista ordenada no destructiva de regiones de audio. Las regiones DEBEN referenciar medios fuente y conservar el desplazamiento dentro de la fuente, la posición temporal, la duración, los desvanecimientos y la ganancia de región. Quitar, mover o recortar una región NO DEBE borrar ni alterar silenciosamente el archivo fuente. Session, Arrangement y Mezclador DEBEN mostrar la misma identidad y estado de pista. La interfaz NO DEBE llamar «audio» a una pista sólo por su aspecto si falta la ruta de importación/reproducción.

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

La estación DEBE ofrecer una vista Session de clips/escenas y una vista Arrangement de línea de tiempo sobre las mismas pistas ordenadas, medios, estado de mezcla y transporte del proyecto. Session organiza columnas de pistas y filas de escenas; Arrangement organiza las mismas pistas verticalmente contra el tiempo musical. Cambiar de vista DEBE conservar el contenido del proyecto y el estado de reproducción. Ableton Live 12 DEBE ser la referencia visual y de interacción principal para la jerarquía creativa, densidad y relación entre ambas vistas. Ardour PUEDE consultarse únicamente para detalles técnicos de ingeniería de audio; NO DEBE definir la jerarquía visual ni el flujo creativo. El estilo visual por sí solo no satisface este requisito.

#### Scenario: abrir un proyecto en Session

- **CUANDO** un proyecto aparece en la vista creativa principal
- **ENTONCES** las columnas de pistas y filas de escenas muestran casillas tipadas, estado reproduciendo/en cola y controles de lanzamiento de escena
- **Y** cada casilla ofrece acciones acordes con su tipo de pista y clip
- **Y** las casillas vacías ofrecen acciones reales de creación/importación o un estado vacío auténtico

#### Scenario: alternar entre Session y Arrangement

- **CUANDO** la persona cambia de vista durante un proyecto
- **ENTONCES** ambas representan las mismas pistas, clips, referencias de fuentes, controles de pista y cursor de reproducción
- **Y** la reproducción no se detiene ni reinicia sólo por cambiar de vista

#### Scenario: revisar audio y MIDI en Arrangement

- **CUANDO** una pista de audio o MIDI contiene regiones
- **ENTONCES** Arrangement muestra la forma de onda real o el contenido de notas MIDI en su posición musical
- **Y** selección, movimiento, recorte, división, duplicación, bucle y ajuste a rejilla actualizan el proyecto mediante comandos y se pueden deshacer

### Requirement: las casillas y escenas de Session producen reproducción real

El lanzamiento de clips/escenas en Session DEBE compartir el reloj de transporte, tempo y compás del proyecto. La cuantización, sustitución/parada por pista y lanzamiento de escena completa DEBEN planificarse fuera de la devolución de audio mediante mensajes de control acotados; cambiar de casilla NO DEBE reconstruir ni reiniciar el dispositivo. Las notas MIDI y regiones de audio DEBEN detenerse/liberarse correctamente al terminar el clip, al sustituirlo, pausar o detener el transporte. Los ajustes de lanzamiento —cuantización, modo de repetición/lanzamiento y acciones posteriores— sólo se muestran si su comportamiento está implementado con exactitud.

#### Scenario: lanzar un clip MIDI o de audio

- **CUANDO** la persona lanza un clip admitido con ajuste de cuantización
- **ENTONCES** comienza en el límite esperado del transporte y se repite/termina según su duración persistida y ajustes de lanzamiento
- **Y** reemplazarlo detiene/libera el clip anterior de esa pista sin afectar otras pistas ni reabrir el dispositivo

#### Scenario: lanzar una escena

- **CUANDO** la persona lanza una escena
- **ENTONCES** las casillas compatibles y ocupadas comienzan juntas en el límite de cuantización compartido
- **Y** las casillas vacías siguen su comportamiento explícito de detener/continuar, sin inventar contenido

### Requirement: los encabezados de pista y canales del mezclador representan el flujo real de señal

Los encabezados de pistas de audio y MIDI/instrumento DEBEN identificar el tipo de pista y ofrecer sólo controles implementados. El mezclador y los controles DEBEN seguir el modelo compartido y configurable de Session/Arrangement de Live, mostrando valores y actividad reales, no controles o medidores simulados. Para la corrección técnica de pistas de audio, entrada, cadena de procesadores, panorama/ganancia, medidor y salida DEBEN formar un flujo legible; puede contrastarse con la descripción de canales de Ardour cuando sea útil. Los valores de ganancia/panorama/silencio/solo/activa DEBEN coincidir en todas las vistas. El armado de grabación, monitorización de entrada, ruteo, envíos, buses y procesadores sólo DEBEN habilitarse cuando esté disponible su ruta completa de ejecución.

#### Scenario: cambiar estado compartido del mezclador

- **CUANDO** la persona cambia ganancia, panorama, silencio, solo o estado activo desde una vista admitida
- **ENTONCES** el valor se guarda y coincide inmediatamente en Session, Arrangement y Mezclador
- **Y** el comando del proyecto participa en deshacer/rehacer

#### Scenario: abrir el menú contextual de una pista

- **CUANDO** la persona abre el menú contextual de una pista MIDI, audio, bus o master
- **ENTONCES** aparecen las acciones aplicables de creación, nombre, orden, edición y mezcla según el tipo
- **Y** las acciones no admitidas de ruteo, grabación o procesadores se omiten o explican por qué están deshabilitadas

### Requirement: el navegador y el editor inferior son herramientas creativas contextuales

El navegador DEBE ofrecer búsqueda y navegación de medios reales del proyecto/usuario y de instrumentos/dispositivos instalados, filtros de metadatos, escucha previa y arrastrar/soltar sólo cuando el motor lo admita. DEBE conservar la procedencia y nunca afirmar acceso a contenido sólo por su etiqueta visual. El panel contextual inferior DEBE cambiar según la selección: editor de notas MIDI para clips MIDI, controles de región/fuente para audio o parámetros reales para dispositivos cargados.

#### Scenario: buscar y escuchar medios

- **CUANDO** la persona busca o escucha un elemento de biblioteca o un archivo de audio
- **ENTONCES** los resultados provienen de contenido indexado del proyecto/usuario/instalación, con tipo y procedencia
- **Y** la escucha previa se oye y se distingue del transporte de sesión cuando se ofrece
- **Y** soltar un resultado en una pista crea una entidad de proyecto compatible a través de la ruta de comandos de aplicación

### Requirement: transporte y ajustes informan el estado real del motor

El transporte DEBE exponer reproducir/pausar/detener, posición musical, tempo/compás y las operaciones implementadas de bucle, metrónomo, navegación y grabación. Posición y reproducción DEBEN provenir del reloj de transporte de sesión y de eventos del motor. Los ajustes de audio DEBEN distinguir preferencias guardadas de ajustes aplicados al flujo activo, incluido si hace falta reiniciar. La interfaz DEBE informar errores de dispositivo en vez de indicar éxito cuando falle un comando de dominio o el inicio del motor.

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

La interfaz DEBE tener una jerarquía de estación de trabajo con superficie creativa utilizable de inmediato, organización clara de pistas/escenas, forma de onda y notas MIDI legibles, transporte compacto y persistente, navegador/editor contextual y mezclador integrado. La validación visual DEBE inspeccionar una ventana en ejecución en tamaños estándar y mínimo y recorrer los flujos principales; los colores, capturas del código fuente o una compilación no demuestran fidelidad del producto.

#### Scenario: revisar una sesión estándar

- **CUANDO** se revisa un proyecto MIDI/audio mixto en el tamaño de escritorio admitido
- **ENTONCES** Session, Arrangement, Navegador, controles de pista y Mezclador se leen como una estación conectada
- **Y** las superficies musicales dominan el espacio disponible, en vez de tarjetas de tablero o secciones genéricas de página web

#### Scenario: revisar el tamaño mínimo de ventana

- **CUANDO** la ventana está en sus dimensiones mínimas admitidas
- **ENTONCES** transporte, identidad de pista, vista seleccionada y acciones esenciales de edición/lanzamiento siguen accesibles sin superponerse
- **Y** los paneles opcionales se contraen o desplazan sin ocultar la superficie activa de pistas
