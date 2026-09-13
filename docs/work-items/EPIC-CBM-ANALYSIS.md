# EPIC-CBM-ANALYSIS

## Problem

Rationale will integrate Codebase Memory as its first structural provider, but the architecture cannot settle the adapter, the transport, or the revision/coverage model without reproducible evidence of Codebase Memory's real behavior (`Rationale_Arquitectura_Conceptual_v0.1.md §0, §6, §7`).

## Goal

Produce the 13 research documents in `docs/research/codebase-memory/` with reproducible evidence, closing with an adapter-boundary recommendation (`12-integration-recommendation.md`).

## Non-goals

- The adapter is not implemented yet.
- No language is chosen here (that is the spike in `docs/research/language/`).
- The Codebase Memory clone is not modified.

## Base revision

Codebase Memory: `97ce23f9827177fff3858831156e9795c6832b18` (`DeusData/codebase-memory-mcp`, 2026-07-23). See `docs/research/codebase-memory/00-source-lock.md`.

## Subtasks (`Rationale_Proceso_Construccion_Agentes_v0.1.md §8`)

| Task | Description | Status | Notes |
|---|---|---|---|
| CBM-001 | Clone and lock revision | ✅ done | `00-source-lock.md` + `source-lock.yaml`. Found a version discrepancy: installed binary 0.8.1 versus a clone 338 commits ahead of v0.9.0 |
| CBM-002 | Build on MacBook Air M4 | ✅ done | `01-build-and-test.md`. Successful build, 2m49s, 296 MB binary, reports version `dev` |
| CBM-003 | Run tests | ✅ done (partial) | `01-build-and-test.md`. `test-foundation` fails at link time (`_suite_*` symbols); the full suite was not run because of its cost — documented as a known limitation, not Rationale's responsibility |
| CBM-004 | Index itself | ✅ done | `02-module-map.md`. 20,747 nodes / 77,956 edges |
| CBM-005 | Map modules | ✅ done | `02-module-map.md`. Finding: `packages` in clusters does not distinguish real modules |
| CBM-006 | Inspect MCP | ✅ done | `03-mcp-contracts.md`. 14 tools documented; CBM's ADR is a single document, not a decision log |
| CBM-007 | Inspect CLI | ✅ done | `04-cli-contracts.md`. Measured latency: 6.8 s cold, 2.2 s with a warm daemon |
| CBM-008 | Inspect revision and coverage | ✅ done | `05-revision-and-coverage.md`. **Critical finding:** `detect_changes` returned empty for 200 really modified files |
| CBM-009 | Inspect daemon and watcher | ✅ done | `06-daemon-and-watcher.md`. `hook_augment.c` confirms the non-blocking pattern of `v0.5 §20.7` with code evidence |
| CBM-010 | Inspect workspace support | ✅ done | `08-workspaces-and-monorepos.md`. **Critical finding:** zero `IMPORTS` relationships cross packages in the real Monorepo (8 npm packages) |
| CBM-011 | Measure CLI vs MCP | ✅ done (partial) | `11-performance-observations.md`. CLI measured formally; MCP only qualitatively — the formal measurement remains a research item before ADR-0002 |
| CBM-012 | Recommend adapter boundary | ✅ done | `12-integration-recommendation.md`. Synthesis and adapter-boundary recommendation |

Additional documents completed outside the original subtask list: `07-storage-and-cache.md`, `09-installation-and-agents.md`, `10-failure-modes.md` (a cross-cutting consolidation).

## The question that governs the entire architecture (CBM-008)

> Does Codebase Memory expose the revision it indexed, distinguish the working tree from HEAD, and report partial coverage verifiably?

Rationale's entire per-revision consistency guarantee depends on this (`Rationale_v0.5.md §4.8, §12.5, §20.3`; Subject `architecture.revision-consistency` in `.rationale/subjects/`). If the answer is negative or partial, it is not glossed over: it is documented as a finding, reproduced, and turned into an ADR — it may force deriving the revision from Git instead of trusting the provider.

## Risks

- The provider can have false relationships, empty traces, or silently empty results (`Rationale_v0.5.md §20.6`) — do not assume the absence of a relationship means it does not exist.
- Confusing "I did not find a relationship" with "the relationship does not exist" (`Rationale_Arquitectura_Conceptual_v0.1.md §4.6`).

## Plan

See `Rationale_Arquitectura_Conceptual_v0.1.md §7` for the structure of the 13 documents and their 6 mandatory sections (`Observed / Claimed / Verified / Unknown / Risk / Decision impact`).

## Tests

No code applies yet. "Test" in this epic means: every research command must be reproducible by another agent with the same declared results.

## Docs

The 13 files `docs/research/codebase-memory/00-source-lock.md` … `12-integration-recommendation.md`.

## Success criterion

The 13 documents exist, with reproducible commands, complete sections, and CBM-008 answered explicitly with evidence — even if the answer is "it does not expose it".

**Status: complete.** The 13 documents (`00`–`12`) plus `source-lock.yaml` are written with reproducible evidence. Two critical findings with direct architectural impact:

1. `detect_changes` did not detect 200 really modified files (CBM-008) → Rationale's Revision Coordinator must derive its own revision truth from Git and never trust the provider's signal.
2. Zero `IMPORTS` relationships cross packages in a real monorepo of 8 npm packages (CBM-010) → Rationale's cross-workspace retrieval cannot depend only on provider edges; it needs manual/contractual bindings as a fallback.

Research items open before ADR-0002: formal MCP latency measurement, reading `pass_pkgmap.c`, and confirming whether the coverage finding through the CLI at HEAD also appears over MCP. See `12-integration-recommendation.md §Next research items` (all three were later resolved in B1; see that document).
