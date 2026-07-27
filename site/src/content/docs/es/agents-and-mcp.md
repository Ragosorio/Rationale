---
lang: es
slug: agents-and-mcp
title: Agentes y MCP
description: Conecta Claude Code, Codex o Cursor sin entregar la aprobación al protocolo.
section: Proyecto
order: 11
---

## Configuración automática

```bash
rationale init --skip-agent-config
rationale install-agent --dry-run
rationale install-agent
```

El instalador detecta agentes compatibles, escribe un bloque idempotente de
instrucciones y registra el servidor MCP donde el agente soporta configuración
por proyecto. El bloque contiene el [prompt maestro](/es/docs/prompt-master).

## Configuración manual

Para registrar Codex globalmente:

```bash
codex mcp add rationale -- "$HOME/.local/bin/rationale" serve
```

Inspecciona la configuración del agente antes de commitearla. Nunca incluyas
tokens, claves privadas ni paths sensibles.

## Lo que MCP no hace

MCP expone health, preparación, explicación y captura. La aprobación, disputa,
revocación, superseder y cambio de autoridad permanecen en la CLI interactiva.
