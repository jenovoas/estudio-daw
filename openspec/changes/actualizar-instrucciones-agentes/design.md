# Design

## Context

`AGENTS.md` es el único archivo de instrucciones de agente presente en la raíz del repositorio. Ya preserva límites técnicos específicos y una bitácora auditable, pero contiene rutas absolutas, un orden parcial de consulta del vault y referencias a herramientas de otro entorno.

## Goals / Non-Goals

**Goals:**

- Hacer que las instrucciones comunes sean portables y aplicables por los agentes que leen `AGENTS.md`.
- Mantener las reglas técnicas y de dominio vigentes del proyecto.
- Indicar que la disponibilidad de cada herramienta se verifica en la sesión y no se presupone.

**Non-Goals:**

- Cambiar arquitectura, requisitos del producto, dependencias, hooks, código o despliegues.
- Reescribir los handoffs históricos ni eliminar su contexto.
- Afirmar que todo agente del entorno consume `AGENTS.md` automáticamente.

## Decisions

- Actualizar el bloque de entrada común en `AGENTS.md`, sin duplicar archivos globales ni reemplazar las secciones específicas del proyecto. Así las reglas de repositorio permanecen junto a sus restricciones y no se introduce otra fuente de verdad.
- Usar referencias del vault relativas a su raíz y seguir el orden vigente: `HANDOFF.md`, `MAPA_VAULT`, `Pendientes`, preferencias, convención y contexto del proyecto. Esto evita depender de rutas absolutas.
- Nombrar las herramientas actuales (`obsidian-vault`, Graphify, Playwright, Context7 y documentación de OpenAI) como capacidades condicionales: verificar disponibilidad y no inventar resultados cuando falten.
- Mantener OpenSpec como flujo para cambios no triviales y dejar explícito que commit, push y despliegue requieren autorización correspondiente. No alterar el estado de Git ni publicar cambios.

## Risks / Trade-offs

- Algunos agentes pueden no cargar `AGENTS.md` → limitar el cambio a los archivos de entrada verificados y no afirmar cobertura de runtime fuera del repositorio.
- La lista de MCP disponibles puede variar entre perfiles → describirlos como herramientas del entorno que se comprueban, no como garantías universales.
- El archivo contiene handoffs históricos con datos de estado potencialmente antiguos → no editarlos en esta tarea; el cambio se limita a las reglas permanentes.

## Migration Plan

Actualizar el bloque permanente de instrucciones en `AGENTS.md`, conservar sus secciones de proyecto y validar el diff y OpenSpec. No hay despliegue ni migración de datos; la reversión consiste en revertir únicamente los cambios propios si se solicita.
