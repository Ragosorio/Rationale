---
description: "Presents conflicts with pinned Records and hands the decision to a person."
argument-hint: ""
arguments: []
disable-model-invocation: true
allowed-tools: Bash(rationale conflicts:*)
---

Prepare the person's decision on Rationale conflicts.

Pending conflicts injected by the skill:

!`rationale conflicts`

If the line above still appears as a literal `!`command`` (for example, through an MCP prompt), get the same list before replying.

Reply in the language the user writes in. Keep conflict ids, Record ids, decisions, and commands verbatim, and quote statements exactly.

1. A conflict appears when an agent tried to replace (`supersedes`) a `pinned` Record. That assertion was not written to the canon: it waits for this decision.
2. For each conflict, show the pinned rule, the new assertion, and the question, without leaning toward an answer.
3. The decision belongs to the person: `keep_pinned` keeps the pinned rule; `adopt_new` replaces it and requires authority declared in `.rationale/config.yaml`.
4. Tell them they can decide in a terminal with `rationale resolve <conflict-id> <keep-pinned|adopt-new>`. If they prefer to decide in this conversation, call `resolve_conflict` only after their explicit answer and pass their words verbatim in `human_answer`.
5. Do not choose an option for them, do not pin or unpin Records, and do not claim a resolution before the tool confirms it.
