---
lang: en
slug: workflow
title: The prepare → change → capture loop
description: A complete change with Rationale — context before the edit, durable memory after it, and a person only where authority is at stake.
section: Operate
order: 5
---

## 1. Locate

The agent finds the real target with Codebase Memory: the symbol, its callers,
and the files around it. Structure answers *where* and *how it connects*; it
never decides *why* the code must stay.

## 2. Prepare

```text
prepare_change(target: "src/billing/retry.rs::backoff", intent: "cap retries at three")
```

Rationale compiles a context packet within a token ceiling and returns an
`operation_id`:

- **constraints and decisions** that govern the target, with authority
  (`pinned` or `normal`) and provenance;
- **explained relationships** with their structural state
  (`observed`, `indirect`, `orphaned`, `unknown`) and the Record that says why;
- a bounded **structural neighborhood** (callers, callees, dependencies, tests)
  and the target's source;
- **intent conflicts**, risks, known unknowns, and provider coverage.

If something governs the target, the agent must say whether its intent respects
it, contradicts it, or remains undetermined. A lexical conflict is a signal, not
a proven contradiction. Governing knowledge is never truncated to fit the
budget; when it cannot fit, the packet says `budget_overflow`.

## 3. Change

The agent makes the smallest change consistent with that context and runs the
relevant tests. If the code looks strangely complex, `explain_target` comes
first: it may be a fence whose reason lives in the canon.

## 4. Capture

```text
finalize_change(operation_id, summary, candidates: [...])
```

Each candidate is knowledge that stays true after the change: `kind`,
`statement`, a `rationale` that gives the cause, `durability: durable`, and the
`bindings` it governs. Rationale's gate discards noise, transient notes,
restated rationales, duplicates, and candidates without a meaningful binding —
always with a reason — and writes the rest as canonical Records **in the same
call**. There is no approval queue, and no candidates means no memory.

## 5. Decide only what is yours

People keep the authority that matters:

- `rationale pin <record-id>` fixes a rule no agent may replace.
- If a candidate tries to supersede a pinned Record, it is **not written**.
  `finalize_change` returns a conflict with the question; the agent asks you.
- You answer with `rationale resolve <conflict-id> keep-pinned|adopt-new`, or
  the agent relays your literal answer through `resolve_conflict`.
  `adopt-new` requires a declared actor, and the replacement inherits `pinned`.

## 6. Watch and review

`rationale ui` shows each operation live: what the agent was given, what it
captured, what was discarded, and which explanations are at risk. Records are
plain YAML in Git, so the change and its reasons travel through the same pull
request.
