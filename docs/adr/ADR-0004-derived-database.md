# ADR-0004: Derived database

**Status:** proposed — pending independent cross-review before `accepted`.
**Date:** 2026-07-25
**Deciders:** Claude Code (analysis and implementation); pending human approval and/or cross-review by another agent
**Supersedes / Superseded by:** none

## Context

`Rationale_v0.5.md §26.2` specifies the derived layer as "SQLite. Regenerable. Optimized. Not necessarily versioned. Invalidated by revision, schema, or provider generation." `Rationale_Arquitectura_Conceptual_v0.1.md §11.7` assigns this layer: indexing Records, FTS, aliases, scope paths, binding resolutions, candidate retrieval, deduplication, assessment cache, and invalidation by revision.

Today (Phase D complete) Rationale has no derived layer — it reads YAML directly on every invocation (`src/storage.rs`). That is correct for a vertical slice with a single constraint, but Phase E2/E3 introduces `Assessment` (which must be recomputed, not rewritten over the original Record) and a Context Compiler with a budget and ranking over potentially many Records — reading and parsing all the YAML on every query stops being sustainable.

## Decision

**SQLite (through `rusqlite`, feature `bundled`)** is the engine of the derived layer: computed assessments, an FTS5 index over statements and titles, and cached binding resolutions. It is added as a real core dependency in Phase E3 (today it exists only in the language spike).

## Evidence

- `rusqlite 0.31` (feature `bundled`) was already validated end to end in `spikes/language/rust/`: a successful build and 6 unit tests, including a real round trip of table creation + insert + select (`docs/research/language/candidates.md`), without requiring a system SQLite (vendored in C).
- Codebase Memory, the structural provider Rationale already consumes, uses the same pattern (a SQLite database per project in `~/.cache/codebase-memory-mcp/*.db`, with WAL mode active) — confirmed by direct file inspection in `docs/research/codebase-memory/07-storage-and-cache.md`. It is a real precedent, not only a preference, that SQLite suits this kind of derived index for a local developer tool.
- `v0.5 §26.2` already prescribes SQLite explicitly for this layer — this ADR introduces no new alternative; it formalizes with evidence a decision already pointed to in the conceptual contract.

## Alternatives considered

- **No database, always re-read YAML**: what Phase D does today (correct for its minimal scope). Discarded for Phase E because the Context Compiler needs ranking and filtering over a growing volume of Records/Assessments without re-parsing YAML on every query — it does not scale with the number of Records in a real project.
- **A different embedded engine (sled, redb)**: discarded without its own evaluation — `v0.5 §26.2` already sets SQLite, and there is no evidence Rationale needs the specific guarantees of a pure-Rust embedded KV store over SQLite, which also has native FTS5 (needed for `retrieval` according to `v0.5 §19.1`).
- **PostgreSQL/a managed database**: explicitly discarded by `Arquitectura §4.1` ("shall not mandatorily require... a managed database").

## Consequences

- `rusqlite = { version = "0.31", features = ["bundled"] }` is added to the root `Cargo.toml` in Phase E3.
- The binary grows (SQLite vendored in C is compiled into the Rust binary) — already measured indirectly in the spike, with no specific figure for the real core yet; measured in the Phase E verification.
- It introduces a dependency with a compiled C component — consistent with the advantage already evaluated in ADR-0001 ("interoperability with C processes", 5% of the weighted criteria, in Rust's favor).
- The derived layer is never the only copy of a decision (`Arquitectura §11.7`) — everything SQLite stores must be rebuildable from `.rationale/` (verified explicitly in Phase E3 with a "cache rebuild from scratch" test).

## Risks

- Corruption of the SQLite file (power loss, a process killed mid-write) — mitigation: WAL mode (the same pattern observed in Codebase Memory) and full regenerability as the safety net, not as an exception.
- Unbounded cache growth over time — pending an explicit invalidation/expiry policy in Phase E3 (it does not exist yet in Rationale, and `07-storage-and-cache.md` did not confirm that it exists in Codebase Memory).

## Validation

`rusqlite` is already validated by the spike (Phase C). The real integration into the core is validated in Phase E3 with a table creation + insert + select test on real data, a full regeneration test from `.rationale/`, and a measurement of the resulting binary size.

**This ADR is `proposed`**, pending cross-review and human approval.

## Revisit trigger

Reopen if the real Phase E3 measurement shows an unacceptable binary size or compilation time, or if a reproducible corruption case appears that WAL does not mitigate.
