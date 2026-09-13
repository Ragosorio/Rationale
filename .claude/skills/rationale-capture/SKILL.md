---
description: "Closes a change and writes only durable knowledge to the canon through finalize_change."
argument-hint: "[statement]"
arguments: ["statement"]
disable-model-invocation: true
---

Close the current change with Rationale. Optional statement from the person:

`$statement`

Live Git context injected by the skill:

- Current HEAD: !`git rev-parse HEAD`
- Status: !`git status --short`
- Diff since HEAD: !`git diff --no-ext-diff HEAD`

If those lines still appear as literal `!`command`` text (for example, because this action arrived as an MCP prompt), collect the same data with the available Git tools before continuing.

Reply in the language the user writes in. Keep tool names, Record ids, field values, paths, and commands verbatim. Write candidate statements and rationales in the language the existing Records use.

1. Review the diff and the tests that ran. Separate observed facts from intent and inference.
2. Decide which knowledge will stay true after this change: why the code is the way it is and what must be preserved. What only describes this change (what was edited, the steps, temporary state) is not memory: it goes in `summary`, not in `candidates`.
3. **One decision per Record.** Split into several candidates when the parts could be replaced or revoked separately, answer different questions, or have different lifetimes. Do not break a single decision into fragments that mean nothing apart.
4. Call `finalize_change` with the preflight's `operation_id` (if there is one), a short `summary`, and the `candidates`. Each candidate has `kind`, `statement`, a `rationale` that gives the cause (not a repeat of the statement), `durability: "durable"`, and `bindings` to the code it governs (`src/x.rs` or `src/x.rs::symbol`). Name the Records it replaces in `supersedes`. Use the statement above only when it is not empty and expresses a real decision.
5. When nothing durable was learned, call `finalize_change` without candidates: no memory is written, and that is fine.
6. Rationale writes valid candidates as canonical Records in that same call, with agent provenance; there is no approval queue. Report what was written, what was discarded and why, and which Records were superseded.
7. If the response includes `conflicts`, a candidate tried to replace a pinned rule. Stop: show the person both statements and the question, and only after their explicit answer call `resolve_conflict(conflict_id, decision, human_answer)` with their literal words. Never decide for them.

When the `rationale` skill is installed, its `references/capture.md` and `references/records.md` cover candidates in depth, and `scripts/check_candidates.py` checks them against the gate's rules before you send them.
