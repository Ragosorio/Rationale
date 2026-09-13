# 08 — Workspaces and monorepos (CBM-010)

**Source of evidence:** the project `Users-roor.osorio-Desktop-Monorepo` (12,367 nodes / 23,331 edges), a **real work monorepo** with genuine Turborepo and npm workspaces — not a synthetic fixture. Confirmed structure: `apps/{native,web,docs}` and `packages/{ui,crm-services,shared-types,api-client,icons,dev-mobile-bff,typescript-config}`, each with its own `package.json`, plus `turbo.json` at the root.

## Observed

1. **`get_graph_schema` has no `Package` or `Workspace` label.** There is a `Project` label with `count: 1` — the whole monorepo is modeled as a single project node, with no intermediate nodes for each workspace/npm package.
2. **The `packages` field of every cluster in `get_architecture(aspects=["clusters"])` always repeats the same value** (`osorio-Desktop-Monorepo`, the full project name) across the 12 detected clusters — the same as observed for Codebase Memory's own repository in `02-module-map.md`. This pattern is now confirmed in a real monorepo with genuine npm manifests, not only in a flat C project.
3. **Zero `IMPORTS` relationships cross the boundary from one package into another**, verified with three independent Cypher queries:
   - `apps/web` → any file outside `apps/web`: **0 results**.
   - Any `IMPORTS` with a destination inside `packages/*`: **10 sampled results, all intra-package** (for example `packages/shared-types/src/auth.ts` → `packages/shared-types/src/permissions.ts`; `packages/api-client/src/client.ts` → `packages/api-client/src/types.ts`).
   - No `apps/web` file appears as the origin of an `IMPORTS` toward `packages/ui`, `packages/crm-services`, `packages/api-client`, or any other shared package, even though these apps almost certainly consume those packages at runtime (the typical Turborepo monorepo pattern with specifiers such as `@repo/ui`).
4. There are 126 `Route` nodes and 90 `HTTP_CALLS` edges in this project — HTTP route resolution seems to be captured independently of import resolution between packages.

## Claimed

No documentation observed in this epic promises that `IMPORTS` resolves workspace package specifiers (for example `@repo/ui`) to the real physical package — it is an empirical observation, not a breach of a documented promise.

## Verified

- The three Cypher queries are reproducible and consistent with one another: the absence of package crossing is not an artifact of a single query but a pattern confirmed from both ends (searching outward from `apps/web`, and searching toward `packages/*` from any origin).
- The manifest structure (a `package.json` in each app/package + `turbo.json`) confirms this is indeed a monorepo with real, well-defined package boundaries, not a flat folder.

## Update (B1.2) — the functionality exists, and the failure is real and more serious than it seemed

`~/Desktop/codebase-memory-mcp/src/pipeline/pass_pkgmap.c` (1,849 lines) was read directly. Findings that **close** the earlier doubt about the root cause and reopen it in a more serious sense:

1. **The module exists exactly for this case.** Its header comment: *"Scans discovered files for manifest files (package.json, go.mod, Cargo.toml, ...) and builds a hash table mapping bare package specifiers to resolved module QNs. This enables IMPORTS edges for non-relative imports like '@myorg/pkg'"*. And explicitly: *"This is what lets bare workspace imports (e.g. '@org/pkg' declared in an ignored package.json) resolve..."*.
2. **The resolution mechanism is correct on paper:** it reads the `"name"` field of each `package.json` (`parse_package_json`), builds a specifier→module map, and uses prefix matching (`resolve_slash_prefix`, `resolve_dot_prefix`, `resolve_backslash_prefix`) to resolve imports such as `@repo/shared-types/foo` against the `@repo/shared-types` entry.
3. **The conditions for this to work were verified to be present in the real Monorepo:**
   - `packages/shared-types/package.json` declares `"name": "@repo/shared-types"`.
   - `packages/ui/package.json` declares `"name": "@repo/ui"`; `packages/crm-services/package.json` declares `"name": "@repo/crm-services"`.
   - `apps/web/app/types/interactions.ts` and `apps/web/app/types/kanban.ts` literally import `from "@repo/shared-types"`; `apps/web/app/components/Topbar.tsx` imports `from "@repo/icons/web"` — real bare imports, exactly the pattern `pass_pkgmap.c` says it resolves.
