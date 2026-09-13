# Code map

A factual map of `src/` as it exists on `main` after 1.0. It does not
reinterpret `Rationale_Arquitectura_Conceptual_v0.1.md`; it describes the real
code with its real names.

## Modules and responsibilities

| Module | Responsibility | Does not |
|---|---|---|
| [`storage.rs`](../../src/storage.rs) | Canonical store: reads and writes `Record` (`.rationale/records/`; `.rationale/proposals/` only to migrate pre-1.0 proposals). Typed vNext fields: `provenance.kind`, `authority`, `supersedes`, `relationship_bindings`. Atomic writes (temp file + rename). Every nested struct (`BindingDeclaration`, `Approval`, `Evidence`, `Risk`, `RecordSubjectRef`) captures unmodeled fields in `extra: yaml_serde::Mapping` — round-trip fidelity is verified. | Decide authority or resolve Subjects — that is `subjects.rs`. |
| [`subjects.rs`](../../src/subjects.rs) | Reads `Subject` (`.rationale/subjects/`). `resolve_by_id_or_alias` (step 1 of v0.5 §9.1) and `resolve()` (steps 2–5: normalized name, binding overlap, scope, lexical similarity). | Decide authority; it never creates a Subject, only suggests an `action`. |
| [`assessment.rs`](../../src/assessment.rs) | Computes an `Assessment` (epistemic/authority/applicability/linkage) from a `Record`, the Git revision, and the provider state. Never persists anything itself. | Read or write files directly. |
| [`revision.rs`](../../src/revision.rs) | `GitSnapshot`/`Consistency` — the revision is ALWAYS derived from Git, never from the structural provider (ADR-0006). | Query any provider. |
| [`project.rs`](../../src/project.rs) | Resolves `path::symbol` to a `Target` inside the project, canonicalizing and rejecting path traversal. | Resolve symbols against a provider — only the path. |
| [`providers/`](../../src/providers/) | `CodeIntelligenceProvider` (trait) + `CodebaseMemoryClient` (the real implementation over persistent MCP, ADR-0002) + `ProviderHandle` (decouples spawning from use: the CLI spawns per invocation, the MCP server once). | Read the provider's internal storage (`constraint.no-provider-internal-access`). |
| [`cache.rs`](../../src/cache.rs) | Derived layer: SQLite (WAL), cached assessments invalidated by exact revision, FTS5 over statements. 100% regenerable from `.rationale/`. | Hold the only copy of a decision (Arquitectura §11.7). |
| [`retrieval.rs`](../../src/retrieval.rs) | Context compiler: `compile_packet` with priority levels (v0.5 §18.1), an explicit budget, and deterministic detection of conflicts with the intent. | Trim critical constraints to fit the budget; use embeddings. |
| [`capture.rs`](../../src/capture.rs) | Mechanical capture (v0.5 §15.1): normalized `diff --name-status`, final revision, working tree, provider coverage. Everything is `epistemic_status: observed`. | Infer a normative decision — only verifiable facts. |
| [`signals.rs`](../../src/signals.rs) | High-value signals (v0.5 §15.4) and capture levels 0–3 (v0.5 §16) — deterministic, no LLM. They inform `finalize_change` as observed facts and never write memory themselves. | Decide to block; assign authority. |
| [`pipeline.rs`](../../src/pipeline.rs) | The pure pipeline shared by the CLI and the MCP server: `prepare`, `explain`, `health`, `finalize`. No `println!`/`eprintln!` — it returns `diagnostics: Vec<String>` and lets each caller decide where to write them. | Print anything to stdout or stderr. |
| [`review.rs`](../../src/review.rs) | `rationale review` confirms pre-1.0 proposals (legacy), and `mutate_record` implements the human lifecycle of Records (correct, dispute, revoke, supersede, authority, evidence), always with auditable events and claim/TOCTOU protection. | Run inside the MCP server — it needs an interactive human. |
| [`prompts.rs`](../../src/prompts.rs) | Single source of the six pre-made actions and their argument substitution for MCP prompts and Claude Code shortcut skills. Agent-facing text is English and asks the agent to reply in the user's language. `protocol` is an MCP prompt only. | Execute tools or approve Records; it only describes actions. |
| [`skill_bundle.rs`](../../src/skill_bundle.rs) | Embeds the `rationale` Agent Skill from `skills/rationale/` (without `evals/`) and lists retired skill files. Its tests keep the bundle identical to the directory, enforce the Agent Skills limits and one-level references, and keep the candidate validator in sync with the capture gate. | Install anything — `agents.rs` does. |
| [`agents.rs`](../../src/agents.rs) | Detects agents, registers `serve --client <agent>` per user (migrating the earlier form), converges instruction blocks (recognizing the pre-1.1 Spanish marker), writes the Claude Code shortcuts and the `rationale` skill for Claude Code and Codex with atomic writes and per-file hash reversal, and retires skills of retired actions only when they keep their hash. | Delete a skill file the user edited, write through a symbolic link, or install skills for Cursor. |
| [`mcp/framing.rs`](../../src/mcp/framing.rs) | Separate JSON-RPC codecs: newline-delimited stdio for Rationale's server and `Content-Length` for the client toward Codebase Memory (ADR-0007). Explicit limits after the adversarial review. | Interpret message content — it only frames it. |
| [`mcp/server.rs`](../../src/mcp/server.rs) | MCP server: five tools (`health`, `prepare_change`, `explain_target`, `finalize_change`, `resolve_conflict`) and six prompts (`prompts/list`/`prompts/get`). One persistent `ProviderHandle` session for the whole life of the process. `catch_unwind` turns tool panics into `isError` without ending the session. | Write anything but protocol to stdout — see `Arquitectura §11.1`. |
| [`configuration.rs`](../../src/configuration.rs) | Finds `.rationale/` by walking up directories (the way Git finds `.git/`) and loads `config.yaml`. | — |
| [`evaluation.rs`](../../src/evaluation.rs) | RFC 3339 timestamps (`now_iso8601`, `now_rfc3339_millis`) and tolerant reading of legacy `epoch:` values. | Write logs: activity lives in `activity.rs`. |
| [`canon.rs`](../../src/canon.rs) | Autonomous canon (vNext): candidate gate, deduplication, commit, explicit supersession, conflicts with `pinned` Records, and migration of pre-vNext proposals. | Grant `pinned` on an agent's behalf; order Git SHAs. |
| [`context.rs`](../../src/context.rs) | The target's bounded structural neighborhood and explained relationships with their derived state; role-based selection with a ceiling and core-derived keys. | Dump the whole graph. |
| [`relationships.rs`](../../src/relationships.rs) | Derived state of a `RelationshipBinding`: observed / indirect (compatible path of ≤3 hops) / orphaned / unknown. | Persist the state or delete the explanation. |
| [`operations.rs`](../../src/operations.rs) | The `prepare_change` operation: `operation_id`, local snapshot of the considered and selected subgraph, closed by `finalize_change`. | Act as canon: it is derived, regenerable state. |
| [`doctor.rs`](../../src/doctor.rs) | Canon integrity: invalid severities and authorities, Records without bindings, broken `path_hint`s, dangling Subjects, unmigrated proposals. `--repair` asks for confirmation per finding. | Repair without human confirmation. |
| [`ui/`](../../src/ui/) | `rationale ui`: an HTTP/1.1 server using only `std` on `127.0.0.1` (`http.rs`: GET/HEAD, Host allowlist, limits, CSP), REST and SSE views over the canon, operations, and activity (`api.rs`, `mod.rs`), and assets embedded from `ui/dist` by `build.rs` (`assets.rs`). The frontend lives in `ui/` (React + Three). | Write state or serve file-system paths. |
| [`activity.rs`](../../src/activity.rs) | Local per-session activity (`.rationale-local/activity/<session>.ndjson`, ADR-0017): `Recorder`, `Scope`, combined reading, and `Tail` for the live stream. Payloads are built only with `activity::payload`. | Record code, statements, or rationales; make a tool fail. |

