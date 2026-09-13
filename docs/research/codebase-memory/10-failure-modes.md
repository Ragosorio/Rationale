# 10 — Failure modes

It consolidates the failure modes observed throughout the entire epic, plus two additional deliberate edge-case tests. Each entry cross-references the document where it originated.

## Observed

| # | Failure mode | Source | Observed behavior |
|---|---|---|---|
| 1 | Real changes not detected | `05-revision-and-coverage.md` | `detect_changes` returned `{"changed_files": [], "changed_count": 0}` with three different `since` formats, despite 200 really modified files verifiable through Git. **A silent failure** — no error, no warning, just an empty result indistinguishable from "there were no changes". |
| 2 | Inconsistent test build target | `01-build-and-test.md` | `make -f Makefile.cbm test-foundation` fails at link time with dozens of unresolved `_suite_*` symbols. **An explicit, loud failure** (the linker aborts with an error message) — unlike case 1, this one is impossible to ignore. |
| 3 | Three mutually inconsistent version identifiers | `00`, `01`, `06` | `--version` → `0.8.1` (release) / `dev` (local build); `git describe` → `v0.9.0-338-g97ce23f9`; `daemon status` → `build: dev (52ddfafc803f...)`. None of the three can be used to infer the capability compatibility of another. |
| 4 | Missing cross-package coverage with no warning signal | `08-workspaces-and-monorepos.md` | Zero `IMPORTS` relationships cross packages in a real monorepo of 8 npm packages. The query returns no error or warning — there are simply no edges, and nothing in the response indicates "this might be incomplete". |
| 5 | Invalid Cypher query | Direct test in this session | `query_graph` with invalid syntax (`"THIS IS NOT VALID CYPHER {{{"`) returns `{"error": "expected token type 0, got 85 at pos 0"}"` — **an explicit error, correct in that it does not fabricate a result**, but it exposes internal implementation detail (parser token numbers) instead of a user-oriented message. |
| 6 | Nonexistent symbol in `get_code_snippet` | Direct test in this session | With an invented `qualified_name`, it returns `{"error": "symbol not found. Use search_graph(name_pattern=\"...\") first to discover the exact qualified_name..."}"` — **the best error pattern observed in the entire epic**: explicit, it fabricates no content, and it says exactly what to do next. |
| 7 | Project not found | Observed repeatedly when passing the `project` parameter incorrectly | It returns `{"error": "project not found or not indexed", "hint": "...", "available_projects": [...]}` — also a good pattern: explicit, with a list of valid alternatives. |

## Claimed

No CBM documentation explicitly promises a unified catalog of failure modes or a declared "fail loud versus fail silent" policy — it is inferred only from case-by-case observation.

## Verified

All 7 listed failure modes were reproduced directly in this research session (they are not second-hand inferences).

## Unknown

- Whether a consistent internal policy explains why some failures are loud (2, 5, 6, 7) and others completely silent (1, 4) — it looks more like a consequence of which application layer detects the problem (parser/input validation versus a graph query that simply finds no edges) than a deliberate policy, but it was not confirmed with the CBM team or through further code reading.
- Whether newer versions (after HEAD `97ce23f9`) specifically address cases 1 and 4.

## Risk

**The most important cross-cutting finding of the entire epic:** the **loud** failure modes (2, 5, 6, 7) are manageable — an adapter can catch and translate them. The **silent** modes (1, 4) are structurally dangerous because **nothing in the response distinguishes "there are no changes/relationships" from "I could not detect them"**. This is precisely the central warning of `Rationale_v0.5.md §4.9` and `§19.2`: *"No relationship was found"* is not the same as *"the relationship was shown not to exist"*, and here two real, reproducible cases showed that the provider does not distinguish the two in its response.

## Decision impact

1. **Rationale's `CodeIntelligenceProvider` adapter (`Rationale_v0.5.md §21`) must treat every absence of a relationship or change as `unknown`/`not found within the available coverage`, never as a negative confirmation** — this was already in the conceptual contract (`§19.2`) and now has two concrete empirical cases supporting it (cases 1 and 4).
2. **Never propagate the provider's raw error messages (case 5) directly to the agent** without normalizing them — the adapter must translate internal errors into the explicit states that `Rationale_Arquitectura_Conceptual_v0.1.md §11.1` requires (`supported/unsupported/degraded/unknown`), not forward strings such as `"expected token type 0, got 85 at pos 0"`.
3. **Patterns 6 and 7 (an explicit error + an actionable suggestion + alternatives) are the standard to match**, not merely something to avoid — Rationale's own Subject `policy.no-inferred-blocks` benefits from a provider that fails that clearly when it can.
4. This closes the evidence needed to write `12-integration-recommendation.md` with an explicit section on the adapter's failure handling.

## Reproduce

```text
query_graph(project="...", query="THIS IS NOT VALID CYPHER {{{")
get_code_snippet(project="...", qualified_name="this_function_does_not_exist_anywhere_xyz")
index_status(project="project-that-does-not-exist")
```
