# 05 — Revision and coverage (CBM-008)

**This is the question that governs Rationale's entire per-revision consistency architecture** (`Rationale_v0.5.md §4.8, §12.5, §20.3`; Subject `architecture.revision-consistency`). It is analyzed before the rest of the epic because it can invalidate assumptions in other documents.

**Source of evidence:** every observation in this document comes from the **installed binary (0.8.1)** invoked through the MCP tools active in this session (`mcp__codebase-memory-mcp__*`), not from reading the source code in the clone (HEAD `97ce23f9`). See `00-source-lock.md` — the two sources are 338 commits apart and must not be mixed.

## Question

> Does Codebase Memory expose the revision it indexed, distinguish the working tree from HEAD, and report partial coverage verifiably?

## Observed

1. **`index_status(project)`** returns only:
   ```json
   {"project": "...", "nodes": 20747, "edges": 77956, "status": "ready"}
   ```
   It includes no Git revision, index generation, indexing timestamp, or any coverage field.

2. **`get_architecture(project, aspects=["overview"])`** returns only `project`, `total_nodes`, and `total_edges`. The same revision/coverage gap.

3. **`get_graph_schema(project)`** exposes the graph's shape (node labels, edge types, and their properties), but no node or edge carries a project-level Git revision property. The `File` node does have `last_modified` (see point 4) and `change_count`.

4. **`last_modified` on `File` nodes is a Unix (epoch) timestamp, not a Git SHA.** A real example:
   ```json
   {"f.name": "server.json", "f.last_modified": "1781251641", "f.change_count": "9"}
   ```
   `change_count` is a counter (possibly of indexings or of commits that touched the file) with no unit documented anywhere visible in this contract — see Unknown.

5. **`detect_changes(project, since=X)` returned `{"changed_files": [], "changed_count": 0, "impacted_symbols": [], "depth": 2}` for all three `since` formats tested:** `HEAD~5`, an explicit commit SHA (`50d0cc8c`), and a date (`2026-01-01`).

6. **Cross-check with real Git:** in the same clone, `git diff --stat HEAD~5..HEAD` shows **200 modified files, 90,964 insertions, and 3,533 deletions** — real, substantial changes exist in that range. `detect_changes` reported them as zero in all three attempts.

## Claimed

`get_graph_schema` itself includes an `adr_hint` suggesting `get_architecture(aspects=['all'])` before using `manage_adr` — that is, the product presents itself as aware of needing more context than the basic `overview`. No live documentation was found, in the tool surface available in this session, that explicitly promises "indexed revision" or "partial coverage" as response fields.

## Verified

- The revision/coverage gap in `index_status` and `get_architecture` is reproducible (repeated across multiple calls).
- The empty `detect_changes` result is reproducible with three different `since` formats, and it directly contradicts the real Git state in the same repository.

## Unknown

- **Cross-update with `02-module-map.md`:** the build pipeline (`src/pipeline/`) does include `pass_gitdiff.c` and `pass_githistory.c` — that is, the source at HEAD contains passes dedicated to Git diff and history. This makes it **more likely** that `detect_changes` is implemented and working internally, and that the observed empty result is a problem specific to: (a) the 0.8.1 release binary (outdated relative to HEAD), (b) the contract of the `since` parameter as exposed over MCP, or (c) an unmet precondition (for example, it requires the `project` to have been indexed with a mode that enables these passes). Which one could not be confirmed without access to testing the locally built binary against the same MCP calls in this session.
- **Whether the empty `detect_changes` result is a real bug in the 0.8.1 build, a documented limitation, or a misunderstanding of the tool's contract** (for example: `detect_changes` may compare the current working tree against the *last indexed revision* rather than freely against the `since` argument; or it may require the daemon/watcher to be running; or it may require re-indexing first). None of these explanations is ruled out without more evidence.
- What exactly `change_count` means on a `File` node (commits that touched the file? times it was re-indexed?) — not documented in the exposed schema.
- Whether the source build at HEAD (`97ce23f9`, pending in CBM-002) exposes revision/coverage fields that the 0.8.1 release does not — that is, whether this was already fixed in the 338 later commits.
- Whether an additional MCP tool not tested in this session (the list of available tools may not have been exhaustive) reports revision/generation/coverage explicitly.

