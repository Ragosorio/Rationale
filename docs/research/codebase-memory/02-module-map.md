# 02 — Module map (CBM-005)

**Source of evidence:** an explicit mix of two distinguished sources — (a) the indexed graph of the **0.8.1 binary** through MCP (`get_architecture`), and (b) direct reading of the **source code at HEAD `97ce23f9`**. Each finding states its source.

## Observed

### From the source code (HEAD)

`src/` has 17 top-level modules plus `internal/cbm/` (extraction and grammars). Approximate size by lines of `.c`/`.cpp`:

| Module | Lines | Apparent role |
|---|---:|---|
| `cli/` | 33,015 | The largest module in the project. CLI, installation, transactional activation, Windows launcher state |
| `pipeline/` | 23,973 | Extraction core: `pass_definitions`, `pass_calls`, `pass_usages`, `pass_semantic`, `pass_tests`, `pass_githistory`, `pass_gitdiff`, `pass_configures`, `pass_route_nodes`, `pass_cross_repo`, `pass_pkgmap`, `pass_k8s`, `pass_complexity`, among others |
| `daemon/` | 18,439 | Daemon lifecycle, coordination between sessions, IPC, "version cohort" |
| `mcp/` | 12,318 | MCP server, indexing supervisor, compact output |
| `foundation/` | 9,936 | Primitives: arena, hash table, string interning, logging, platform, locks |
| `store/` | 7,799 | Persistence (SQLite + its own writer) |
| `ui/` | 4,165 | Embedded HTTP server, 3D graph visualization |
| `cypher/` | 4,803 | Cypher query engine (used by `query_graph`) |
| `discover/` | 3,218 | Language discovery, `.gitignore`, user configuration |
| `semantic/` | 2,243 | AST profile, semantic analysis |
| `graph_buffer/` | 1,843 | In-memory graph buffer |
| `launcher/` | 1,684 | Launcher (relevant for Windows) |
| `watcher/` | 1,440 | File watching |
| `simhash/` | 538 | MinHash / similarity |
| `git/` | 421 | Git integration — surprisingly small given that `pipeline/` already contains `pass_gitdiff.c` and `pass_githistory.c`; the Git logic seems spread between this thin module and those passes rather than concentrated here |
| `traces/` | 142 | Traces |

`internal/cbm/` contains the multi-language extraction layer: ~180 `grammar_<language>.c` files (one per language supported through tree-sitter) plus `extract_*.c` (definitions, calls, imports, usages, semantic, type_refs, env_accesses, k8s, channels) and the tree-sitter runtime.

### From the indexed graph (0.8.1 binary, through `get_architecture(aspects=["clusters"])`)

- Community detection (Leiden) on the call graph produced **13 clusters**, with cohesion between 0.58 and 1.0.
- The largest clusters (36–38 members) have `top_nodes` such as `run`, `main`, `require`, `check`, `SmokeFailure`, `probe_future_generation_rendezvous` — consistent with clusters centered on **testing/smoke infrastructure**, not on the product domain.
- **Relevant finding:** every cluster's `packages` field always returns the same value (`osorio-Desktop-codebase-memory-mcp`, the project name), for all 13 clusters. Sub-packages or internal modules are not identified through this field — that is, **for a C project without package manifests (no npm/cargo/go.mod), `get_architecture` does not distinguish internal modules as separate packages**; it only groups by call community.

## Claimed

No documentation in the analyzed surface promises that `packages` distinguishes internal modules of a single-language project without manifests — this is an observation, not a broken promise.

## Verified

- The relative size of modules (CLI and pipeline as the largest) is consistent with the project's own commit history observed in `00-source-lock.md` (dominated by Windows hardening, the daemon, and test infrastructure).
- The 13 clusters and their cohesion are reproducible with the same call.

## Unknown

- How accurately Leiden clustering separates **product** modules (pipeline, store, mcp) from **test infrastructure** modules — the `top_nodes` of the largest clusters suggest that much of the "observed architecture" from clustering describes the test scaffolding, not the functional domain.
- Whether `get_architecture(aspects=["all"])` (not tested; only `overview` and `clusters` were) exposes a more useful hierarchical view by module/folder instead of by call cluster.

## Risk

**Low to medium.** It does not invalidate using Codebase Memory, but it confirms that **the automatic architectural view does not replace reading the real folder structure** to understand a project's module boundaries — relevant for Rationale's own adapter, which should base its understanding of "module" on provider conventions (manifests, folders) rather than assume structural clustering always reflects domain boundaries.

## Decision impact

- It confirms the Subject `architecture.provider-boundary`: Rationale must treat Codebase Memory's clustering/architecture output as one more signal, not as the source of truth about modules or packages — especially in projects without clear package manifests.
- Relevant for CBM-010 (workspaces/monorepos): if the `packages` field does not distinguish modules in a flat C repository, it must be checked specifically on the Monorepo fixture (`~/Desktop/Monorepo`, which does have npm/similar manifests) whether `packages` is populated correctly there — see `08-workspaces-and-monorepos.md`.

## Reproduce

```text
get_architecture(project="Users-roor.osorio-Desktop-codebase-memory-mcp", aspects=["clusters"])
```

```bash
cd ~/Desktop/codebase-memory-mcp
ls -F src/
for d in src/*/; do echo "$d: $(find "$d" -name '*.c' -o -name '*.cpp' | xargs wc -l 2>/dev/null | tail -1)"; done
```