## Flow: `rationale prepare` (CLI)

```text
main.rs:cmd_prepare
  → pipeline::prepare (PrepareRequest, ProviderHandle, activity::Recorder)
      1. configuration::load
      2. storage::list_records
      3. project::resolve_target
      4. subjects::resolve_by_id_or_alias   (when the Record references a Subject)
      5. revision::snapshot + operations::Operation::new
                                            → activity: context.requested, provider.started
      6. ProviderHandle → resolve_target    (CBM session, spawned by cmd_prepare)
      7. revision::check_consistency
      8. cache::open / rebuild_fts / search_candidates / get_cached_assessment
      9. assessment::compute + cache::cache_assessment
      10. context::gather                   → activity: target.resolved, relationship.*, provider.finished
      11. retrieval::compile                (vNext packet; the budget is a ceiling)
      12. operations::save                  → activity: packet.compiled
  ← PrepareOutcome { packet, assessment, diagnostics, latency_ms, operation, activity }
main.rs: diagnostics → stderr, packet → stdout, activity: packet.delivered, session.ended
```

## Flow: `rationale serve` (MCP)

```text
main.rs → mcp::server::run()
  ProviderHandle::spawn()   ← ONCE for the whole life of the process
  loop { framing::read_message → dispatch by method → framing::write_message }
    "initialize"   → Session::observe_initialize → activity: agent.connected
    "prompts/list" → prompts::ACTIONS
    "prompts/get"  → prompts::render (a retired prompt answers with its replacement)
    "tools/call"   → handle_tools_call
      catch_unwind( match tool_name {
        "prepare_change"   → pipeline::prepare
        "explain_target"   → pipeline::explain
        "health"           → pipeline::health
        "finalize_change"  → pipeline::finalize
        "resolve_conflict" → canon::resolve_conflict → activity: conflict.resolved
      })
      → { content: [...], isError } — never lets a panic escape
```

