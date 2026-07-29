---
description: "Comprueba conexión MCP, proveedor y salud del canon."
argument-hint: ""
arguments: []
disable-model-invocation: false
allowed-tools: Bash(rationale doctor --check:*)
---

Diagnostica la salud de Rationale.

Resultado local de `doctor` inyectado por el skill:

!`rationale doctor --check 2>/tmp/.rationale-doctor-check.$$; code=$?; if [ "$code" -eq 1 ] && [ ! -s /tmp/.rationale-doctor-check.$$ ]; then rm -f /tmp/.rationale-doctor-check.$$; exit 0; fi; cat /tmp/.rationale-doctor-check.$$ >&2; rm -f /tmp/.rationale-doctor-check.$$; exit "$code"`

Si la línea anterior todavía aparece como un literal `!`comando`` (por
ejemplo, mediante un prompt MCP), ejecuta el chequeo equivalente antes de
responder.

1. Llama la herramienta MCP `health`.
2. Distingue: disponibilidad de las herramientas MCP, estado/cobertura de Codebase Memory, revisión Git y salud del canon.
3. Reporta exactamente qué funciona, qué está degradado y qué no fue comprobado. No conviertas ausencia del proveedor en ausencia del canon ni inventes cobertura.
