# Language spike protocol — Rust vs Go

**Status:** run in Phase C. This document keeps the evaluation criteria that were frozen before the spike; the results are in [`candidates.md`](candidates.md), [`compatibility-matrix.md`](compatibility-matrix.md), [`benchmark-results.json`](benchmark-results.json), and [`spike-notes.md`](spike-notes.md), precisely to avoid the bias of setting the criteria after seeing which language "felt better" (`Rationale_Arquitectura_Conceptual_v0.1.md §8`).

ADR-0001 keeps its `proposed` status until human approval; the later implementation does not constitute self-approval (`Rationale_Proceso_Construccion_Agentes_v0.1.md §9`).

## Candidates

**Rust vs Go**, by the team's decision. The conceptual architecture document (`Arquitectura_Conceptual_v0.1.md §8.1`) also lists C and TypeScript/Node.js as possible candidates; they are excluded from this spike by explicit decision, not by evaluation:

- **C**: Codebase Memory is already written in C. It is discarded as a candidate for Rationale's core precisely to preserve the protocol/adapter boundary instead of sharing a language or process (`Arquitectura_Conceptual_v0.1.md §28.1` of the conceptual document, and §8.1 of that document: "it must not be chosen only because Codebase Memory uses C").
- **TypeScript/Node.js**: reserved for prototypes, tooling, or evaluation harnesses, not for the distributed core (`Arquitectura_Conceptual_v0.1.md §8.1`).

If this spike's result is unsatisfactory for both candidates, it will be documented as such and the comparison reopened — a choice between two weak options is not forced.

## Identical workload (`Proceso §9.1`)

Each candidate must implement exactly the same minimal function, with no shortcuts or additional functionality in either:

```text
Input:
  target + intent + revision

Operations:
  1. Read a Record (YAML) from disk.
  2. Open a SQLite database (create it if it does not exist, insert and read a row).
  3. Call or mock an external structural provider (a subprocess or a simulated MCP call).
  4. Verify a revision (compare two revision strings, e.g. Git SHAs).
  5. Rank a constraint (sort a small list by a numeric field).
  6. Emit JSON to stdout.

Measurements:
  - Startup time (cold).
  - End-to-end latency of the full operation.
  - Peak resident memory.
  - Size of the compiled binary (release, without debug symbols).
  - Speed of the test suite.
  - Feasibility of cross-compilation / a CI strategy for macOS, Linux, and Windows.
```

**Workload equality rule:** one candidate may not implement a trivial demo while the other implements a more complete version (`Proceso §9.2`). Both must build exactly the six operations, no more and no less.

## Weighted criteria (`Arquitectura_Conceptual_v0.1.md §8.2`)

```text
20% Memory safety and reliability
15% Distribution as a binary
15% Performance and latency
10% MCP and JSON-RPC
10% SQLite and filesystem
10% macOS/Linux/Windows compatibility
10% Maintainability with agents
5%  Compilation and development time
5%  Interoperability with C processes
```

Beyond the minimal workload, each candidate must also test:

- A minimal MCP server.
- A client toward Codebase Memory or a CLI wrapper.
- File locking.
- Subprocesses.
- Cancellation.
- A deadline.
- An arm64 build.
- Binary size.
- Test tooling.
- Fuzzing or property tests (feasibility, not necessarily a complete implementation in the spike).
- Packaging (feasibility).

## Expected spike deliverables

```text
docs/research/language/
├── spike-protocol.md        (this document)
├── candidates.md            (notes per candidate after running the spike)
├── benchmark-results.json   (raw measurements)
├── compatibility-matrix.md  (macOS/Linux/Windows per candidate)
├── spike-notes.md           (qualitative observations: maintainability with agents, ergonomics)
└── ADR-0001-core-language.md → lives in docs/adr/, not here
```

When this protocol was written, none of these files existed yet; the document only set the protocol.

## What invalidates the spike

According to `Arquitectura_Conceptual_v0.1.md §22`, the resulting ADR cannot say only "we chose X because it is fast". It must record evidence, trade-offs, discarded alternatives and why, reversal risk, and a review date.

## Next step

Run the spike (Phase C, outside the scope of this bootstrap plan), implementing the identical workload in both candidates, measuring on the same hardware (`docs/environment/reference-development-machine.md`) and under the same conditions.
