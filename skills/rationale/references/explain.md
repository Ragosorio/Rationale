# Explain

Find out why a target exists before simplifying, deleting, or "fixing" it. Code
that looks redundant, defensive, or oddly ordered is often a Chesterton fence:
its reason lives in the canon, not in the code.

## When to run it

- Before removing a branch, guard, retry, lock, cache, or duplicate-looking
  path.
- Before replacing an unusual implementation with the obvious one.
- When the user asks why code is the way it is, or whether it can go.

## Steps

1. Locate the target as in preflight: `path` or `path::symbol`.
2. Call `explain_target(target)`.
3. For each governing Record, report its statement, `kind`, `authority`,
   `provenance`, evidence, and whether its bindings still resolve.
4. Keep three things apart in the answer: what the Records state (retrieved),
   what you infer from them and the code (inference), and what remains unknown.
5. Decide:
   - **A Record explains the code.** Keep the behavior. If the user still wants
     the change, run preflight with the real intent so the verdict is explicit.
   - **Nothing governs it.** Say the canon has no recorded reason. That is the
     absence of a record, not proof the code is unnecessary: check tests,
     callers, and history before removing it, and capture the reason if you find
     one.
   - **The Record looks outdated.** Report why. Replacing it goes through
     capture with `supersedes`, or through a person when it is `pinned`.

## Answer shape

Lead with the reason in one sentence when a Record gives one. Quote Record
statements verbatim and cite their ids. Write the explanation in the user's
language.

```text
`charge()` returns early when `payment.reached_bank` is set because a charge
that reached the bank must never be retried (constraint.payments-idempotent,
pinned, human_stated). Removing the guard would allow double charges on retry.
Unknown: the `refunds → charge` relationship is orphaned, so its explanation
may be stale.
```
