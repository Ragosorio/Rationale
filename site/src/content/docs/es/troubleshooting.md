---
lang: es
slug: troubleshooting
title: Diagnóstico
description: Diagnostica cobertura del proveedor, silencio de MCP, enlaces viejos y estado de revisión sin adivinar.
section: Verificar
order: 8
---

## `health` dice que el proveedor no está disponible

Comprueba que `codebase-memory-mcp` esté instalado y en `PATH`. Rationale sigue
funcionando, pero la resolución de símbolos y los bindings automáticos tienen
menos cobertura. Lee la advertencia del packet en vez de tratar unavailable
como complete.

## `serve` parece silencioso

Es esperado al iniciarlo manualmente: stdio espera tráfico JSON-RPC y conserva
stdout limpio. Envía mensajes JSON por línea. No añadas banners, logs ni
Chestie a stdout.

## Falta una constraint

Ejecuta `rationale health` e inspecciona severidad, aprobación, bindings y
linkage del Record. `medium` es visible; cero bindings se reporta como
unresolved. Usa `rationale doctor --check` para encontrar Records legados con
paths inexistentes, severidad inválida, Subjects colgantes o sin aprobación.

## El agente quiere simplificar código raro

Pídele que llame a `explain_target` primero. Una rama extraña puede ser una
valla de Chesterton cuyo motivo vive en un Record aprobado.

## Se capturó dos veces una propuesta

No borres una a mano. Revisa las propuestas pendientes, compara evidencia y
bindings y rechaza o corrige el duplicado mediante la revisión interactiva.
