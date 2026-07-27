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

## Boundary

MCP does not expose approval, correction, dispute, revocation, supersession, or
authority changes. Those actions require the CLI’s interactive review path.
The stdio protocol is JSON per line; `Content-Length` is used only when the
client talks to Codebase Memory.
