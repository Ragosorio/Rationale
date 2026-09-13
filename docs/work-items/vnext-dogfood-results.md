# vNext dogfood — results

Date: 2026-09-13. Branch `codex/rationale-vnext`, base `adf9c6e`.
Binary: `target/debug/rationale` built from that base. Provider: the user's
real Codebase Memory 0.8.1 (`successful / complete`). Local activity on.

## What ran

One persistent MCP session of `rationale serve --client claude-code` in this
repository, driven over stdio the way an agent drives it, while
`rationale ui` was open on `127.0.0.1:9748`. No global agent configuration
was touched.

The change: a client that still asks for a retired prompt (`review`, the
pre-vNext approval queue) received a bare `prompt desconocido`. `prompts/get`
now answers with what replaced it (`conflicts`, and `rationale migrate` for
legacy proposals).

| Step | Observed |
|---|---|
| `initialize` + `health` | Git `adf9c6e`, clean tree, provider `successful / complete`. |
| `prepare_change(src/mcp/server.rs::handle_prompts_get, intent)` | `op_1a0996617282ec7cc39`, 119 ms, 10 nodes / 10 relationships selected, 7.0 KB (~1.8k tokens), target source included, consistency `structural-index-behind`. |
| Governance | `constraint.kind-inferred-from-record-id-prefix` governs the whole file by file binding; intent conflict `undetermined`, no shared terms. The intent does not touch `kind` inference, so it respects the Record — stated before editing. |
| Change + tests | `RetiredAction { name, replacement }`; `handle_prompts_get` names the replacement. `cargo clippy -D warnings` clean, 366 tests pass. |
| `finalize_change` (same session, same `operation_id`) | 3 candidates → **3 committed, 0 discarded, 0 conflicts**. Symbol bindings confirmed by the provider with portable ids (`src.agents.serve_args`, no machine prefix), `provisional: true` because the code was uncommitted; mechanical capture `entirely-uncommitted` over base `adf9c6e`. |
| Records on disk | `authority: normal`, `provenance.kind: agent_asserted`, client `claude-code` (source `flag`), `operation_id` and `session_id` linked, lifecycle event `asserted`. `rationale doctor`: no findings. |
| Retrieval of the new memory | A second `prepare_change` on `src/agents.rs::expected_managed_entry` with an intent that contradicts the new constraint returned it as the governing critical constraint (`structural` match), used its rationale as `primary_reason`, and marked `expected_managed_entry` and `skill_names` with its id in the structural subgraph. |
| Control Room | Live: session "Claude Code · activo · 2 op", both operations, capture strip "3 candidatos · 0 descartados · 3 escritos", timeline `change.finalized → capture.candidate × 3 → record.committed × 3`, Memory 15/15 with the three Records marked new and agent-asserted, node detail with the explaining Record. Closing stdin produced `session.ended`, and the session turned "terminado" without reload. |

Records written (one decision each — the two about retired actions have
different lifetimes, so they are separate):

- `constraint.retired-skills-remain-managed-destinations`
- `decision.retired-prompts-name-their-replacement`
- `decision.mcp-registration-identifies-the-client`

## Findings

1. **Codebase Memory resolves some Rust calls by name.** The packet for
   `handle_prompts_get` listed as `observed` callees
   `src.subjects.Resolution.action` (the real call is `prompts::action`),
   `src.ui.mod.get` (`serde_json::Value::get`) and `src.ui.http.Response.json`
   (the `json!` macro). `observed` means "present in the provider's index",
   which is true; it is not a semantic guarantee. Rationale reaches the
   provider only through its public MCP tools
   (`constraint.no-provider-internal-access`), so this is reported, not
   patched around.
2. **The lexical intent-conflict polarity is noisy in both directions.** With
   an intent that directly contradicts the new constraint, the constraint came
   back `undetermined` (both texts contain `without`), while
   `risk.agent-test-unix-path-is-platform-scoped` came back `opposed` with no
   shared terms. The heuristic is a deliberate fence: a test pins a real
   cross-language conflict with no literal overlap, and every conflict carries
   the note that lexical overlap is unverified. Left as is; the governing
   Record itself was surfaced correctly, which is what the agent must act on.
3. **Index freshness and symbol confirmation disagree in wording.** The
   snapshot said `structural-index-behind` while the provider confirmed
   symbols added after the last commit. Both are reported faithfully; no
   behavior depends on reconciling them.
4. **Historical test pollution remains in local activity.** Pairs of Codex
   sessions with two events and no end date from before the integration tests
   disabled activity; the UI labels them idle, not connected. They are local,
   Git-excluded data and were not deleted.

## Not exercised live

Pinning and resolving a conflict are human authority acts: `rationale pin`
requires an interactive terminal and an actor declared in
`.rationale/config.yaml`, and `resolve_conflict` requires the human's literal
answer. An agent run cannot honestly perform them. The chain is covered by
`tests/dogfood_governance_chain.rs` (candidate over a pinned Record →
conflict → human decision).

## Environment notes

- The Browser pane's launcher could not start `rationale ui`: the process
  blocked in `dyld` opening its `~/Desktop` working directory, the signature
  of a pending macOS privacy prompt for a process spawned by the app. The UI
  server ran from the shell instead.
- The session's own `rationale` MCP server is the installed beta.3, so the
  vNext chain was driven through the built binary directly.
