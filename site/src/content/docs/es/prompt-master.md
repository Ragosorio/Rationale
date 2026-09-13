---
lang: es
slug: prompt-master
title: Prompt maestro
description: El protocolo de invocación que install-agent escribe para tus agentes — preparar, cambiar, capturar y entregar los conflictos a una persona.
section: Empezar
order: 3
---

## Se instala solo

`rationale install-agent` escribe este protocolo en `CLAUDE.md`, `AGENTS.md` o
la regla de Cursor, dentro de un bloque delimitado que `uninstall-agent` retira
con exactitud. Solo necesitas pegarlo a mano en un cliente que Rationale no
configura.

La versión que se instala es la inglesa, `docs/prompt-master.md`, compilada en
el binario. Esta traducción vive en `docs/prompt-master.es.md` y un chequeo de
CI exige que ambas tengan los mismos pasos.

La versión en inglés está disponible en [master prompt](/docs/prompt-master).

## Qué le pide al agente

Localizar con Codebase Memory, llamar a `prepare_change` antes de un cambio no
trivial, declarar explícitamente cualquier Record gobernante o conflicto, hacer
el cambio mínimo y cerrar con `finalize_change` llevando solo conocimiento
durable — una decisión por Record. Si un candidato choca con una regla fijada,
el agente se detiene, le pregunta a la persona y llama a `resolve_conflict` con
su respuesta literal.

## Copiar

El bloque de abajo es la fuente canónica en español.
