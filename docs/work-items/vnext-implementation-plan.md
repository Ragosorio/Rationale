# Rationale vNext — implementation delta-plan

Status: in progress on branch `codex/rationale-vnext` (local only — no push,
PR, merge or release is part of this work).

## Authority and scope

The project owner (`user:ragosorio`, declared `architecture-owner` in
`.rationale/config.yaml`) supplied the vNext brief on 2026-09-12. It replaces
the approval queue as the normal workflow with autonomous capture plus human
pin/override. Historical v0.5 documents stay historical.

In scope: capture, canon lifecycle, retrieval, provider boundary, MCP
contracts, local activity, `rationale ui` (HTTP/SSE + Graph/Activity
frontend), agent workflow, proposal migration, docs.

Out of scope: landing page, CBM changes, other repositories, global agent
configuration on this machine, releases, tickets/Obsidian/ledger, native
indexing, multi-project.

## Baseline (clean `HEAD` f926233, detached worktree)

- `cargo fmt --check`: pass. `cargo clippy --locked --all-targets -D warnings`: pass.
- `cargo test --locked`: 278 passed (225 unit + 53 integration), 0 failed.
- Installed tools: `rationale v0.1.0-beta.3`, `codebase-memory-mcp 0.8.1`.
- CBM project `Users-roor.osorio-Desktop-Rationale`: 3,655 nodes / 8,044 edges, `ready`.
- CBM reference clone (read-only, sparse `graph-ui/` + `LICENSE`):
  `DeusData/codebase-memory-mcp@7d74337` (MIT, © 2025 DeusData).

A previous agent session left uncommitted work on this branch: an RFC3339
`now_iso8601` and a governing-only filter in `pipeline::prepare`. The
timestamp algorithm is kept (with more tests and formatting); the retrieval
filter is replaced by a fix inside the compiler, where the invariant belongs.

## Evidence that changes the brief's assumptions

1. **Padding bug confirmed live.** The installed preflight for
   `src/retrieval.rs::select_constraints` returns five constraints with
   `governs_target: false`. Cause: `select_constraints` keeps
   `max(max_critical_constraints, governing_count)` of *all* constraints.
   Filtering at the call site would leave `compile_packet` still padding.
2. **Legacy timestamps exist in the canon** (`approved_at: epoch:1785512885`).
   Reading must accept `epoch:<secs>` and RFC3339.
3. **Canon symbol ids are machine-specific.**
   `constraint.kind-inferred-from-record-id-prefix` stores
   `Users-roor.osorio-Desktop-Rationale.src.storage.infer_kind_from_id`: the
   CBM project prefix encodes a local path. vNext `NodeBinding` stores the
   qualified name with the provider's own project prefix removed (the prefix
   comes from `list_projects`, not from re-implementing CBM naming). Legacy
   ids still match through `binding_match`'s token-boundary suffix rule.
4. **CBM call resolution for Rust is incomplete.** `pipeline::prepare` has
   `in_degree: 0` although `main.rs` and `mcp/server.rs` call it. An absent
   edge is not proof of absence, so derived relationship state keeps an
   explicit `unknown` beside `observed / indirect / orphaned`.
5. **CBM 0.8.1 Cypher supports bounded paths** (`[:CALLS*1..2]` and fixed
   multi-hop patterns verified). Bounded `find_paths` is feasible through the
   public MCP contract.
6. **CBM `USAGE` edges are noisy** (edges to Markdown `Section` nodes and to
   `site/astro.config.mjs`). The adapter filters low-signal labels and
   inferred edge types (`SEMANTICALLY_RELATED`, `SIMILAR_TO`) from working
   subgraphs.
7. **Integration tests index temp repos in the user's real CBM** (~383 temp
   projects in `list_projects`). New core tests use a fake provider; the
   legacy tests are left as they are and reported.
8. **No pending proposals in this repo.** Migration is exercised with fixtures.
9. **MCP server and UI are different processes.** Activity reaches the UI
   through per-session NDJSON files; working subgraphs are persisted as local
   operation snapshots under `.rationale-local/operations/`.
10. **`ProviderHandle` is a concrete enum.** A `Custom` variant lets core
    tests and a fixture provider share the same pipeline.
