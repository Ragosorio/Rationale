---
lang: es
slug: agents-and-mcp
title: Agentes y MCP
description: Conecta Claude Code, Codex o Cursor — un registro por usuario, un protocolo en el proyecto y una frontera humana que sigue siendo humana.
section: Operar
order: 6
---

## Configuración automática

El instalador registra el servidor MCP en los agentes que detecta. Dentro de un
proyecto, `install-agent` además escribe el protocolo:

```bash
rationale install-agent --dry-run
rationale install-agent
```

Qué escribe:

| Agente | Registro MCP (por usuario) | En el proyecto |
|---|---|---|
| Claude Code | `~/.claude.json` | bloque en `CLAUDE.md` + seis skills en `.claude/skills/` |
| Codex | `codex mcp add` | bloque en `AGENTS.md` |
| Cursor | `~/.cursor/mcp.json` | `.cursor/rules/rationale.mdc` |

Cada registro ejecuta `rationale serve --client <agente>` con la ruta absoluta
del binario instalado, así las aplicaciones gráficas funcionan sin heredar el
`PATH` de tu shell, y cada sesión se atribuye al agente correcto en el Control
Room. Los archivos del proyecto nunca contienen una ruta personal.

La instalación es convergente: repetirla no cambia nada, un registro anterior
del mismo binario se migra, y `rationale uninstall-agent` revierte exactamente
lo que se escribió — las skills que editaste se conservan.

## Skills de Claude Code

- `/rationale-preflight <target> <intent>` — localizar, preparar y declarar los
  Records gobernantes antes de editar.
- `/rationale-explain <target>` — explicar una posible valla de Chesterton.
- `/rationale-capture [statement]` — cerrar el cambio con candidatos durables.
- `/rationale-conflicts` — presentar los conflictos pendientes con Records
  fijados y entregarte la decisión. Los agentes no pueden invocarla por su
  cuenta.
- `/rationale-health` — health por MCP más `rationale doctor`.
- `/rationale-protocol` — cargar el [prompt maestro](/es/docs/prompt-master)
  completo.

Las mismas acciones existen como prompts MCP: `preflight`, `explain`,
`capture`, `conflicts`, `health`, `protocol`.

## Codex

Codex lee el protocolo en `AGENTS.md`. Pídelo por escrito en vez de suponer un
slash command:

> Prepara este cambio con Rationale para `<target>` con intención `<intent>`.

## Registro manual

Para un cliente que Rationale no configura, registra el servidor tú mismo:

```bash
codex mcp add rationale -- "$HOME/.local/bin/rationale" serve --client codex
```

Cualquier cliente MCP por stdio funciona con `rationale serve`. Sin `--client`,
la sesión usa el nombre que el cliente declara en `initialize`, registrado con
esa fuente, o `unknown`.

## La frontera

MCP puede preparar contexto, explicar targets, reportar salud, capturar
conocimiento durable y transmitir una decisión humana sobre un conflicto. No
puede fijar ni desfijar, y `resolve_conflict` se niega a actuar sin la
respuesta literal de la persona. Reemplazar una regla fijada exige un actor
declarado en `.rationale/config.yaml`.
