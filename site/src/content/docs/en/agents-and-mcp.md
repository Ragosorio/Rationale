---
lang: en
slug: agents-and-mcp
title: Agents and MCP
description: Connect Claude Code, Codex, or Cursor — one user-scoped registration, a protocol in the project, and a human boundary that stays human.
section: Operate
order: 6
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
| Claude Code | `~/.claude.json` | `CLAUDE.md` block + six skills in `.claude/skills/` |
| Codex | `codex mcp add` | `AGENTS.md` block |
| Cursor | `~/.cursor/mcp.json` | `.cursor/rules/rationale.mdc` |

Each registration runs `rationale serve --client <agent>` with the absolute path
of the installed binary, so GUI applications work without inheriting your shell
`PATH`, and each session is attributed to the right agent in the Control Room.
Project files never contain a personal path.

Installation is convergent: running it again is a no-op, an older registration
of the same binary is migrated, and `rationale uninstall-agent` reverts exactly
what was written — skills you edited are kept.

## Claude Code skills

- `/rationale-preflight <target> <intent>` — locate, prepare, and state
  governing Records before editing.
- `/rationale-explain <target>` — explain a possible Chesterton fence.
- `/rationale-capture [statement]` — close the change with durable candidates.
- `/rationale-conflicts` — present pending conflicts with pinned Records and
  hand the decision to you. Agents cannot invoke it on their own.
- `/rationale-health` — MCP health plus `rationale doctor`.
- `/rationale-protocol` — load the full [master prompt](/docs/prompt-master).

The same actions are exposed as MCP prompts: `preflight`, `explain`, `capture`,
`conflicts`, `health`, `protocol`.

## Codex

Codex reads the protocol from `AGENTS.md`. Ask in plain language instead of
assuming a slash command:

> Prepare this change with Rationale for `<target>` with intent `<intent>`.

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
