# Phase G — formal dogfood on Rationale

Run date: 2026-07-26. This work item records reproducible evidence from the
Phase G dogfood and separates what the binary already demonstrated from what
still requires a human action or a new agent session.

## G1 — MCP connection

`.mcp.json` is versioned and points at the real server `cargo run --quiet
--release -- serve`. The Codex session that ran F8 was already open before this
server was added to the context, so Rationale's native MCP tools do not appear
in that session; loading `.mcp.json` requires restarting the agent session. As
non-substitutive evidence, the real binary `target/release/rationale serve` was
run with `Content-Length` framing and an ephemeral transport client, not a mock:

- `initialize` confirmed `2024-11-05`.
- `tools/list` returned exactly `prepare_change`, `explain_target`,
  `health`, and `finalize_change`.
- `health` returned `provider_status=successful`, `provider_coverage=complete`,
  and `working_tree_dirty=true` with the local cache available.

The ephemeral transport validates the server, but it is not presented as
evidence that the Codex session already loaded the native integration. That
part remains pending a session restart.

## G2 — `prepare_change` before changing

Before closing F8, `prepare_change` was run on `src/review.rs`, both through the
CLI and through the real MCP server, with the intent of verifying human review
and the atomic claim. The packet delivered:

- `consistency=working-tree-ahead`, consistent with uncommitted local changes;
- provider `successful`, but coverage `unknown` for the requested symbol;
- the provider-boundary Subject and Record, with authority `unreviewed`;
- an explicit warning that the symbol was not within the available coverage.

The degradation is honest: the context was useful to confirm the provider
boundary and its risks, but it did not pretend to have full structural coverage
of the target.

## G3 — capturing real decisions

Three real `finalize_change` calls were run against this repository, using the
commit before F8 as `base_revision`:

| Pending Record | Captured decision | Result |
|---|---|---|
| `constraint.f8-roundtrip-fidelity` | round-trip fidelity of the canonical Record | proposal written |
| `constraint.f8-atomic-proposal-claim` | atomic claim of a proposal | proposal written |
| `constraint.f8-project-authority` | authority declared by the project | proposal written |

The three proposals live in `.rationale/proposals/`, have `status: pending`, and
have no approvals. The second run of `rationale review` showed the three one per
screen, resolved the Git actor as
`user:ragosorio <ragosorio777@gmail.com>`, and showed `architecture-owner` from
`.rationale/config.yaml`. `skip` was entered for all three: no decision became
an approved Record.

This satisfies G3's mechanical capture without violating G4. Human approval of
these decisions — and especially of the nine foundational Subjects and the
twelve ADRs that remain `unreviewed`/`proposed` (with ADR-0011 partially
accepted but still open) — is deliberately left pending.

## G5 — honest measurement and limits

- The packet was actionable for the provider boundary and the revision state,
  but the coverage of `src/review.rs` was `unknown`; it is not counted as full
  coverage.
- The sandbox environment does not always allow opening the derived SQLite
  database; with authorized access to the local cache, `health` stayed
  `successful/complete`.
- CI for Linux and macOS is versioned in `.github/workflows/ci.yml`, but its
  remote execution still requires GitHub.
- No Records were self-approved. The `review_record` lifecycle is already
  implemented through the interactive CLI and covered by tests; embeddings,
  Jaccard calibration, Windows, and the monorepo pilot remain later gates.

## Graph verification

After the changes, Codebase Memory was re-indexed in `fast` mode. The state
became `indexed` with 2,440 nodes and 4,448 edges; later searches found
`src/review.rs::claim_proposal`, `src/review.rs::mutate_record`, and
`src/configuration.rs::ResolvedConfig.authority_for_actor`. The index coverage is
partial by design of the fast mode and does not replace reviewing the source code
directly; `docs/`, `scripts/`, and local artifacts are excluded.
