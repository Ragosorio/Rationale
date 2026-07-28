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

Claude Code también recibe seis skills de proyecto:
`/rationale-preflight`, `/rationale-explain`, `/rationale-capture`,
`/rationale-health`, `/rationale-protocol` y `/rationale-review`, reservado
para una persona.

## Configuración manual

Para registrar Codex globalmente:

```bash
codex mcp add rationale -- "$HOME/.local/bin/rationale" serve
```

Codex lee el protocolo de Rationale desde `AGENTS.md`. Usa solicitudes escritas
en vez de asumir un slash command específico del cliente, por ejemplo:

> Prepara este cambio con Rationale para `<target>` con intención `<intent>`.

Inspecciona la configuración del agente antes de commitearla. Nunca incluyas
tokens, claves privadas ni paths sensibles.

## Lo que MCP no hace

MCP expone health, preparación, explicación y captura. La aprobación, disputa,
revocación, superseder y cambio de autoridad permanecen en la CLI interactiva.
