---
description: "Comprueba conexión MCP, proveedor y salud del canon."
argument-hint: ""
arguments: []
disable-model-invocation: false
allowed-tools: Bash(rationale doctor:*)
---

Diagnostica la salud de Rationale.

Resultado local de `doctor` inyectado por el skill:

!`rationale doctor`

Si la línea anterior todavía aparece como un literal `!`comando`` (por
ejemplo, mediante un prompt MCP), ejecuta el chequeo equivalente antes de
responder.

1. Llama la herramienta MCP `health`.
2. Distingue: disponibilidad de las herramientas MCP, estado/cobertura de Codebase Memory, revisión Git y salud del canon.
3. Reporta exactamente qué funciona, qué está degradado y qué no fue comprobado. No conviertas ausencia del proveedor en ausencia del canon ni inventes cobertura.
