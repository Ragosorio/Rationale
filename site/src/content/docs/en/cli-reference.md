---
lang: en
slug: cli-reference
title: CLI reference
description: The commands that initialize, inspect, prepare, review, and maintain a Rationale project.
section: Operate
order: 4
---

## Commands

| Command | Purpose |
| --- | --- |
| `init` | Create the `.rationale/` canon, or keep it intact when it exists, then converge agent integration. |
| `health` | Report project identity, Git revision, provider status, and coverage. |
| `prepare <target>` | Compile context for a path or symbol. |
| `review` | Human-review pending proposals. |
| `review-record <id>` | Mutate an approved Record through its audited lifecycle. |
| `install-agent` | Add idempotent instructions and MCP registration. |
| `uninstall-agent` | Revert only what `install-agent` wrote. |
| `update` | Install the latest preview selected by the local helper. |
| `serve` | Run the persistent MCP stdio server. |

## Common options

```bash
rationale health --project-root /path/to/project
rationale prepare "src/auth.rs::resolve" --project-root /path/to/project
rationale install-agent --project-root /path/to/project --dry-run
```

`prepare` takes the first positional argument that is not an option as its
target. `init --help` and invalid options are side-effect free. `init` does not
accept `--project-root`; run it from the project root instead. Running `init`
again still detects and configures agents, so a project initialized before
Claude Code was installed can recover without deleting `.rationale/`.
`--skip-agent-config` and `RATIONALE_SKIP_AGENT_CONFIG=1` keep that step
disabled on both the first and subsequent runs.

## Output boundary

Commands that return a machine contract keep JSON on stdout. Human-facing
diagnostics and Chestie stay on stderr or are suppressed for non-terminal
output. `serve` never prints a banner to stdout.
