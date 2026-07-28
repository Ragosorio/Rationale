---
lang: en
slug: agents-and-mcp
title: Agents and MCP
description: Connect Claude Code, Codex, or Cursor without handing approval to the protocol.
section: Project
order: 11
---

## Automatic setup

```bash
rationale init --skip-agent-config
rationale install-agent --dry-run
rationale install-agent
```

The installer detects supported agents, writes an idempotent instruction block,
and registers the MCP server where the agent supports a project configuration.
The block contains the [master prompt](/docs/prompt-master).

Claude Code also receives six project skills:
`/rationale-preflight`, `/rationale-explain`, `/rationale-capture`,
`/rationale-health`, `/rationale-protocol`, and the human-only
`/rationale-review`.

## Manual setup

For a global Codex registration:

```bash
codex mcp add rationale -- "$HOME/.local/bin/rationale" serve
```

Codex reads the Rationale protocol from `AGENTS.md`. Use plain-language requests
instead of assuming a client-specific slash command, for example:

> Prepare this change with Rationale for `<target>` with intent `<intent>`.

Inspect agent configuration before committing it. Never commit tokens, private
keys, or sensitive paths.

## What MCP does not do

MCP exposes health, preparation, explanation, and capture. Approval, dispute,
revocation, supersession, and authority changes stay in the interactive CLI.