## Flow: autonomous capture and human authority (1.0)

```text
An agent makes a real change in the repository
  → MCP tools/call "finalize_change" { operation_id, summary, candidates: [...] }
      → pipeline::finalize
          1. operations::load                (diff base: declared, the one from prepare, or HEAD)
          2. capture::capture                (mechanical diff, final revision, coverage)
          3. signals::*                      (observed facts; they inform, they do not write)
          4. canon::capture_candidates       (under CanonLock)
             per candidate: validation → durability → mechanical noise → bindings
                            → duplicates → explicit supersedes
             → discarded with a reason, or
             → canonical Record in .rationale/records/ (provenance: agent_asserted), or
             → if it replaces a `pinned` Record: conflict in .rationale-local/conflicts/ (not written)
          5. relationships::assess_records   (state of the relationships the new Records explain)
  ← FinalizeOutcome { committed, discarded, conflicts, superseded, relationships, signals, capture }

A human (interactive CLI, never an agent on its own)
  rationale pin <id> / unpin <id>         → declared authority + confirmation → lifecycle event
  rationale conflicts
  rationale resolve <id> keep-pinned      → the pinned rule keeps governing
  rationale resolve <id> adopt-new        → requires declared authority; the replacement inherits `pinned`
  (or MCP resolve_conflict with the literal human_answer, stored for audit)
```

## Flow: `rationale install-agent`

```text
main.rs:cmd_install_agent
  → agents::install (per project)
      ensure .rationale-local/ is excluded from Git
      migrate legacy absolute manifest entries
      for each detected agent (binary on PATH or project files):
        validate managed paths (no symbolic links)
        upsert the delimited block in CLAUDE.md / AGENTS.md / .cursor/rules/rationale.mdc
        Claude Code: shortcut skills from prompts::ACTIONS; retire prompt-only and retired actions by hash
        Claude Code and Codex: skill_bundle::FILES into .claude/skills/rationale/ and .agents/skills/rationale/
          (skipped with a report line when the directory is a symbolic link)
      save .rationale-local/installed-agent-files.json (a hash per owned file)
  → global registration per user (serve --client <agent>), unless the agent is absent
```

## Canonical versus derived boundary (Arquitectura §11.7)

```text
CANONICAL (Git, versioned)          DERIVED (local, regenerable)
.rationale/config.yaml               ~/.cache/rationale/projects/<sanitized-path>/derived.sqlite3
.rationale/subjects/                   - assessments_cache (invalidated by exact revision)
.rationale/records/                    - records_fts (FTS5)
.rationale/archive/proposals/        .rationale-local/ (excluded from Git)
                                       - activity/<session>.ndjson
                                       - operations/, conflicts/
                                       - installed-agent-files.json
```

Deleting everything derived never loses a decision — it is rebuilt from the
canonical layer (verified by `cache::tests::cache_rebuild_from_scratch_never_loses_canonical_data`).
