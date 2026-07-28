---
lang: es
slug: mcp-reference
title: Referencia de MCP
description: Comportamiento herramienta por herramienta en la frontera del agente, incluyendo lo que MCP nunca puede aprobar.
section: Operar
order: 5
---

## `health`

Devuelve identidad del proyecto, revisión Git, estado del proveedor y
cobertura. La ausencia de Codebase Memory es un resultado degradado explícito,
no una razón para inventar símbolos o bloquear el cambio.

## `prepare_change`

Entrada: `target` y la `intent` real del agente; `severity` es explícita al
capturar una afirmación nueva. La salida incluye restricciones, evidencia,
autoridad, linkage, cobertura del proveedor, conflictos de intención y si se
requiere un veredicto de gobernanza.

Los Records gobernantes no desaparecen porque su severidad sea `medium`, y un
match vacío es honesto: el servidor nunca cae al primer Record no relacionado.

## `explain_target`

Devuelve el mismo conjunto de Records gobernantes para un target sin intent.
Usa el mismo matcher que `prepare_change`, así que ambas herramientas coinciden
en un binding de archivo o un sufijo estructural.

## `finalize_change`

Captura archivos observados mecánicamente, resolución del proveedor, intent,
evidencia y un statement propuesto. Los archivos sin commit se marcan
provisionales y siguen siendo capturables; la propuesta espera revisión humana.

## Prompts

El servidor declara la capability MCP `prompts`. `prompts/list` devuelve seis
acciones pre-hechas desde la misma fuente que genera los skills de Claude Code:

| Prompt | Propósito | Argumentos |
| --- | --- | --- |
| `preflight` | Prepara restricciones y conflictos de intención antes de editar. | `target`, `intent` |
| `explain` | Explica una posible valla de Chesterton antes de simplificar. | `target` |
| `capture` | Guía `finalize_change` después de un cambio. | `statement` opcional |
| `review` | Lista propuestas pendientes y entrega la aprobación a la CLI humana. | ninguno |
| `health` | Diagnostica MCP, proveedor, Git y canon. | ninguno |
| `protocol` | Carga el protocolo maestro completo. | ninguno |

`prompts/get` sustituye argumentos por nombre y devuelve un mensaje de usuario.
Un prompt desconocido produce un error JSON-RPC sin terminar la sesión
persistente. El descubrimiento y la decoración del comando pertenecen a cada
cliente MCP; no dependas de un slash command sin verificar ese cliente. En
Codex, pídelo por escrito, por ejemplo: «Prepara este cambio con Rationale para
`<target>` con intención `<intent>`». Claude Code recibe por separado el skill
limpio `/rationale-preflight` dentro del proyecto.

## Frontera

MCP no expone aprobación, corrección, disputa, revocación, superseder ni cambio
de autoridad. Esas acciones requieren la revisión interactiva de la CLI. El
stdio usa JSON por línea; `Content-Length` solo aparece cuando el cliente habla
con Codebase Memory.
