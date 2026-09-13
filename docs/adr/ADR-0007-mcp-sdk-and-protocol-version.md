# ADR-0007: MCP SDK and protocol version

**Status:** proposed — pending independent cross-review before `accepted`.
**Date:** 2026-07-25
**Deciders:** Claude Code (analysis and implementation); pending human approval and/or cross-review by another agent
**Supersedes / Superseded by:** none

## Context

Phase E5 turns Rationale into an MCP server (`prepare_change`, `explain_target`, `health`) in addition to a client (already implemented in `src/providers/codebase_memory.rs` since Phase D). It remains to decide whether to use an external SDK or keep separate manual codecs for each boundary.

## Decision

1. **Keep separate manual codecs per boundary**: line-delimited JSON for Rationale's stdio MCP server, and `Content-Length` only for the client toward Codebase Memory, whose historical contract is preserved.
2. **Protocol `2024-11-05`** — the same version the client already declares and Codebase Memory already accepts in real production.
3. **`rmcp` (the official SDK) remains a documented candidate for a future migration**, not for Phase E5.

## Evidence

- **The manual implementation has already been tested twice against a real MCP server** (Codebase Memory): once in the language spike (`spikes/language/rust/src/main.rs`, with a Python test client) and again in the real production client (`src/providers/codebase_memory.rs`, Phase D, with `initialize`/`tools/call` working end to end and a measured latency of 15–30 ms in a warm session). Zero framing incidents in either test.
- **`rmcp` (`github.com/modelcontextprotocol/rust-sdk`) is the official SDK** of the organization that defines the protocol — it was verified to build cleanly in this environment (successful `cargo build`, 26.5 s).
- **But `rmcp` pulls in a substantial dependency tree**: building it brought ~15 new transitive crates, including `tokio` (a full async runtime, feature `full`), `futures`, `async-trait`, `schemars`, `chrono`, `darling`, and `tracing` — a change in nature, not only in size, compared with Rationale's current synchronous approach (today only `serde`/`serde_json`/`serde_yaml`, with no async runtime in any module).
- **`rmcp` is in beta** (`3.0.0-beta.2`) — its API surface may still change before a stable release.

## Alternatives considered

- **Adopting `rmcp` now**: discarded for Phase E5. It would require converting `main.rs`, `providers/codebase_memory.rs`, and the entire current synchronous flow to `async`/`await` on a Tokio runtime — a cross-cutting architectural change, not an isolated "which SDK for the server" decision, right when Phase E already introduces large changes in the canonical store and the derived layer. Stacking both risks in the same phase violates the principle of small, verifiable changes (`Proceso §6.4`).
- **Another third-party SDK** (`rust-mcp-sdk`, `mcp-attr`, `tower-mcp`): not evaluated in the same depth — none is backed by being the SDK of the organization that defines the protocol, and adopting any of them would carry the same async conversion cost without the advantage of being "official".
- **Staying without an MCP server** (CLI only): discarded — it is exactly the limit this plan (Phase E) aims to remove; without an MCP surface, no agent can consume Rationale.

## Consequences

- The Phase E5 MCP server implements the standard stdio transport: read a JSON line, dispatch by `method`, write a JSON line — all synchronous, with no async runtime. The `Content-Length` codec stays encapsulated in the client toward Codebase Memory and is not reused for Codex.
- **A critical operating rule inherited from `Arquitectura §11.1`**: stdout is reserved exclusively for the MCP protocol; every log goes to stderr or a file. It is verified by an explicit test in Phase E6.
- If Rationale ever needs to serve multiple concurrent sessions/agents without blocking, migrating to `rmcp` (or to its own async runtime) becomes more attractive — but that is a hypothetical capability, not a current need.

## Risks

- Keeping the framing by hand means Rationale is responsible for following any future evolution of the MCP protocol manually, without the compatibility guarantees a maintained official SDK would offer. Mitigation: the Phase E5 surface is small (3 tools, no streaming, no cancellation of in-flight requests) — the risk of protocol divergence is low for this scope.
- Delaying `rmcp` means that, if a migration is decided later, the async conversion cost is still pending — it does not disappear; it is postponed to a moment with fewer simultaneous changes.

## Validation

Framing verified twice against the real Codebase Memory (Phases C and D). The extension to server mode is validated in Phase E5/E6 with a test client (the same Python pattern used in `docs/research/codebase-memory/11-performance-observations.md`) that calls `prepare_change`/`explain_target`/`health` against the real Rationale binary.

**This ADR is `proposed`**, pending cross-review and human approval. Codex's cleanroom evidence reopened the original decision to share `Content-Length` between both boundaries.

## Revisit trigger

Reopen when: (a) `rmcp` reaches a stable (not beta) release AND there is a real need for concurrency/async not covered by the synchronous approach; or (b) the MCP protocol introduces a capability (streaming, cancellation) that manual framing cannot reasonably support.