11. **MCP `initialize.clientInfo` is ignored today.** Client identity:
    `--client` flag first, then the client's self-reported `clientInfo.name`
    (recorded with its source), else `unknown`.
12. **The legacy `resolve_target` fell back to the first pattern-search hit.**
    Live on this repo, `src/pipeline.rs::prepare` resolved to a site Markdown
    section (with the machine-specific project prefix). It now delegates to
    the exact in-file `resolve_node`, as
    `decision.provider-owned-project-identity` requires.
13. **Governing decisions were invisible in the packet.** `select_constraints`
    only serves `kind: constraint`; the preflight of `pipeline::prepare` showed
    its governing decision only inside `assessment`. The vNext packet adds
    `decisions` (decision, exception and risk Records that govern the target).
14. **ADR-0012 (proposed) forbids agent prompts and Record content in local
    telemetry, and the guard test it promised never existed.** The activity
    stream carries identifiers plus a single-line intent of at most 280
    characters; statements and rationale travel by reference. Documented as
    ADR-0017 (proposed) and enforced by a guard test.
15. **vNext writers under `.rationale-local/` skipped the Git exclusion** that
    only `init`/`install-agent` installed (ADR-0014 §Decision 3). Activity,
    operation snapshots and conflicts now ensure it first.
16. **Dogfood: Codebase Memory lost this repo's project while
    `.rationale-local/codebase-memory-project.json` still named it**, so every
    structural query degraded. The provider's `not found` error arrives with
    `isError: true` and a truncated JSON body (it drags the list of 467
    projects, 436 of them temp repos indexed by Rationale's own integration
    tests). The adapter now validates a remembered project once per session,
    forgets it only on an explicit `not found`, re-resolves through
    `list_projects` (indexing only when the provider confirms absence, with
    its own deadline) and never reindexes on an unreadable listing. The
    live recovery exposed one more defect: `index_repository` received the
    relative path `.` and returned a project named `root` that could never be
    queried. The provider now always receives an absolute path, relative
    provider root paths never identify a repo, and an indexed identity is
    confirmed before it is remembered. The
    integration tests run with `RATIONALE_PROVIDER=none` unless they pass a
    fixture.
17. **`build.rs` can keep embedding an empty asset table.** It watches
    `ui/dist`, else `ui/`, else only `build.rs` (a missing path would rerun
    it on every build). A target dir whose last build predates `ui/` — here,
    Phase 7 before the frontend existed — keeps serving the fallback page
    after `npm run build` until `build.rs` changes or is touched. Once `ui/`
    is versioned a fresh checkout watches it, so the trade-off stays; the
    symptom is `aviso: este binario no incluye la interfaz web` at startup.
18. **`scripts/check-docs.sh` failed on this branch since the plan existed.**
    Its release-version guard exempts history (`docs/adr/`,
    `docs/work-items/`) but not `docs/`, and the baseline above records the
    installed `v0.1.0-beta.3` while `docs/RELEASE_VERSION` says `beta.2`. The
    plan is a work record, so it moved to `docs/work-items/`; the version
    mismatch itself belongs to the release/docs pass.
19. **Retiring a skill would have broken uninstall.** `expected_managed_entry`
    only recognized current actions, so any manifest from a previous install
    that recorded `rationale-review` would make `uninstall-agent` reject the
    whole manifest as an unmanaged path. Retired actions stay recognized.
20. **This repo tracked an inert pre-beta.3 `.mcp.json` entry** (`rationale
    serve`, logical command). Verified before retiring it: Claude Code never
    approved it (`enabledMcpjsonServers: []`) and `claude mcp get rationale`
    reports `Scope: User config` inside the repo, so sessions already used the
    global registration. With the owner's approval, `install-agent` retired it
    per ADR-0016.

## Governing Records and how vNext honors them

- `decision.provider-owned-project-identity` (governs `pipeline.rs`,
  `providers/`): provider expansion keeps public-MCP-only access,
  root-scoped project handles, no reindex of an existing project, symbols
  resolved within the declared file, responses correlated by id.
