---
lang: en
slug: mcp-reference
title: MCP reference
description: Tool-by-tool behavior at the agent boundary, including what MCP can never approve.
section: Operate
order: 5
---

## `health`

Returns project identity, Git revision, provider status, and coverage. A
missing Codebase Memory provider is an explicit degraded result, not a reason
to invent symbol resolution or block the change.

## `prepare_change`

Input: `target` and the agent’s actual `intent`; `severity` is explicit when a
new assertion is being captured. Output includes constraints, evidence,
authority, linkage, provider coverage, intent conflicts, and whether a
governance verdict is required.

Governing Records are not hidden because their severity is `medium`, and an
empty match is honest: the server never falls back to the first unrelated
Record.

## `explain_target`

Returns the same governing Record set for a target without an intent. It uses
the same binding matcher as `prepare_change`, so the two tools do not disagree
about a file-only binding or a structural suffix.

## `finalize_change`

Captures mechanically observed files, provider resolution, intent, evidence,
and a proposed statement. Uncommitted files are marked provisional and remain
capturable; a proposal is still pending until a human reviews it.

## Prompts

The server advertises the MCP `prompts` capability. `prompts/list` returns six
pre-made actions from the same source used to generate Claude Code skills:

| Prompt | Purpose | Arguments |
| --- | --- | --- |
| `preflight` | Prepare constraints and intent conflicts before editing. | `target`, `intent` |
| `explain` | Explain a possible Chesterton fence before simplifying it. | `target` |
| `capture` | Guide `finalize_change` after a change. | optional `statement` |
| `review` | List pending proposals and hand approval to the human CLI. | none |
| `health` | Diagnose MCP, provider, Git, and canon health. | none |
| `protocol` | Load the complete master protocol. | none |

`prompts/get` substitutes named arguments and returns one user message. An
unknown prompt is a JSON-RPC error and does not terminate the persistent
session. Prompt discovery and command decoration belong to each MCP client; do
not rely on a slash-command name without verifying that client. In Codex, ask
in plain language, for example: “Prepare this change with Rationale for
`<target>` with intent `<intent>`.” Claude Code separately receives the clean
`/rationale-preflight` project skill.

## Boundary

MCP does not expose approval, correction, dispute, revocation, supersession, or
authority changes. Those actions require the CLI’s interactive review path.
The stdio protocol is JSON per line; `Content-Length` is used only when the
client talks to Codebase Memory.
