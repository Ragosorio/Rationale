---
description: "Reads the Records that govern a target and states conflicts with the intended change before editing."
argument-hint: "[target] [intent]"
arguments: ["target","intent"]
disable-model-invocation: true
---

Run Rationale's preflight for `$target` with this intended change:

`$intent`

Reply in the language the user writes in. Keep tool names, Record ids, field values, paths, and commands verbatim.

1. Locate the target. If Codebase Memory is available, use it to confirm the symbol, its callers, and the files around it, and note its coverage and warnings. It tells you where code is, not why it must stay.
2. Call `prepare_change(target: "$target", intent: "$intent")` and keep the `operation_id` it returns: `finalize_change` uses it to close the same operation.
3. Before editing, summarize what governs the target: `critical_constraints` and `decisions` with their authority (`pinned` or `normal`) and provenance; the explained `relationships` with their structural state (`observed`, `indirect`, `orphaned`, `unknown`) and why they exist; `intent_conflicts`; risks; `known_unknowns`; provider coverage; and `budget_overflow` if it appears.
4. For each governing Record and intent conflict, state whether the intent respects it, contradicts it, or remains undetermined, and why. Do not proceed silently, and do not treat lexical overlap as a proven contradiction. A `pinned` Record is a rule the project fixed: when the intent contradicts one, stop and ask instead of working around it.
5. An `orphaned` or `unknown` relationship does not prove that the explanation is wrong or that the relationship is gone; report it as uncertainty.
6. When the decision is not yours to make, stop and ask the person the specific question.

When the `rationale` skill is installed, its `references/preflight.md` and `references/packet.md` cover each step and packet field in depth.
