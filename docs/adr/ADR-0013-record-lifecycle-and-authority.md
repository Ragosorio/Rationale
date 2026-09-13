# ADR-0013: Auditable Record lifecycle with project-declared authority

**Status:** proposed
**Date:** 2026-07-26
**Deciders:** the project's human owner + cross-review

## Context

F8 capture produces pending proposals, and the initial approval is already
human. The alpha also needs to operate on approved Records: correct, dispute,
revoke, and supersede them, change their authority, and add evidence, without
erasing history or raising an actor's role by accident.

## Decision proposal

- Every mutation is recorded as an event under `Record.lifecycle.events`.
- `revoke` prevails over historical approvals.
- `supersede` sets `applicability_policy.superseded_by` and marks the lifecycle
  as `superseded`.
- Changing authority adds an auditable approval, but only for an actor and role
  present in `.rationale/config.yaml`.
- An undeclared actor cannot run lifecycle mutations or elevate itself.
- `review_record` is an interactive CLI; MCP stays read-only/prepare.
- Writing compares the content it read before overwriting, and aborts if it
  drifted.

Note added after 1.0: authority became `normal` or `pinned`. `rationale pin` and
`rationale unpin` change it through the same declared-authority rules, and MCP
captures agent-asserted Records through a gate but still never pins, unpins, or
mutates a Record's lifecycle. See `docs/work-items/vnext-implementation-plan.md`.

## Evidence

- `src/review.rs::mutate_record`
- `src/storage.rs::is_revoked` and `superseded_by`
- `src/assessment.rs`
- tests for dispute, revocation, supersession, evidence, and self-elevation
