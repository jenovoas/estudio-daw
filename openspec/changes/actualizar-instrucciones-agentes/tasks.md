# Tasks

## 1. Alinear instrucciones comunes

- [x] 1.1 Actualizar `AGENTS.md` con idioma, protocolo vigente del vault, herramientas condicionales, OpenSpec y límites de autorización; verificar con revisión del diff que se conservaron las reglas técnicas del proyecto.
- [x] 1.2 Retirar la instrucción activa sobre `codebase-memory` no disponible y documentar el uso de Graphify sólo cuando esté instalado y exista `graphify-out/`; verificar que no queden instrucciones activas a esa herramienta heredada. Se preserva una mención dentro del handoff histórico.

## 2. Validar y registrar

- [x] 2.1 Validar el cambio con `openspec validate actualizar-instrucciones-agentes --strict` y `git diff --check`; registrar el resultado y el estado local sin afirmar despliegue.
