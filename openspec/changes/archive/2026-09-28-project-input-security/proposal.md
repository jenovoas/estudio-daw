# Propuesta

## Por qué

Un `project.json` puede contener rutas de ejecutables y configuración de Wine que llegan al proceso de reproducción; abrir un proyecto no confiable y pulsar Play puede ejecutar código sin una aprobación local. La auditoría también encontró rutas capaces de entrar en pánico o hacer trabajo dinámico dentro del callback de audio, por lo que hace falta formalizar el límite de confianza del proyecto y reforzar el contrato de tiempo real.

## Qué cambia

- Tratar la configuración persistida del proyecto como datos no confiables: no concederle autoridad para ejecutar aplicaciones, cargar plugins ni definir variables de entorno del proceso.
- Exigir aprobación local explícita para ejecutar una aplicación standalone o cargar un plugin externo, y registrar confianza sobre la identidad canónica del binario fuera del proyecto.
- Resolver el prefijo Wine desde configuración local confiable; no aplicar un `WINEPREFIX` suministrado por el proyecto.
- Validar rutas e índices de canales antes de activar el plan y mantener acceso a canales seguro dentro del callback.
- Retirar del callback las asignaciones, liberaciones de memoria y locks asociados a comandos de clips; conservar límites acotados y propiedad segura de los buffers.
- Evitar que un mutex envenenado del helper VST3 o la ausencia de un dispositivo/backend CPAL termine en un pánico del helper o de la aplicación; devolver diagnósticos estructurados.
- Dejar de versionar la base comprimida generada por codebase-memory.
- Mantener fuera de alcance los once hallazgos de severidad baja que no están enumerados en el resumen recibido, hasta cotejarlos individualmente.

## Capacidades

### Capacidades nuevas

- `project-security`: define la confianza local requerida para lanzar ejecutables o cargar plugins a partir de referencias persistidas en un proyecto.

### Capacidades modificadas

- `audio-render-chain`: refuerza el contrato del callback para incluir liberaciones de memoria, locks y accesos indexados derivados de configuración del proyecto.
- `runtime-midi-playback`: requiere errores estructurados y estado de transporte coherente cuando no puede iniciarse el backend/dispositivo de audio o falla un helper de instrumento.

## Impacto

- Flujos Tauri de Play y asignación de instrumentos standalone/VST3, configuración local de confianza y prefijos Wine.
- Compilación del plan de audio y publicación de comandos de Session hacia `AudioClipMixerNode`.
- Gestión de errores en el helper VST3 y selección del backend CPAL.
- Validación del modelo/configuración del proyecto, artefactos OpenSpec y `.gitignore`/seguimiento de `.codebase-memory/graph.db.zst`.
- Los proyectos existentes conservan sus referencias musicales; los instrumentos externos requerirán aprobación local la primera vez y el prefijo Wine guardado en el proyecto dejará de controlar el entorno de ejecución.
