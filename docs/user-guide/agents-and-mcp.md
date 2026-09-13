# Agents and MCP

Rationale can run from the CLI alone, but its main flow is an agent that
prepares context and captures knowledge without gaining authority over pinned
rules.

## Automatic setup

The installer registers the MCP server for the agents it detects. Inside a
project:

```bash
rationale install-agent --dry-run
rationale install-agent
```

| Agent | MCP registration (per user) | In the project |
|---|---|---|
| Claude Code | `~/.claude.json` | Block in `CLAUDE.md`, the `rationale` skill in `.claude/skills/rationale/`, and five shortcuts in `.claude/skills/rationale-*/` |
| Codex | `codex mcp add` | Block in `AGENTS.md` and the `rationale` skill in `.agents/skills/rationale/` |
| Cursor | `~/.cursor/mcp.json` | `.cursor/rules/rationale.mdc` |

The `rationale` skill is installed by `install-agent` starting with the release
after v1.0.0. v1.0.0 installs six shortcut skills, including
`/rationale-protocol`, which later releases retire in favor of `/rationale`.

Each registration runs `rationale serve --client <agent>` with the absolute path
of the installed binary (ADR-0016): it works from graphical applications
without depending on their `PATH`, and each session is attributed to the right
agent in the Control Room. Installation is convergent — repeating it changes
nothing, and an older registration of the same binary is migrated — and project
files never contain a personal path. Restart the agent after installing.

The text installed in the instruction files is the
[master prompt](../prompt-master.md).

To revert exactly those changes:

```bash
rationale uninstall-agent
rationale uninstall-agent --global-only
```

Skill files you edited are kept, file by file.

## Language

Everything an agent reads — the protocol, the skill, the shortcuts, and the MCP
tool descriptions — is written in English and tells the agent to reply in the
language the user writes in. Tool names, Record ids, field values, paths, and
commands stay verbatim in every language. New Records follow the language the
existing canon already uses.

## The `rationale` skill

The skill is the complete playbook: a router in `SKILL.md` and one reference
per operation (preflight, explain, capture, conflicts, health, adopt, maintain).
Claude Code and Codex load it on their own when a task matches its description,
and you can invoke it explicitly:

- Claude Code: `/rationale`, or `/rationale capture` to name the operation.
- Codex: `$rationale`, or `$rationale explain src/auth.rs::resolve`.
- Other agents that read Agent Skills: `npx skills add Ragosorio/Rationale`.

See [the skill guide](skills.md) for its layout, validator, and evals.

## Claude Code shortcuts

People type these; agents do not invoke them on their own, because the
`rationale` skill is the single entry point the model selects:

- `/rationale-preflight <target> <intent>`
- `/rationale-explain <target>`
- `/rationale-capture [statement]` — injects live Git context.
- `/rationale-conflicts` — presents conflicts and hands the decision to you.
- `/rationale-health` — MCP `health` plus `rationale doctor`.

The `rationale-review` skill from earlier versions, and `rationale-protocol`,
are retired automatically on reinstall when nobody edited them.

## Codex

Codex reads the protocol from `AGENTS.md` and the skill from
`.agents/skills/rationale/`, whose `agents/openai.yaml` declares its display
name and its dependency on the `rationale` MCP server. Use `$rationale`, or ask
in plain language:

> Prepare this change with Rationale for `<target>` with intent `<intent>`.

## MCP tools

The server exposes five tools:

- `health`
- `prepare_change`
- `explain_target`
- `finalize_change`
- `resolve_conflict` — only with the person's literal answer (`human_answer`)

It also exposes six prompts: `preflight`, `explain`, `capture`, `conflicts`,
`health`, and `protocol`, from the same source as the Claude Code shortcuts.
`rationale serve` is a line-delimited JSON-RPC stdio server: it stays open and
prints no banner on stdout, so a silent manual run is waiting for traffic.

MCP cannot pin or unpin Records, and replacing a pinned rule requires an actor
declared in `.rationale/config.yaml`.

## Manual configuration

To register Codex by hand:

```bash
codex mcp add rationale -- "$HOME/.local/bin/rationale" serve --client codex
```

Without `--client`, the session uses the name the client declares in
`initialize`, or `unknown`. Review your agent's configuration before versioning
it: never include tokens, private keys, or sensitive paths.

## Structural provider

Codebase Memory is optional. When it is unavailable, `health` reports degraded
coverage and packets carry honest warnings. Rationale never reads the
provider's internal SQLite database.
