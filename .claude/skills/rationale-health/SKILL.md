---
description: "Checks the MCP connection, the structural provider, and the health of the canon."
argument-hint: ""
arguments: []
disable-model-invocation: true
allowed-tools: Bash(rationale doctor:*)
---

Diagnose Rationale's health.

Local `doctor` result injected by the skill:

!`rationale doctor`

If the line above still appears as a literal `!`command`` (for example, through an MCP prompt), run the equivalent check before replying.

Reply in the language the user writes in. Keep tool names, field values, and commands verbatim.

1. Call the MCP tool `health`.
2. Keep apart: availability of the MCP tools, Codebase Memory status and coverage, the Git revision, and the health of the canon.
3. Report exactly what works, what is degraded, and what was not checked. Do not turn a missing provider into a missing canon, and never invent coverage.
