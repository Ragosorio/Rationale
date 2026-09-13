---
lang: en
slug: agents-and-mcp
title: Agents and MCP
description: Connect Claude Code, Codex, or Cursor — one user-scoped registration, a protocol in the project, and a human boundary that stays human.
section: Operate
order: 7
---

## Automatic setup

The installer registers the MCP server for the agents it detects. Inside a
project, `install-agent` also writes the protocol:

```bash
rationale install-agent --dry-run
rationale install-agent
```

What it writes:

| Agent | MCP registration (per user) | In the project |
|---|---|---|
| Claude Code | `~/.claude.json` | `CLAUDE.md` block + the `rationale` skill and five shortcuts in `.claude/skills/` |
| Codex | `codex mcp add` | `AGENTS.md` block + the `rationale` skill in `.agents/skills/` |
| Cursor | `~/.cursor/mcp.json` | `.cursor/rules/rationale.mdc` |

Each registration runs `rationale serve --client <agent>` with the absolute path
of the installed binary, so GUI applications work without inheriting your shell
`PATH`, and each session is attributed to the right agent in the Control Room.
Project files never contain a personal path.

Installation is convergent: running it again is a no-op, an older registration
of the same binary is migrated, and `rationale uninstall-agent` reverts exactly
what was written — skill files you edited are kept.

Updating from v1.0.0 retires the `/rationale-protocol` skill in favor of
`/rationale`, unless you edited it. For an agent Rationale does not configure,
add the skill with `npx skills add Ragosorio/Rationale`.

## Claude Code

`/rationale` loads the [`rationale` skill](/docs/skill), optionally with an
operation: `/rationale capture`. The agent can also select it on its own when a
task matches.

Five shortcuts stay available for you. Agents do not invoke them on their own:

- `/rationale-preflight <target> <intent>` — locate, prepare, and state
  governing Records before editing.
- `/rationale-explain <target>` — explain a possible Chesterton fence.
- `/rationale-capture [statement]` — close the change with durable candidates.
- `/rationale-conflicts` — present pending conflicts with pinned Records and
  hand the decision to you.
- `/rationale-health` — MCP health plus `rationale doctor`.

The same actions are exposed as MCP prompts: `preflight`, `explain`,
`capture`, `conflicts`, `health`, and `protocol`, which loads the
[master prompt](/docs/prompt-master).

## Codex

Codex reads the protocol from `AGENTS.md`. With the skill in
`.agents/skills/rationale/`, `$rationale` invokes it by name. You can always
ask in plain language:

> Prepare this change with Rationale for `<target>` with intent `<intent>`.

## Language

The protocol, the skill, and the shortcuts are written in English and tell the
agent to reply in the language you write in, keeping tool names, Record ids,
field values, paths, and commands verbatim. New Records follow the language the
project's canon already uses.

## Manual registration

For a client Rationale does not configure, register the server yourself:

```bash
codex mcp add rationale -- "$HOME/.local/bin/rationale" serve --client codex
```

Any MCP client that speaks stdio works with `rationale serve`. Without
`--client`, the session uses the name the client reports during `initialize`,
recorded with that source, or `unknown`.

## The boundary

MCP can prepare context, explain targets, report health, capture durable
knowledge, and relay a human decision on a conflict. It cannot pin or unpin,
and `resolve_conflict` refuses to act without the human's literal answer.
Replacing a pinned rule requires an actor declared in `.rationale/config.yaml`.
