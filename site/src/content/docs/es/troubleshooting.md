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

## Un cliente GUI reporta el servidor MCP como no disponible

Cursor o una app de escritorio abierta desde el Dock reporta `rationale` como
no disponible, mientras Codex y Claude Code en terminal funcionan.

La configuración MCP declara a propósito el comando lógico `rationale` en vez
de una ruta absoluta personal, para que el archivo pueda versionarse y
compartirse. Ese comando solo resuelve si el cliente ve el directorio donde
está el binario. En macOS, una aplicación abierta desde el Dock hereda el
entorno de `launchd`, no el de tu shell, así que `~/.local/bin` —donde el
instalador coloca el binario— le resulta invisible.

`install-agent` avisa de esto e imprime el remedio. Puedes abrir el cliente
desde un terminal, o exponer el binario donde las apps GUI lo vean:

```bash
sudo ln -sf ~/.local/bin/rationale /usr/local/bin/rationale
```

No hace falta cambiar nada en la configuración del proyecto.
