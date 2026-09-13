# ADR-0002: Codebase Memory transport

**Status:** proposed — pending independent cross-review before `accepted`.
**Date:** 2026-07-25
**Deciders:** Claude Code (analysis and implementation); pending human approval and/or cross-review by another agent
**Supersedes / Superseded by:** none

## Context

`Rationale_Arquitectura_Conceptual_v0.1.md §7.1` requires an explicit answer to whether "client-to-server MCP is better than a CLI subprocess for the first vertical", with measured evidence, not preference. `docs/research/codebase-memory/04-cli-contracts.md` and `11-performance-observations.md` already produced formal measurements of both transports against the same Codebase Memory binary.

## Decision

Rationale's `CodeIntelligenceProvider` adapter (`Rationale_v0.5.md §21`) will use a **long-lived persistent MCP session** (a Codebase Memory child process started once per life of the Rationale process, not one CLI subprocess per operation) as its primary transport toward Codebase Memory.

## Evidence

Formal measurements (`04-cli-contracts.md`, `11-performance-observations.md`, our own stdio MCP client built in B1.1):

| Transport | Scenario | Measured latency |
|---|---|---:|
| CLI | No daemon (new process per invocation) | 6.811 s – 6.873 s |
| CLI | With a prior `daemon start` | 2.275 s – 2.283 s |
| MCP | `initialize` (handshake, once per process) | 6.791 s – 6.859 s |
| MCP | Subsequent `tools/call`, same session | 15 ms – 30 ms |

**Central finding:** the ~6.8 s cost is identical between a cold CLI and MCP's `initialize` handshake — it is the same startup cost of the Codebase Memory binary (loading ~180 tree-sitter grammars, checking existing SQLite indexes), not a transport difference. The real difference appears **after** startup: every subsequent MCP call in the same session costs 15–30 ms, while every CLI invocation repeats a cost of at least 2.2 s (with a warmed-up daemon) because there is no concept of a "session" between `cli <tool>` invocations.

## Alternatives considered

- **A CLI subprocess per operation**: discarded. Even with CBM's daemon warmed up (`daemon start`), each `cli <tool>` invocation costs ~2.2 s — two orders of magnitude above the baseline budget in `Rationale_v0.5.md §20.5.2` (P95 ≤ 150 ms) and above the intent-aware budget itself (~2 s, `Arquitectura_Conceptual_v0.1.md §13.2`, already at the limit with a single call).
- **A new MCP session per operation**: discarded for the same reason — it would pay the 6.8 s handshake on every operation, with no advantage over the cold CLI.
- **Connecting to Codebase Memory's persistent daemon** (`06-daemon-and-watcher.md`): **not discarded but deferred**. If Rationale could connect a new MCP session to an already-running CBM daemon (instead of starting its own child process), the 6.8 s cost could be paid once per machine instead of once per Rationale process. This was not tested in this epic (an explicit research item, not blocking for Phase D).

## Consequences

- Rationale's adapter must manage the lifecycle of a long-lived child process (spawn once, keep alive, terminate cleanly when Rationale closes) — more process-management complexity than a stateless CLI subprocess, but necessary to meet the latency budget.
- The first `prepare_change`/`explain_target` of a Rationale session will inevitably pay Codebase Memory's ~6.8 s startup cost — it must be communicated honestly to the user/agent as "warm-up", not hidden or presented as part of the baseline budget.
- The **baseline fast path** (`Rationale_v0.5.md §20.5.1`) still cannot depend on this MCP session for its first cold invocation — it must depend exclusively on bindings already resolved locally by Rationale, as `12-integration-recommendation.md` already concluded. This transport decision resolves the intent-aware mode, not the baseline.

### Post-adversarial-review correction (`docs/work-items/adversarial-review-adr-0001-0002-0006.md`)

**The Phase D implementation (`src/main.rs`) does NOT yet achieve the amortization this ADR describes.** `cmd_health`/`cmd_prepare` call `CodebaseMemoryClient::spawn()` at the start of each invocation and the `rationale` binary exits at the end of `main()` — each CLI run is a new operating-system process that pays the full ~6.8 s handshake. The measured, real 15–30 ms per-call advantage only materializes **within** a single invocation (between the several MCP calls one `prepare` run makes internally), never **between** successive CLI invocations from a terminal.

This does not invalidate the transport decision (MCP over a CLI subprocess is still superior in every scenario), but it does mean that **the promised amortization depends on an architectural decision not yet made**: whether Rationale itself is a single-use process (CLI) or a long-lived process (daemon/server). That question was already marked open in `Arquitectura_Conceptual_v0.1.md §28` ("One process per session or a shared daemon?").

**Phase E5 (the MCP surface) is precisely the resolution of this gap**: an MCP server is, by construction, a long-lived process that serves multiple `tools/call` without exiting between them — the session toward Codebase Memory opens once per life of Rationale's *server*, not per CLI invocation. The CLI (`rationale prepare` from a terminal) will keep paying the full cost every time until Rationale has its own daemon (out of scope for this ADR — it belongs to ADR-0009, Baseline integration surfaces).

## Risks

- A long-lived child process can be orphaned or become a zombie if Rationale exits abnormally — mitigation: explicit signal handling and a health check (`health`) at the start of each Rationale session. **Note from the adversarial review: this mitigation is described but not yet implemented** (there is no `ctrlc`/`signal-hook` in the code); the only current mechanism is a `Drop` that does not run on an unhandled signal. Low practical risk today because each CLI invocation is already short-lived, but it must be implemented before Phase E5 introduces a real long-lived server.
- If Codebase Memory updates its binary while the MCP session is alive, the adapter could keep talking to an obsolete version — mitigation: capability negotiation (`capabilities()`) on reconnect; do not assume a long session is always valid.
- **No response `id` correlation:** the current client assumes strictly sequential calls (documented in a comment, `src/providers/codebase_memory.rs`) and attributes any incoming message to the last request sent, without checking the `id`. If Phase E5 needs to serve concurrent calls from multiple agents, this assumption stops holding and must be fixed first.
- **SQLite contention between concurrent Rationale processes:** not evaluated. If Phase E5 keeps the current pattern of one CBM child per Rationale process, two concurrent sessions on the same repository would open two CBM children competing for the same project SQLite cache.

## Validation

Reproducible measurement with the stdio MCP client in `docs/research/codebase-memory/11-performance-observations.md §Reproduce`.

**This ADR is `proposed`.** Pending: measure whether connecting to CBM's persistent daemon avoids paying the 6.8 s handshake per process (a research item from `12-integration-recommendation.md`) — if confirmed, it would update this ADR with an even faster path without changing the central decision (MCP over CLI).

## Revisit trigger

Reopen if: (a) connecting to CBM's persistent daemon is confirmed to avoid the 6.8 s cost, changing the design of the adapter's process management; (b) a future Codebase Memory version drastically reduces the cost of `initialize`, which could make one MCP session per operation viable after all.