## Risk

**High, and no longer only hypothetical — it was confirmed empirically.** `Rationale_v0.5.md §4.9` warns about exactly this scenario: "an absent relationship may mean it does not exist... or that the index is behind... or that there was a provider error", and it requires distinguishing "no relationship was found" from "the relationship was shown not to exist". Here the most severe possible case was observed: a tool designed specifically to detect changes (`detect_changes`) returned zero changes when 200 modified files verifiable through Git existed. If Rationale trusted this signal to decide whether to degrade an `Assessment`, it would fail at exactly the worst moment: when real changes did happen that should make a decision `stale`.

This directly validates the principle of `Rationale_Arquitectura_Conceptual_v0.1.md §4.6` ("fail with humility") and reinforces why `Rationale_v0.5.md §4.16` requires freshness to be checked actively by comparing Git revisions, **never delegating that check to the structural provider without independent verification**.

## Later update (see `04-cli-contracts.md` and B1.3)

When testing the same `index_status` through the **CLI on the binary built at HEAD** (not the 0.8.1 release used in this document), the response **did include** structured coverage fields: `parse_partial`, `skipped`, and `not_indexed` (with counts and a `truncated` flag). This was not present in the MCP call on 0.8.1 documented above, leaving open whether the difference was one of version or of transport.

**Resolved (B1.3):** we built our own stdio MCP client (JSON-RPC 2.0 framed with `Content-Length`, see `11-performance-observations.md`) and invoked `index_status` **over real MCP, not the CLI**, against the same HEAD binary. The MCP response included exactly the same coverage fields (`parse_partial`, `skipped`, `not_indexed`, all 0/empty for this already fully indexed project). **Definitive conclusion: the difference is one of version, not of transport.** The MCP protocol does carry these fields correctly; the 0.8.1 release simply did not produce them yet. This is good news: the MCP contract is not the bottleneck, and more recent Codebase Memory versions do expose structured coverage consumable by Rationale's adapter.

The empty `detect_changes` finding (see below) remains unexplained and is not affected by this — it is still an independent problem, not resolved by this update.

## Decision impact

1. **Rationale's Revision Coordinator cannot depend on `detect_changes` (or on any provider revision field) as the source of truth about whether the code changed.** The canonical revision must be derived directly from Git (`git rev-parse HEAD`, `git status`, working-tree hashes) on Rationale's side, and used to invalidate/degrade `Assessments` independently of what the structural provider reports or fails to report. This confirms the decision already made in `Rationale_v0.5.md §4.16` and `§15.7`, turning it from a preventive principle into a demonstrated necessity.
2. **`Coverage` as a field (`Rationale_v0.5.md §5.3`, `§21.1`) can be populated — confirmed by B1.3 — but only against Codebase Memory versions after 0.8.1.** The adapter must negotiate capabilities (`capabilities()`) to know whether `parse_partial`/`skipped`/`not_indexed` are available in the installed version, and degrade to `unknown` when they are not — never assume the field exists just because this epic's schema documents it.
3. **Resolved:** the source build at HEAD, invoked both through the CLI and through real MCP (B1.3), consistently exposes structured coverage. The gap was the release binary's version, not the transport or the provider's current code.
4. It directly affects ADR-0006 (Revision fingerprint): Rationale's revision fingerprint must be built entirely on Rationale's side (Git + a working-tree hash), treating any revision reported by the provider as additional low-confidence data, never as the authoritative source.

## Reproduce

```text
# Requires the project to be already indexed in Codebase Memory (MCP tools):
index_status(project="Users-roor.osorio-Desktop-codebase-memory-mcp")
get_architecture(project="Users-roor.osorio-Desktop-codebase-memory-mcp", aspects=["overview"])
get_graph_schema(project="Users-roor.osorio-Desktop-codebase-memory-mcp")
query_graph(project="...", query="MATCH (f:File) RETURN f.name, f.file_path, f.last_modified, f.change_count LIMIT 5")
detect_changes(project="...", since="HEAD~5")
detect_changes(project="...", since="<commit-sha>")
detect_changes(project="...", since="2026-01-01")
```

```bash
# Independent cross-check in the real clone:
cd ~/Desktop/codebase-memory-mcp
git diff --stat HEAD~5..HEAD
```
