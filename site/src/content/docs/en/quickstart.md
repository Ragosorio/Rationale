---
lang: en
slug: quickstart
title: Five-minute quickstart
description: Install Rationale, initialize a project, connect the optional structural provider, and run the first health check.
section: Start
order: 1
---

## What you need

Rationale runs locally on macOS, Linux, and the published preview targets. You
need Git and a shell. Codebase Memory is recommended for structural lookup but
optional: without it, Rationale reports degraded coverage and keeps working.

## Install

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Ragosorio/Rationale/releases/download/v0.1.0-alpha.6/rationale-installer.sh | sh
rationale --version
```

To install the companion provider first:

```bash
curl -fsSL https://raw.githubusercontent.com/DeusData/codebase-memory-mcp/main/install.sh | bash
```

## Initialize a project

Run these commands from the repository you want to govern:

```bash
rationale init
rationale health
rationale install-agent --dry-run
rationale install-agent
```

`init` creates `.rationale/`, preserves the canonical YAML files in Git, and
offers agent integration. `install-agent` writes an idempotent instruction
block and registers the MCP server where the detected agent supports it.

## Make the first governed change

Ask the agent to locate the target with Codebase Memory, then call
`prepare_change(target, intent)` before a non-trivial edit. After the edit it
should call `finalize_change(...)`. That creates a pending proposal when the
change contains a high-value signal; it does not approve anything.

Review the proposal yourself:

```bash
rationale review
```

Approval, correction, dispute, revocation, supersession, and authority changes
remain interactive human actions.

## Verify the installation

```bash
rationale --help
rationale --version
rationale health
```

The CLI emits the documented JSON contract on stdout. Diagnostics belong on
stderr, and `rationale serve` keeps stdout reserved for MCP JSON-RPC messages.