- `constraint.no-provider-internal-access`: every structural query goes
  through CBM's public MCP tools (`search_graph`, `query_graph`,
  `get_code_snippet`, `get_architecture`, `detect_changes`, `index_status`).
- `constraint.f8-project-authority`: the approval queue disappears from the
  normal workflow by owner decision, but the invariant survives for the new
  authority acts. Pinning and adopting a new assertion over a pinned Record
  require an actor declared in `.rationale/config.yaml`. Agents never grant
  `pinned`.
- `constraint.f8-atomic-proposal-claim`: migration claims each legacy
  proposal atomically before canonicalizing or archiving it.
- `constraint.f8-roundtrip-fidelity`: new typed fields (`provenance.kind`,
  `authority`, `supersedes`, `relationship_bindings`) coexist with the
  flattened `extra` mapping; legacy `provenance.created_by` is preserved.

## Model decisions

- **Provenance** `agent_asserted | human_stated | migrated`, stored as
  `provenance.kind` inside the existing mapping. Absent → `migrated`
  (pre-vNext). No `structural` provenance: structure is evidence.
- **Authority** `normal | pinned`, top-level `authority`. Absent → `normal`.
  Legacy approvals remain as history; migration never pins anything.
- **Precedence** pinned > normal; explicit `supersedes` > coexistence. Git SHAs
  are never ordered.
- **Lifecycle** reuses `lifecycle.status` / `lifecycle.events` and
  `applicability_policy.superseded_by`.
- **Conflict** only when a candidate supersedes or revokes an active pinned
  Record. A lexical "opposed" signal against a pinned Record on the same
  binding is a warning, never a block.
- **Conflict continuation** is local state in `.rationale-local/conflicts/`.
  `resolve_conflict(conflict_id, keep_pinned | adopt_new)`: `adopt_new`
  requires declared authority, and the replacement inherits `pinned`.
- **Budget** is a ceiling. Governing knowledge is never truncated; direct
  pinned knowledge that exceeds the budget is reported as
  `budget_overflow: authoritative_context`.

## Phases and expected files

1. **Foundation**: `evaluation.rs` (RFC3339 + legacy parse), `retrieval.rs`
   (no padding), tests.
2. **Autonomous canon**: `storage.rs` (provenance/authority/supersedes),
   new `canon.rs` (gate, dedupe, commit, supersede, conflicts, migration),
   `pipeline.rs::finalize`, `mcp/server.rs` (`finalize_change` candidates,
   `resolve_conflict`), `main.rs` (`pin`, `unpin`, `migrate`), `review.rs`
   (claim reuse), tests.
3. **Structural model**: `providers/mod.rs` (normalized types + trait),
   `providers/codebase_memory.rs` (adapter), `providers/fixture.rs`
   (deterministic provider, also `RATIONALE_PROVIDER=fixture:<path>`;
   `RATIONALE_PROVIDER=none` disables the provider), tests.
4. **Relationship rationale**: `storage.rs` (`relationship_bindings`), new
   `relationships.rs` (derived state), `binding_match.rs`, tests.
5. **Context compiler**: `pipeline.rs::prepare`, `retrieval.rs` packet vNext,
   new `operations.rs` (operation ids + snapshots), tests.
6. **Activity**: `evaluation.rs` → new `activity.rs` (`ActivityEvent`,
   per-session NDJSON, reader/merger), instrumentation in pipeline/MCP/CLI.
7. **UI backend**: new `ui/server.rs` (std-only localhost HTTP, Host
   validation, bounded requests, SSE), `main.rs` (`rationale ui`).
8. **Control Room frontend**: `ui/` (React + Three + d3-force-3d, Graph +
   Activity, adapted CBM renderer with MIT notices), `THIRD_PARTY.md`.
9. **Agent workflow**: `prompts.rs`, `docs/prompt-master*.md`, `agents.rs`
   (`serve --client <name>`, migration-tolerant registration matching), docs.
10. **Dogfood**: a real change in this repo through MCP → CBM → packet →
    finalize → UI; evidence recorded in `docs/work-items/`.

## Risks

- Canon writes from concurrent agent processes: atomic write + re-read under
  an exclusive lock file per commit; tests for races.
