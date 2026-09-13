# ADR-0009: Integration surfaces for the local MVP

**Status:** proposed
**Date:** 2026-07-26
**Deciders:** the project's human owner + cross-review

## Context

Rationale has two different boundaries: the agent needs to query and prepare
context, while canonical decisions require a human. A single interface mixing
both would let an MCP call mutate authority without visible confirmation.

## Decision

- MCP exposes `health`, `prepare_change`, `explain_target`, and
  `finalize_change`.
- MCP never approves, revokes, supersedes, or changes authority.
- The interactive CLI exposes `review` for proposals and `review-record` for the
  lifecycle of Records.
- Mutations write only under `.rationale/` and verify that the YAML did not
  change while the human was deciding.

Note added after 1.0: release 1.0 replaced the proposal queue with autonomous
capture. `finalize_change` now writes agent-asserted Records through a gate, and
MCP gained `resolve_conflict`, which applies only a human's literal answer on a
conflict with a pinned Record. Pinning, unpinning, and the Record lifecycle stay
in the interactive CLI. See `docs/work-items/vnext-implementation-plan.md`.

## Consequences

- Automation can prepare context without becoming authority.
- Agents need an MCP session and humans need the CLI binary.
- The lifecycle requires an interactive terminal during the alpha.
- A future non-interactive mode will need another ADR and explicit
  authorization; it is not inferred from this decision.

## Evidence

- `src/mcp/server.rs`
- `src/main.rs::cmd_review` and `cmd_review_record`
- `src/review.rs::mutate_record`
- `tests/mcp_server.rs` and the lifecycle tests in `src/review.rs`
