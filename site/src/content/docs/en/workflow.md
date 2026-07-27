---
lang: en
slug: workflow
title: The propose → review → approve cycle
description: A complete change lifecycle with explicit human authority and honest uncertainty.
section: Operate
order: 6
---

## 1. Locate

Use Codebase Memory to find the symbol, callers, and relevant files when it is
installed. Treat its result as structural coverage, not as a decision.

## 2. Prepare

Call `prepare_change(target, intent)` before changing non-trivial code. Read
the constraints, evidence, authority, linkage, provider coverage, and intent
conflicts. A governing Record requires the agent to state whether the planned
change aligns with or contradicts it; Rationale does not pretend a local
heuristic is semantic proof.

## 3. Change and finalize

Make the smallest change consistent with the context. If code looks strange,
call `explain_target` before simplifying it. After the edit, call
`finalize_change`; it captures committed, staged, unstaged, and untracked files
without treating a dirty tree as “nothing happened”.

## 4. Review

Run:

```bash
rationale review
```

Inspect the statement, evidence, bindings, provisional status, and effect. A
proposal can be approved, corrected, rejected, or skipped. The reviewer must
confirm the required word for critical assertions.

## 5. Approve and share

Approval moves the Record into the canonical `records/` area with a human
approval event. Commit `.rationale/` with the code so the next session sees the
same decision. Never describe a proposal as an approved Record.
