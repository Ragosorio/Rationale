---
lang: es
slug: agents-and-mcp
title: Agentes y MCP
description: Conecta Claude Code, Codex o Cursor — un registro por usuario, un protocolo en el proyecto y una frontera humana que sigue siendo humana.
section: Operar
order: 7
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
| Claude Code | `~/.claude.json` | bloque en `CLAUDE.md` + el skill `rationale` y cinco atajos en `.claude/skills/` |
| Codex | `codex mcp add` | bloque en `AGENTS.md` + el skill `rationale` en `.agents/skills/` |
| Cursor | `~/.cursor/mcp.json` | `.cursor/rules/rationale.mdc` |

Cada registro ejecuta `rationale serve --client <agente>` con la ruta absoluta
del binario instalado, así las aplicaciones gráficas funcionan sin heredar el
`PATH` de tu shell, y cada sesión se atribuye al agente correcto en el Control
Room. Los archivos del proyecto nunca contienen una ruta personal.

La instalación es convergente: repetirla no cambia nada, un registro anterior
del mismo binario se migra, y `rationale uninstall-agent` revierte exactamente
lo que se escribió — los archivos de skills que editaste se conservan.

La tabla describe `install-agent` a partir de la release posterior a v1.0.0.
v1.0.0 escribe los mismos bloques de instrucciones y, para Claude Code, seis
skills: los cinco atajos de abajo más `/rationale-protocol`, que la próxima
release retira en favor de `/rationale`. Mientras tanto, agrega el skill con
`npx skills add Ragosorio/Rationale`.

## Claude Code

`/rationale` carga el [skill `rationale`](/es/docs/skill), opcionalmente con una
operación: `/rationale capture`. El agente también puede elegirlo por su cuenta
cuando la tarea coincide.

Cinco atajos siguen disponibles para ti. Los agentes no los invocan por su
cuenta:

- `/rationale-preflight <target> <intent>` — localizar, preparar y declarar los
  Records gobernantes antes de editar.
- `/rationale-explain <target>` — explicar una posible valla de Chesterton.
- `/rationale-capture [statement]` — cerrar el cambio con candidatos durables.
- `/rationale-conflicts` — presentar los conflictos pendientes con Records
  fijados y entregarte la decisión.
- `/rationale-health` — health por MCP más `rationale doctor`.

Las mismas acciones existen como prompts MCP: `preflight`, `explain`,
`capture`, `conflicts`, `health` y `protocol`, que carga el
[prompt maestro](/es/docs/prompt-master).

## Codex

Codex lee el protocolo en `AGENTS.md`. Con el skill en
`.agents/skills/rationale/`, `$rationale` lo invoca por nombre. Siempre puedes
pedirlo por escrito:

> Prepara este cambio con Rationale para `<target>` con intención `<intent>`.

## Idioma

El protocolo, el skill y los atajos están escritos en inglés y le piden al
agente responder en el idioma en que escribes, manteniendo literales los nombres
de herramientas, los ids de Records, los valores de campos, las rutas y los
comandos. Los Records nuevos siguen el idioma que ya usa el canon del proyecto.

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
