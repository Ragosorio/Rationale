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

## Manual setup

For a global Codex registration:

```bash
codex mcp add rationale -- "$HOME/.local/bin/rationale" serve
```

Inspect agent configuration before committing it. Never commit tokens, private
keys, or sensitive paths.

## What MCP does not do

MCP exposes health, preparation, explanation, and capture. Approval, dispute,
revocation, supersession, and authority changes stay in the interactive CLI.
