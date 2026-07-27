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

## Frontera

MCP no expone aprobación, corrección, disputa, revocación, superseder ni cambio
de autoridad. Esas acciones requieren la revisión interactiva de la CLI. El
stdio usa JSON por línea; `Content-Length` solo aparece cuando el cliente habla
con Codebase Memory.
