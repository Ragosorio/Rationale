# ADR-0001: Core language and toolchain

**Status:** proposed — pending independent cross-review (`AGENTS.md §Roles and cross-review`) before `accepted`. No agent may self-approve this decision (`evaluation.no-self-certification`, `.rationale/subjects/`).
**Date:** 2026-07-25
**Deciders:** Claude Code (spike implementation and proposal); pending human approval and/or cross-review by another agent
**Supersedes / Superseded by:** none

## Context

`Rationale_Arquitectura_Conceptual_v0.1.md §8` forbids choosing the core language by preference — it requires a spike with an identical workload across real candidates, measured with evidence, before writing production code. `docs/research/language/spike-protocol.md` set the protocol and the weighted criteria **before** implementing anything, precisely to avoid the bias of calibrating the criteria after seeing the result.

The candidates evaluated were Rust and Go (C and TypeScript/Node.js were discarded by an explicit decision already documented in `spike-protocol.md §Candidates`, not by evaluation).

## Decision

**Rust** is Rationale's core language, with evidence from the spike run in `spikes/language/rust/` and `spikes/language/go/`.

## Evidence

Complete implementations of the 6 mandatory operations in both candidates, with an identical, verified workload (`docs/research/language/candidates.md`). Raw measurements in `docs/research/language/benchmark-results.json`.

### Weighted score (`Arquitectura_Conceptual_v0.1.md §8.2`)

| Criterion | Weight | Rust | Go | Empirical basis |
|---|---:|---:|---:|---|
| Memory safety and reliability | 20% | 9 | 6 | Rust was correct on the first attempt in all 6 operations and demos. Go had a **real, measured failure**: the idiomatic implementation of subprocess cancellation (`exec.CommandContext` + `cmd.Output()`) took **5016 ms instead of ~500 ms** because of the Unix orphaned-grandchild problem that keeps the stdout pipe open — it had to be rewritten with process groups (`Setpgid` + `kill(-pid)`) to meet the deadline. |
| Distribution as a binary | 15% | 9 | 7 | Rust release binary: 2,216,304 bytes. Go release binary (stripped): 7,072,994 bytes — 3.2× larger. Both are static Mach-O arm64 executables with no external runtime. |
| Performance and latency | 15% | 9 | 7 | Peak resident memory: Rust ~3.0 MB, Go ~12.7–12.9 MB (4.2× more). MCP latency (`initialize`/`tools call`): Rust 3.5 ms/15.6 ms, Go 8.4 ms/9.0 ms — both negligible at this scale, with no practical difference. |
| MCP and JSON-RPC | 10% | 8 | 8 | Both implemented the same `Content-Length` framing (verified against the same protocol Codebase Memory uses, `docs/research/codebase-memory/11-performance-observations.md`) with equivalent effort in each language's standard library. No real differentiator. |
| SQLite and filesystem | 10% | 7 | 8 | Rust used `rusqlite` with SQLite vendored in C (`bundled`); Go used `modernc.org/sqlite`, pure Go without cgo — a better native cross-compilation story for Go in this specific respect. |
| macOS/Linux/Windows compatibility | 10% | 7 | 5 | **A real gap was found, not only a theoretical one:** the file locking used in the Go spike (`syscall.Flock`) is POSIX-only and the code itself fails explicitly on Windows; Rust also used a POSIX-only path in the spike for simplicity, but it has an unexercised portable alternative in `std` (`std::fs::File::lock`, available since Rust 1.89). See `docs/research/language/compatibility-matrix.md`. |
| Maintainability with agents | 10% | 7 | 7 | Rust "fails earlier and louder" (compilation errors); Go "fails later and silently" (the cancellation bug was caught by neither the compiler nor a linter, only by empirical measurement). A qualitative tie, leaning toward Rust because it matches this project's general principle of preferring explicit failures over silent ones (`docs/research/codebase-memory/10-failure-modes.md`). |
| Compilation and development time | 5% | 5 | 9 | Clean release build: Rust 31.94 s, Go 9.59 s — Go is 3.3× faster. A real advantage for Go in the iteration cycle with agents. |
| Interoperability with C processes | 5% | 8 | 5 | Rust used direct FFI to `flock()` naturally; Go deliberately avoided cgo for SQLite, which reduces cross-compilation friction but also suggests less native comfort with C interop should it be needed. |