- A heuristic noise classifier discarding real knowledge. Discards are always
  reported with a reason, never silent.
- UI assets must be embedded in the binary. The release workflow does not
  build Node assets yet; if they are absent, the binary serves an explicit
  fallback page. Flagged for follow-up; no release changes in this work.
- Localhost HTTP: GET-only, `Host` allow-list against DNS rebinding, request
  size/time limits, embedded assets only (no filesystem paths).
- Existing tests that assert the approval workflow are updated only where the
  owner's vNext contract intentionally replaces them.

## Progress

- [x] Inspection, baseline, preflight
- [x] Phase 1 — foundation fixes (`634b234`)
- [x] Phase 2 — autonomous canon, provenance, pinned authority, conflicts (`aa6cb08`)
- [x] Phase 3 + 4 — normalized structural provider and relationship rationale
  (one commit: relationship bindings extend the same Record/candidate/matcher
  code the provider model feeds)
- [x] Phase 5 — context compiler vNext: `operation_id` + local operation
  snapshots, bounded structural neighborhood (keys derived by the core),
  relationship why with derived state, governing decisions, token budget as a
  ceiling measured on the serialized packet, explicit `budget_overflow`
- [x] Phase 6 — activity: per-session NDJSON `ActivityEvent` stream
  (ADR-0017), minimization guard test, instrumentation of pipeline, MCP and
  CLI, Git exclusion before vNext writes, `RunLog` retired
- [x] Phase 7 — `rationale ui` backend: std-only localhost HTTP (Host
  allow-list, GET/HEAD, bounded heads, CSP), read-only REST views over canon,
  operations and activity, SSE activity stream, assets embedded from
  `ui/dist` with an explicit fallback page
- [x] Phase 8 — Control Room frontend in `ui/` (Vite build embedded from
  `ui/dist`): Graph (instanced 3D working subgraph, roles, structural state
  and causal overlay, node/edge/operation panels), Activity (sessions,
  operations folded from events, timeline), Memory (canon browser, pending
  conflicts) and System views; REST snapshots plus SSE with debounced
  refresh; CBM render techniques adapted under MIT with per-file headers and
  `THIRD_PARTY.md`. Verified live on this repo: a CLI `prepare` reached the
  open UI as 8 SSE events and refreshed the graph and operation panel
  without reload; Host allow-list (421) and read-only method guard (405)
  confirmed. Fixed in verification: ticker items shrank below their content
  and overlapped; Spanish singular counts.
- [x] Phase 9 — agent workflow: master prompt and pre-made actions rewritten
  for the vNext contract (`operation_id`, durable candidates committed in the
  same call, conflicts over pinned Records decided only by a human through
  `resolve_conflict` or `rationale resolve`); the approval-queue `review`
  action retired for a user-only `conflicts` action; global registration as
  `serve --client <claude-code|codex|cursor>` with convergent migration of
  pre-vNext `serve` entries, uninstall recognizing both forms of this binary
  only; retired skills removed only when Rationale still owns them. This
  repo's instructions and skills regenerated with an isolated HOME and PATH
  (no global agent configuration touched). Verified over real MCP stdio.
- [x] Phase 10 — dogfood (`docs/work-items/vnext-dogfood-results.md`): one
  persistent `serve --client claude-code` session with the real Codebase
  Memory and the Control Room open. prepare → governance stated → change
  (retired prompts name their replacement) → tests → finalize with the same
  `operation_id` (3 Records committed, 0 discarded) → a contradicting
  prepare surfaced the new constraint as governing → the UI showed session,
  operations, capture, memory and node overlay live, and the session end.
  Findings: name-based Rust call resolution in the provider, noisy lexical
  polarity (a tested fence, left as is), index-freshness wording.

## Remaining before release (out of this work's scope)

- Documentation pass: `README.md`, `docs/user-guide/` (configuration, daily
  workflow, CLI reference), `site/src/content/docs/` and the landing page
  still describe the approval queue; `docs/RELEASE_VERSION` says beta.2
  while beta.3 is installed.
- Release workflow: build `ui/dist` before `cargo build --release` so the
  published binary embeds the Control Room instead of the fallback page.
