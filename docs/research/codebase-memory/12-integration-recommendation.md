# 12 — Integration recommendation (CBM-012)

A synthesis of `00` to `11`. This document introduces no new evidence — it consolidates the epic's findings into an adapter-boundary recommendation, following the `CodeIntelligenceProvider` contract of `Rationale_v0.5.md §21`.

## Summary of findings by severity

### High architectural impact

1. **Revision and coverage are not reliable from the provider without independent verification** (`05`). `detect_changes` returned empty results in the face of 200 really modified files. **Decision:** Rationale's Revision Coordinator must derive its own revision truth directly from Git, treating any revision/change signal from the provider as additional low-confidence data, never as an authoritative source. This is not conceptually new (it was already in `v0.5 §4.16`) — this epic turns it from a preventive principle into an empirically demonstrated necessity. *(Note: B1.3 did establish that the coverage fields `parse_partial`/`skipped`/`not_indexed` work correctly over MCP in versions after 0.8.1 — the `detect_changes` problem is independent and remains unexplained.)*
2. **Cross-package resolution cannot be assumed — not even when the provider supports it** (`08`, refined in B1.2). Zero `IMPORTS` relationships cross packages in a real monorepo of 8 npm packages, **even though `pass_pkgmap.c` is designed exactly to resolve that pattern** (`@repo/pkg`), was introduced 3 months before the indexing, and the Monorepo's real manifests/imports meet the conditions documented for it to work. **Decision:** the cross-workspace "relevance path" that `v0.5 §19.3, §32.0` promises cannot be built only on provider edges in v1 — manual/contractual bindings declared by the team must be the primary path for cross-package relationships, not a last-resort fallback.
3. **CLI and MCP startup latency incompatible with the baseline fast path** (`04`, `11`, measured formally in B1.1). CLI: 2.2 s–6.8 s per invocation. MCP: the `initialize` handshake costs ~6.8 s (the same as the cold CLI — it is the same startup cost, not a transport problem), but **once completed, each subsequent call in the same session costs 15–30 ms**, within the budget of `v0.5 §20.5.2`. **Decision:** the baseline fast path must never launch a new process/session per operation; it must depend on bindings already resolved locally. For the intent-aware mode, a **long-lived persistent MCP session** (a single `initialize` per life of Rationale's process) is viable and preferable to repeated CLI subprocesses — in favor of ADR-0002.

### Medium impact

4. **Three mutually inconsistent version identifiers** (`00`, `01`, `06`): `--version` (release versus `dev`), `git describe`, and the `daemon status` hash. **Decision:** explicit capability negotiation (`capabilities()`), never inferring compatibility from version parsing.
5. **CBM's "ADR" is a single architecture document, not a decision log** (`03`). **Decision:** it can be consumed as evidence (`stated`/`inferred`), never as the equivalent of an approved Record with provenance and authority.
6. **Structural clustering does not reveal real module/package boundaries** (`02`, `08` — the `packages` field repeats the project name in both cases, flat C and npm monorepo). **Decision:** do not use `get_architecture` as a source of workspace identity.

### Low impact / informative

7. CBM's full source compiles on the reference machine (`01`) — 2m49s, a 296 MB binary.
8. A non-blocking hook pattern already validated in production by CBM (`hook_augment.c`, `06`) — a valuable design reference for a future Rationale hook.
9. Derived storage outside the repository, with restrictive permissions, and no visible secrets (`07`) — a pattern to replicate.
10. Explicit errors with actionable suggestions in several cases (symbol not found, project not found) are the standard to match; silent errors (revision, cross-package) are the real risk (`10`).

## Recommended surface to consume

Of the 14 tools observed (`03`, also confirmed through `--help` in `04`), Rationale's initial adapter (Phase D/E) should consume, in this order of priority:

```text
Essential:
  list_projects        — project identity
  index_status         — a basic health signal (with the limitations of 05)
  get_code_snippet     — code evidence for Claims
  search_graph         — retrieval of binding candidates (deterministic before semantic, v0.5 §19.1)
  trace_path           — impact/relationship evidence (risk_labels as a signal, never as a Decision)

Useful, with nuances:
  get_architecture     — only as an exploratory signal, never as a source of workspace identity
  query_graph          — for one-off investigation cases (archaeology, v0.5 §17), not in the fast path
  manage_adr           — read-only, as a stated/inferred evidence source

Do not use yet / require more research:
  detect_changes       — not reliable according to 05; Rationale must implement its own detection through Git
  index_repository(persistence=true) — avoid the pattern of sharing a binary index in the repository (07)
  ingest_traces        — not evaluated, outside this epic's scope
```

## What the adapter must never do

Reaffirmed with concrete evidence from this epic, not only on principle (`Rationale_Arquitectura_Conceptual_v0.1.md §7.2`):

- Read the `.db` files in `~/.cache/codebase-memory-mcp/` directly (`07`) — they are outside the public contract and their format can change without notice.
- Parse or compare version strings to infer capabilities (`00`, `06` — three inconsistent identifiers prove it).
- Treat an empty result from any tool as a negative confirmation (`05`, `08`, `10`) — always `unknown`/`not found within the available coverage`.
- Forward the provider's raw error messages to the agent without normalizing them (`10`, case 5).

## Failure modes the adapter must absorb

See the full table in `10-failure-modes.md`. Summary of the required translation policy:

```text
Provider returns a silent empty result → adapter reports coverage: unknown, never "does not exist"
Provider returns a parser error        → adapter normalizes to status: degraded, without exposing the raw string
Provider cannot find a project/symbol  → adapter propagates the actionable hint; it is a good pattern to preserve
Provider takes longer than the deadline → fail open, degrade, never block (already in v0.5 §20.5.2)
```

## Research items resolved (B1, before Phase C)

The three items left open when this epic closed have been resolved with direct evidence:

1. **Formal MCP latency (B1.1) — resolved.** Our own stdio client against the HEAD binary: `initialize` costs ~6.8 s (once, the same as the cold CLI startup cost), but each subsequent `tools/call` in the same session costs 15–30 ms. See `11-performance-observations.md`. **In favor of ADR-0002: a persistent MCP session over repeated CLI subprocesses.**
2. **Reading `pass_pkgmap.c` (B1.2) — resolved, with severity revised upward.** The module does resolve exactly the `@org/pkg` pattern, has existed since 3 months before the Monorepo indexing, and the conditions for it to work (manifests, real imports) are present — and yet it produced no cross-package relationship. See `08-workspaces-and-monorepos.md`. **The gap is not "absent capability" but "present capability that fails silently" — reinforcing that manual bindings should be the primary path, not the fallback, for cross-package relationships in v1.**
3. **Coverage: version or transport? (B1.3) — resolved: it is the version.** The same HEAD build, invoked over real MCP (not the CLI), returned the same coverage fields (`parse_partial`/`skipped`/`not_indexed`) seen through the CLI. The MCP protocol is not the bottleneck; the 0.8.1 release simply did not implement them yet. See `05-revision-and-coverage.md`.

No research item remains pending before proceeding to Phase C (the language spike).

## Conclusion

Codebase Memory is a real structural provider, with a rich, well-paginated tool surface and solid engineering patterns on several fronts (non-blocking hooks, auditable installation, a cache with correct permissions). But **it cannot be treated as an oracle of revision, coverage, or workspace identity** — the epic's three high-impact findings are direct, reproducible evidence of exactly the risks that `Rationale_v0.5.md §4.9, §20.6` already anticipated conceptually. The right integration is the one the conceptual contract already required: consume it through a versioned public interface, with capability negotiation, and with Rationale as the layer that decides which of all this remains trustworthy for a specific revision.
