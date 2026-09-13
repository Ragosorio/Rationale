---
lang: es
slug: concepts
title: Conceptos clave
description: El modelo pequeño detrás de Rationale — Records, bindings, procedencia, autoridad, relaciones y operaciones.
section: Empezar
order: 2
---

## Record

Un **Record** es una pieza de conocimiento durable sobre el código: una
`constraint` (lo que debe seguir siendo cierto), una `decision` (por qué es
así), un `risk` o una `exception`. Tiene un statement, un rationale que da la
causa, una severidad y su lifecycle. Los Records viven como YAML en
`.rationale/records/` y se versionan con el proyecto.

**Una decisión por Record.** Si dos partes podrían reemplazarse o revocarse por
separado, son dos Records.

## Binding

Un binding ata un Record al código que gobierna: un archivo (`src/retry.rs`) o
un símbolo (`src/retry.rs::backoff`). Los bindings de símbolo los confirma el
proveedor estructural y se guardan con un id portable, sin rutas de una máquina
concreta. Los que nacen de código sin commitear quedan marcados `provisional`.

## Procedencia

Cada Record dice de dónde viene:

- `agent_asserted` — lo escribió un agente mediante `finalize_change`, con el
  cliente, la sesión y la operación que lo afirmaron;
- `human_stated` — lo declaró una persona;
- `migrated` — viene del flujo de aprobación anterior a 1.0.

La procedencia nunca se asciende en silencio: lo que afirmó un agente no se
presenta como algo que declaró una persona.

## Autoridad

- `normal` — por defecto. Un Record más nuevo puede reemplazarlo de forma
  explícita.
- `pinned` — una regla que el proyecto fijó. Los agentes la usan, pero
  cualquier intento de reemplazarla se convierte en un **conflicto** que solo
  resuelve una persona.

Fijar, desfijar y adoptar un reemplazo sobre un Record fijado exigen un actor
declarado bajo `authority:` en `.rationale/config.yaml`.

## El porqué de las relaciones

Un Record también puede explicar **por qué existe una relación** — por qué
`checkout` llama a `reserve_stock`, por ejemplo. Cada vez que se compila
contexto, Rationale deriva el estado estructural de esa relación desde el
proveedor:

| Estado | Significado |
|---|---|
| `observed` | La relación directa existe en el índice actual. |
| `indirect` | Ya no es directa, pero un camino acotado sigue conectando los extremos. |
| `orphaned` | No se puede localizar; la explicación puede estar obsoleta. Nunca se borra. |
| `unknown` | El proveedor no pudo verificarla. La ausencia no es prueba. |

## Operación

`prepare_change` abre una **operación** con un `operation_id`. Registra qué se
consideró y qué se seleccionó (nodos, relaciones, Records, tamaño del packet)
como snapshot local. `finalize_change` cierra la misma operación, así el
Control Room muestra el cambio completo, del contexto a la memoria capturada.

## Canónico frente a derivado

`.rationale/` es la única fuente de verdad. La cache SQLite de búsqueda en
`~/.cache/rationale/`, los snapshots de operación y la actividad en
`.rationale-local/` son derivados o locales, y se pueden borrar sin perder
ninguna decisión.
