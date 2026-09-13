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

Vive en el repositorio en `docs/prompt-master.md`, se compila dentro del binario
y se inyecta en esta página al construir el sitio: las instrucciones instaladas
y el sitio no pueden separarse.

## Por qué está en inglés

El protocolo se instala en inglés y le pide al agente responder en el idioma en
que escribe la persona. Los nombres de herramientas, los argumentos, los ids de
Records, los valores de campos, las rutas y los comandos se mantienen tal cual.
Así un único texto sirve a cualquier equipo, y lo que lees aquí es exactamente
lo que tu agente tiene instalado: no hay una traducción que pueda quedarse
atrás.

Si le escribes en español, tu agente te responde en español.

## Qué le pide al agente

Localizar con Codebase Memory, llamar a `prepare_change` antes de un cambio no
trivial, declarar explícitamente cualquier Record gobernante o conflicto, hacer
el cambio mínimo y cerrar con `finalize_change` llevando solo conocimiento
durable — una decisión por Record. Si un candidato choca con una regla fijada,
el agente se detiene, le pregunta a la persona y llama a `resolve_conflict` con
su respuesta literal.

El protocolo es el piso que recibe cada conversación. Para profundizar en cada
paso, remite al agente al [skill `rationale`](/es/docs/skill).

## Copiar

El bloque de abajo es la fuente canónica, la misma que se instala.
