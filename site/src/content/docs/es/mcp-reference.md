---
lang: es
slug: mcp-reference
title: Referencia MCP
description: Las cinco herramientas y los seis prompts en la frontera con el agente — entradas, salidas y lo que MCP nunca decide.
section: Operar
order: 8
---

## `health`

Identidad del proyecto, revisión de Git, working tree, estado y cobertura del
proveedor. Si falta Codebase Memory, el resultado es explícitamente degradado;
nunca se inventa cobertura.

## `prepare_change`

Entrada: `target` (`path` o `path::symbol`) y la `intent` real del agente.
Opcional: `max_tokens` (por defecto 2400), `max_nodes` (12),
`max_relationships` (16), `mode: baseline` para omitir la detección de
conflictos con la intención.

Salida: un `operation_id` y un packet con

- `critical_constraints` y `decisions` que gobiernan el target, cada una con
  `authority`, `provenance` y cómo coincidió;
- `relationships` explicadas por Records, con su `state` derivado y el camino
  cuando es `indirect`;
- `structure`: los nodos y aristas seleccionados alrededor del target, cada uno
  con su rol y los ids de Records que lo explican;
- `relevant_code`, `intent_conflicts`, `known_risks`, `known_unknowns`,
  `warnings`, el `snapshot` de consistencia y `budget_overflow` cuando el
  contexto con autoridad excedió el techo.

El presupuesto es un techo, nunca una meta: una vecindad vacía produce un
packet pequeño, y el conocimiento gobernante nunca se recorta para caber.

## `explain_target`

Por qué existe un target: los Records que lo gobiernan por binding exacto, su
Subject y qué es conocido frente a desconocido. Usa el mismo matcher que
`prepare_change`, así las dos herramientas nunca discrepan.

## `finalize_change`

Entrada: `operation_id`, `summary` y `candidates`. Cada candidato exige `kind`,
`statement`, `rationale`, `durability` y `bindings`; puede añadir `supersedes`,
`severity`, `id`, `risks`, `evidence` y `subject`. La base del diff es el
`base_revision` declarado, si no el HEAD que vio `prepare_change`, y si no HEAD.

Salida: los Records `committed`, los candidatos `discarded` con su motivo, los
`conflicts`, los Records `superseded`, el estado derivado de las relaciones que
explican los Records nuevos, las `signals` observadas y la captura mecánica del
cambio.

Entre los motivos de descarte están `transient`, `duplicate`,
`missing_rationale`, `rationale_restates_statement`, `mechanical_noise`,
`no_meaningful_binding` y `legacy_contract` (una llamada anterior a 1.0 sin
candidatos).

## `resolve_conflict`

Entrada: `conflict_id`, `decision` (`keep_pinned` o `adopt_new`) y
`human_answer` — la respuesta literal de la persona, que se guarda para
auditoría. Sin `human_answer` la herramienta se niega. `adopt_new` exige que el
actor de Git esté declarado en `.rationale/config.yaml`.

## Prompts

`prompts/list` devuelve seis acciones desde la misma fuente que las skills de
Claude Code:

| Prompt | Propósito | Argumentos |
| --- | --- | --- |
| `preflight` | Preparar contexto y declarar los Records gobernantes antes de editar. | `target`, `intent` |
| `explain` | Explicar una posible valla de Chesterton. | `target` |
| `capture` | Cerrar un cambio con candidatos durables. | `statement` opcional |
| `conflicts` | Presentar conflictos con Records fijados y entregar la decisión a una persona. | ninguno |
| `health` | Diagnosticar MCP, proveedor, Git y la salud del canon. | ninguno |
| `protocol` | Cargar el protocolo maestro. | ninguno |

Un prompt retirado, como `review`, responde con la acción que lo reemplazó. Un
prompt desconocido es un error JSON-RPC y nunca termina la sesión.

## Transporte

JSON-RPC delimitado por líneas sobre stdio, protocolo `2024-11-05`. Una sesión
persistente con el proveedor por proceso del servidor. Los diagnósticos van a
stderr; stdout solo lleva mensajes MCP.
