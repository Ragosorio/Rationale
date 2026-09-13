# 03 — MCP contracts (CBM-006)

**Source of evidence:** the MCP tools active in this session, served by the **installed binary (0.8.1)**. The exact list of tools available in a session may depend on client configuration; this is the observed surface, not necessarily exhaustive (see Unknown).

## Observed

Observed MCP tools and their exact contract:

| Tool | Required input | Contract notes |
|---|---|---|
| `list_projects` | none | Returns `{name, root_path, nodes, edges, size_bytes}` per indexed project. No revision or coverage. |
| `index_status` | `project` | `{project, nodes, edges, status}`. No revision or generation (see `05-revision-and-coverage.md`). |
| `index_repository` | `repo_path`; optional `mode` (`full`\|`moderate`\|`fast`\|`cross-repo-intelligence`), `persistence` (bool), `target_projects` | **Relevant finding:** `persistence: true` writes a compressed artifact to `.codebase-memory/graph.db.zst` "for team sharing" — CBM already considers sharing the derived index among team members, something Rationale's model (`storage.canonical-vs-derived`) deliberately treats as not shared by default. See Decision impact. |
| `get_architecture` | `project`; optional `aspects[]` | With `aspects=["clusters"]` it returns Leiden communities with cohesion, `top_nodes`, and `packages` (see `02-module-map.md` — `packages` does not distinguish modules in projects without a manifest). |
| `get_graph_schema` | `project` | Returns node labels/edge types and their properties — the graph's shape contract is introspectable, which is valuable for the adapter. |
| `search_graph` | `project`; combines `query` (BM25 full-text), `name_pattern` (regex), `semantic_query` (an array of keywords; requires moderate/full mode) | Three independent, combinable modes. Explicit pagination through `limit`/`offset`/`has_more`/`total` — **a well-defined pagination contract**, relevant for bounding the token budget in Rationale's adapter. |
| `search_code` | `project`, `pattern` | Graph-augmented grep; groups matches into containing functions and ranks by structural importance. Truncates at `limit` (default 10) with no `offset` — to see more, raise `limit` or narrow with `path_filter`/`file_pattern`. |
| `trace_path` | `function_name`, `project` | `calls`/`data_flow`/`cross_service` modes; includes optional `risk_labels` (`CRITICAL`/`HIGH`/`MEDIUM`/`LOW` by hop distance) — a pre-existing structural risk signal Rationale could consume as evidence, not as a normative decision. |
| `get_code_snippet` | `qualified_name`, `project` | Requires the exact `qualified_name` obtained from `search_graph` first — it is not a search tool. |
| `query_graph` | `query` (Cypher), `project` | A hard ceiling of 100k rows; no `offset`; broad queries need an explicit `LIMIT` in the Cypher itself. |
| `detect_changes` | `project`; optional `since`, `base_branch` (default `main`), `depth`, `scope` | See the critical finding in `05-revision-and-coverage.md`: it returned empty results in all three tests despite real changes verifiable through Git. |
| `manage_adr` | `project`; `mode` (`get`\|`update`\|`sections`); optional `content`, `sections[]` | **Relevant finding:** CBM's "ADR" is **a single markdown document per project**, with suggested fixed sections (`PURPOSE, STACK, ARCHITECTURE, PATTERNS, TRADEOFFS, PHILOSOPHY`) — not a collection of individual decisions with dates, alternatives, provenance, or supersession. Confirmed live: `manage_adr(mode="get")` on the CBM project returned `{"status": "no_adr", "content": ""}`. |
| `ingest_traces` | `traces[]`, `project` | Accepts runtime traces to enrich the graph — not tested in this session (it requires real trace data). |
| `delete_project` | (not inspected in depth) | Administrative; not run, to avoid destroying the existing indexes used as evidence in this epic. |

## Claimed

The tool descriptions (visible as MCP metadata) explicitly document usage rules: "use instead of grep", pagination limits, index mode requirements (`semantic_query` requires moderate/full mode), and `manage_adr`'s hint about how to structure the content. The level of documentation embedded in the tool contract is notably higher than in a typical CLI.

## Verified

- The behavior of `manage_adr(mode="get")` returning `no_adr` is reproducible and consistent with no CBM ADR ever having been written for this project.
- The pagination contracts (`search_graph`, `search_code`, `query_graph`) are documented consistently with one another (variants of the same `limit`/`total`/truncation pattern).

## Unknown

- Whether the list of tools observed in this session is exhaustive or whether the MCP client filtered/configured it — it was not compared against a canonical list published by CBM.
- What exact contract difference exists between the 0.8.1 release binary and the local HEAD build (`dev`) — not compared side by side in this session (it would require configuring a second MCP connection against the local binary, out of immediate scope).
- How `index_repository(mode="cross-repo-intelligence")` behaves in practice — not run (it requires multiple target projects and could modify the state of already-indexed projects used as evidence).

## Risk

**Medium.** The `persistence: true` finding in `index_repository` (sharing the compressed derived index with the team) is a direct architectural temptation: **Rationale must not adopt this pattern for its own derived layer** (`Rationale_v0.5.md §26.3`, Subject `storage.canonical-vs-derived`) — CBM's index may be shared for the provider's performance convenience, but Rationale's `Assessments` remain the responsibility of local rebuilding per machine, not of syncing a binary artifact.

## Decision impact

- It confirms that CBM's `manage_adr` **neither replaces nor competes with** Rationale's Record model (`Rationale_v0.5.md §7.4`, §20.2): it is a single-piece living architecture document, with no per-assertion provenance, no per-domain authority, no structured evidence, and no versioned supersession. Rationale may eventually **read** CBM's ADR as a source of evidence (`stated`/`inferred`), never as the equivalent of an approved Record.
- The pagination contract of `search_graph`/`search_code`/`query_graph` is reusable as a design pattern for the `context_budget` of Rationale's own Context Compiler (`Rationale_v0.5.md §18`).
- `risk_labels` in `trace_path` is a structural signal usable as **evidence** of risk (`Rationale_v0.5.md §10.1`, observed facts), never as an approved `Decision` or `Constraint` on its own.
- Relevant for ADR-0002 (transport): this MCP surface is rich and already paginated/bounded — it reinforces the option of consuming it over MCP instead of a CLI subprocess, pending measuring real latency in CBM-011.

## Reproduce

```text
list_projects()
index_status(project="Users-roor.osorio-Desktop-codebase-memory-mcp")
get_graph_schema(project="Users-roor.osorio-Desktop-codebase-memory-mcp")
get_architecture(project="Users-roor.osorio-Desktop-codebase-memory-mcp", aspects=["clusters"])
manage_adr(project="Users-roor.osorio-Desktop-codebase-memory-mcp", mode="get")
```