**Weighted total: Rust 8.05/10, Go 6.80/10.**

This score is an explicit synthesis of evidence, not a law — the weights and scales are subject to the same sensitivity and review principle that `Rationale_v0.5.md §30.1.3` requires for `context_utility_density`. They are documented here precisely so they can be audited and disputed with data, not accepted on the authority of whoever computed them.

## Alternatives considered

- **Go**: discarded not for inability (it completed the 6 operations and the additional tests) but for a lower weighted score, dominated by the highest-weight criterion (memory safety and reliability, 20%), where a real, reproducible failure was found. Go keeps real, documented advantages (3.3× faster compilation, native fuzzing without dependencies, pure-Go SQLite) that must be weighed in the "Revisit trigger" if the evidence changes.
- **C**: discarded without evaluation, by an explicit decision made before this spike (`spike-protocol.md §Candidates`) — to preserve the protocol/adapter boundary with Codebase Memory (written in C) instead of sharing a language or process.
- **TypeScript/Node.js**: discarded without evaluation, reserved for prototypes and evaluation tooling, not for the distributed core (`Arquitectura_Conceptual_v0.1.md §8.1`).

## Consequences

- It enables continuing to Phase C5 (toolchain: formatter, linter, testing guide) and Phase D (vertical slice) in Rust.
- The Codebase Memory adapter (`Rationale_v0.5.md §21`) will be implemented in Rust, with FFI/subprocess toward the CBM binary (written in C) — the C interoperability already shown in the spike (`flock` through FFI) is a direct precedent.
- Go's 3.3× faster compilation is lost — partially mitigable with incremental `cargo check` during active development, not measured in this spike.
- Fuzzing/property testing in Rust will require an external dependency (`proptest` or `cargo-fuzz`) when needed — it is not in the initial Phase C5 toolchain unless a concrete case justifies it.
- File locking in Phase D/E must explicitly use the portable `std` path (`std::fs::File::lock`), not the POSIX-only FFI path used in the spike for simplicity — to be verified on Windows before Phase J (packaging).

## Risks

- The Rust compiler and its crate ecosystem may be less familiar to some agents than Go — mitigation: `Proceso §9.3` already requires creating a style guide, testing guide, and security guide specific to the chosen language (Phase C5).
- The weighted score is a synthesis of a single small spike, not of a production project — a different finding in Phase D (vertical slice, larger scope) could qualify this decision; see Revisit trigger.

## Validation

The spike was run completely in both candidates, with the 6 mandatory operations, an MCP server, file locking, a subprocess with a real deadline, and a test suite (6 unit tests in each, plus native fuzzing in Go). Reproducible: see the commands in `spikes/language/rust/` and `spikes/language/go/`, and `docs/research/language/benchmark-results.json` for the raw measurements.

**This ADR is `proposed`, not `accepted`.** It requires cross-review by another agent (ideally Codex, per `Proceso §13`) that tries to falsify the weighted score and the conclusions before moving to `accepted`, and explicit human approval before committing to Phase D.

## Revisit trigger

Reopen this ADR if:
- The Codebase Memory adapter (Phase E) reveals a need for C interop so intensive that Rust's advantage on that criterion becomes dominant (it would strengthen the decision) or, conversely, a Rust limitation not anticipated here appears (it would weaken the decision).
- Phase D (vertical slice) discovers that Rust's compilation time (31.94 s in this small spike) scales badly and materially affects the iteration speed of the agents building Rationale.
- A memory-safety bug is found in the core's own Rust implementation that contradicts the central premise of this decision.
