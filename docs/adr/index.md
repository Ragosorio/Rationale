# ADR index

No ADR may be justified only with "we chose X because it is fast"
(`Rationale_Arquitectura_Conceptual_v0.1.md §22`). It must contain reproducible
evidence.

The mandatory initial ADRs (0001–0012, `Rationale_Arquitectura_Conceptual_v0.1.md §22`),
ADR-0013 added in Phase G, ADR-0014–0016 added in the pre-beta dogfood, and
ADR-0017 added in vNext are kept explicit in this index; proposals require
evidence before changing status:

| ADR | Title | Status | Depends on |
|---|---|---|---|
| [ADR-0001](ADR-0001-core-language.md) | Core language and toolchain | **proposed** — Rust | Phase C — Rust vs Go spike run, `docs/research/language/`. Adversarial review: holds, with nuances |
| [ADR-0002](ADR-0002-cbm-transport.md) | Codebase Memory transport (MCP vs CLI) | **proposed** — persistent MCP session | Phase B.1 — formal latency measurement. Adversarial review: holds, with significant nuances — see the correction incorporated into the ADR |
| [ADR-0003](ADR-0003-canonical-serialization.md) | Canonical serialization (YAML/JSON) | **proposed** — YAML, with `yaml_serde` replacing `serde_yaml` | Phase E1 — tested with a real round trip against a Subject from the repository |
| [ADR-0004](ADR-0004-derived-database.md) | Derived database | **proposed** — SQLite through `rusqlite` | Phase E1 — already validated in the language spike |
| [ADR-0005](ADR-0005-cache-root-and-project-identity.md) | Cache root and project identity | **proposed** — `~/.cache/rationale/projects/<id>/` | Phase E1 — precedent measured in Codebase Memory |
| [ADR-0006](ADR-0006-revision-fingerprint.md) | Revision fingerprint | **proposed** — derive from Git, never from the provider | Phase B — CBM-008, the critical `detect_changes` finding. Adversarial review: holds |
| [ADR-0007](ADR-0007-mcp-sdk-and-protocol-version.md) | MCP SDK and protocol version | **proposed** — manual framing, `rmcp` deferred | Phase E1 — `rmcp` compiled and evaluated; it requires an async runtime |
| [ADR-0008](ADR-0008-concurrency-and-locking.md) | Concurrency and locking | **proposed** — atomic rename is enough, locking deferred | Phase F1 — a real temporary-name bug found and fixed; a test with 8 threads writing concurrently verified 15/15 |
| [ADR-0009](ADR-0009-integration-surfaces.md) | Baseline integration surfaces | **proposed** | Phase G — MCP queries and prepares; the CLI mutates |
| [ADR-0010](ADR-0010-packaging-strategy.md) | Packaging strategy | **proposed** | Phase G/H — GitHub Release and installers |
| ADR-0011 | Licensing and dependency policy | accepted (partial) | MIT license decided (see the note below); dependency policy pending |
| [ADR-0012](ADR-0012-telemetry-and-privacy.md) | Telemetry and privacy | **proposed — validation failed in part** | Phase E1. The local-exclusion guarantee was invalidated in two real pilots (see §Validation update — 2026-07-28); replacement proposed in ADR-0014. The no-network-transmission decision still stands |
| [ADR-0013](ADR-0013-record-lifecycle-and-authority.md) | Record lifecycle and authority | **proposed** | Phase G — interactive `review_record` |
| [ADR-0014](ADR-0014-local-data-exclusion.md) | Local data exclusion in consumer projects | **proposed** | `alpha.7` → `main` migration on copies of Monorepo and BoostAPI. Proposes replacing ADR-0012's exclusion guarantee |
| [ADR-0015](ADR-0015-mcp-executable-resolution.md) | Executable resolution in per-project MCP config | **proposed** | The `PATH` premise was refuted empirically against Claude Code. Complements ADR-0014 |
| [ADR-0016](ADR-0016-user-scoped-agent-registration.md) | User-scoped MCP registration and convergent migration | **proposed** | ADR-0015's Cursor validation failed; proposes replacing it if it receives review and approval |
| [ADR-0017](ADR-0017-local-activity-stream.md) | Local activity stream and operation snapshots | **proposed** | vNext: the activity view needs the intent and target; narrows ADR-0012 §Decision 3 for these emitters (identifiers and an intent of ≤280 characters, never content) and retires `RunLog` |

**A proposal does not supersede another proposal.** While two ADRs are
`proposed`, the one that proposes replacing the other says so in its header but
gains no authority over it; the replaced ADR's `Superseded by` field is filled
only when the replacement passes cross-review and human approval. That is the
case today for ADR-0012 and ADR-0014.

Every ADR in `proposed` status requires cross-review by another agent and human
approval before moving to `accepted` (`AGENTS.md §Roles and cross-review`,
Subject `evaluation.no-self-certification`). ADR-0001, 0002, and 0006 already
went through an adversarial review by an independent session
(`docs/work-items/adversarial-review-adr-0001-0002-0006.md`) — none was approved
or rejected; the final decision is pending with the project's human owner.

The `rationale` Agent Skill and the move of agent-facing text to English did not
change an architectural boundary: they reuse the ownership, hashing, and
retirement rules of ADR-0008, ADR-0014, and ADR-0016, and are recorded in
`CHANGELOG.md` and in Rationale Records.

## Note on the license (ADR-0011, partial)

**MIT** was chosen for the repository (see `LICENSE`), prioritizing simplicity
and early adoption. MIT does not grant patents explicitly.

If Rationale moves toward an open protocol with third-party implementations
(`Rationale_v0.5.md §34` — the condition for calling it a protocol: at least two
independent consumers or providers, conformance tests, versioning), review
whether to add a patent grant or relicense under Apache-2.0. Doing it now is
trivial; doing it after having external contributors is much more costly (it
requires the consent of every copyright holder).

This ADR is considered **open** until a complete dependency policy exists (see
`docs/dependencies/inventory.yaml`).