4. **The feature predates the indexing, not the other way around:** `pass_pkgmap.c` was introduced on 2026-04-15 (`feat: generic package/module resolution for IMPORTS edges across 10 languages`); the Monorepo `.db` was generated on 2026-07-22 — **more than 3 months later**. It is not a case of "the indexing predates the feature".

**Revised conclusion:** this is no longer an unimplemented design gap — it is a **reproducible failure of a feature that exists, is active, and should have resolved exactly these real imports, and did not.** It is a more serious failure mode than "absent capability": it is "capability present, silently ineffective".

## Unknown

- **The exact cause of why it failed in this specific indexing**, even though manifests and specifiers match the expected pattern. Hypotheses not ruled out: (a) the indexing mode used for the Monorepo (`fast`/`moderate`/`full`, not recorded at the time) may have skipped the `pass_pkgmap` step; (b) TypeScript aliases (`tsconfig.json` "paths", for example the `@/` prefix also seen in the same source code for internal `apps/web` imports) could interfere with prefix matching if the extractor does not distinguish the two mechanisms; (c) a genuine provider bug in this particular kind of project. It could not be isolated without re-indexing the Monorepo with an explicit `mode="full"` and comparing — **a concrete research item before Phase E**.
- Whether `index_repository(mode="cross-repo-intelligence")` (seen in `03-mcp-contracts.md`) solves this differently — not tested; that mode is documented for **crossing Routes/Channels between separately indexed projects**, not necessarily for packages within the same repository.

## Risk

**High, and more severe after B1.2 than it first seemed.** The cross-workspace success case that `Rationale_v0.5.md §32.0` uses as its canonical example (`RoleBadge → @boost/auth-contracts → authorization subject → approved constraint`) depends exactly on the structural provider resolving this class of relationship. Reading `pass_pkgmap.c` showed that **it is not a design gap** — it is a capability that exists, predates this indexing by months, and still did not resolve real imports that match exactly the pattern it claims to support. This validates the caution of `Rationale_v0.5.md §20.7` and `§14.1` in a stronger form than "the provider may not support this": **the provider may support it and still fail silently**, with no visible error or warning in the response.

## Decision impact

1. **Rationale cannot depend only on the structural provider's `IMPORTS`/`CALLS` edges to build the cross-workspace "relevance path"** that `Rationale_v0.5.md §19.3` and `§32.0` promise — not even when the provider declares support for the necessary mechanism. A mandatory, not optional, mitigation: manual/contractual bindings declared explicitly in `.rationale/bindings/` as a fallback (`Rationale_v0.5.md §20.7`: *"It must accept manual or contractual bindings as a fallback in the pilot"*), treated as the primary path for cross-package relationships in v1, not as a last-resort backup.
2. **`provider_gap` stops being a theoretical precaution and is confirmed in its most dangerous form**: it is not enough for Rationale to detect "the provider does not support X" (that would be manageable by declaring `unsupported`) — it must assume that **even a capability declared as supported may produce no results**, and therefore never treat `0 results` as equivalent to `0 real relationships`.
3. It confirms again (the third time in the epic, together with `02` and `05`) that the `packages` field of `get_architecture` must not be used as a source of workspace identity — neither in C projects without a manifest nor in a real npm monorepo with correct manifests.
4. A concrete research item before Phase E (not blocking D): re-index the Monorepo with an explicit `mode="full"` and repeat the same Cypher queries — it would determine whether the original indexing mode was the cause, which would change the severity from "provider bug" to "requires a specific indexing mode documented by Rationale".

## Reproduce

```text
get_graph_schema(project="Users-roor.osorio-Desktop-Monorepo")
get_architecture(project="Users-roor.osorio-Desktop-Monorepo", aspects=["clusters"])
query_graph(project="Users-roor.osorio-Desktop-Monorepo",
  query="MATCH (a)-[r:IMPORTS]->(b) WHERE a.file_path STARTS WITH 'apps/web' AND NOT b.file_path STARTS WITH 'apps/web' RETURN a.file_path, b.file_path, r.local_name LIMIT 10")
query_graph(project="Users-roor.osorio-Desktop-Monorepo",
  query="MATCH (a)-[r:IMPORTS]->(b) WHERE b.file_path STARTS WITH 'packages/' RETURN a.file_path, b.file_path LIMIT 10")
```
