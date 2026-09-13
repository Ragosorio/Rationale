# ADR-0006: Revision fingerprint

**Status:** proposed — pending independent cross-review before `accepted`.
**Date:** 2026-07-25
**Deciders:** Claude Code (analysis and implementation); pending human approval and/or cross-review by another agent
**Supersedes / Superseded by:** none

## Context

`Rationale_v0.5.md §4.8` requires every context package to declare a revision that is consistent across Git, the structural provider, and Rationale's assessments, and requires the tool to degrade or refuse a response rather than serve plausible but incorrect context. `Rationale_Arquitectura_Conceptual_v0.1.md §11.4` defines the Revision Coordinator as the module responsible for this comparison, but did not settle where the "true" revision must come from.

`docs/research/codebase-memory/05-revision-and-coverage.md` (CBM-008) produced direct, reproducible empirical evidence: Codebase Memory's `detect_changes` returned `{"changed_files": [], "changed_count": 0}` for three different `since` formats (`HEAD~5`, an explicit SHA, a date), even though `git diff --stat` over the same range showed **200 modified files, 90,964 insertions, and 3,533 deletions**. This is exactly the scenario `Rationale_v0.5.md §4.9` warned about in the abstract ("no relationship was found" ≠ "the relationship was shown not to exist") — now demonstrated with concrete data.

## Decision

Rationale's **revision fingerprint** is computed **exclusively from Git** (`git rev-parse HEAD`, working-tree state, content hashes where applicable) on Rationale's side. Any revision, coverage, or change signal reported by Codebase Memory (or any future structural provider) is treated as **additional low-confidence data**, never as the authoritative source of whether the code changed.

## Evidence

- `05-revision-and-coverage.md`: `detect_changes` failed with all three `since` formats tested, missing 200 files of real difference.
- `08-workspaces-and-monorepos.md` (B1.2): an independent finding of the same nature — a package-resolution capability (`pass_pkgmap.c`) that exists and is active still produced no real cross-package relationship in a genuine monorepo. It reinforces the pattern: **the provider can fail silently even when the capability is present and active**, not only when it is absent.
- `12-integration-recommendation.md`: consolidates both findings as the weightiest reason not to delegate any "this changed" or "this did not change" claim to the provider.

## Alternatives considered

- **Using Codebase Memory's `detect_changes` as the primary source of changes**: discarded — the evidence in `05` shows it can silently return zero changes when hundreds of real modified files exist, with no error or warning in the response.
- **Using the indexed revision the provider reports (`index_status`) as the reference snapshot**: discarded as the sole source — `index_status` does not even expose a Git revision in the tested version (0.8.1); where it does (the HEAD build, `04-cli-contracts.md`), it still does not solve the `detect_changes` problem, which is independent.
- **Trusting the binary's version string to infer whether its revision data is reliable**: discarded — `00-source-lock.md` and `06-daemon-and-watcher.md` document **three mutually inconsistent version identifiers** (`--version`, the clone's `git describe`, the `daemon status` hash), none usable for that inference.

## Consequences

- Rationale's Revision Coordinator (`Arquitectura_Conceptual_v0.1.md §11.4`) is fully decoupled from the structural provider's reliability for its most critical function (knowing whether the code changed) — a robustness advantage, not a grudgingly accepted limitation.
- Codebase Memory is still consulted for its proper function: structure, symbols, relationships, impact — never for "what changed since the last revision?".
- The adapter must record the revision/generation the provider reports (when it does) only as diagnostic metadata (`provider_generation`, `Rationale_v0.5.md §21.1`), never as an input to an invalidation decision.
- Any Rationale `Assessment` becomes `stale` or `unknown` as soon as the Git fingerprint (computed by Rationale) differs from the fingerprint recorded in the `Assessment`, regardless of what the provider says.

## Risks

- Computing the fingerprint only from Git does not capture changes in unversioned files (generated or ignored ones) — acceptable, because Rationale's conceptual model already limits its scope to what is versioned in Git (`Rationale_v0.5.md §4.19`).
- The cost of computing the working-tree state (not only `HEAD`) may not be trivial in very large repositories — to be measured in Phase D with the real vertical slice; not blocking for this decision.

## Validation

Reproducible evidence in `docs/research/codebase-memory/05-revision-and-coverage.md §Reproduce` and `08-workspaces-and-monorepos.md §Reproduce`. The Phase D vertical slice must include an explicit test: move `HEAD` without re-indexing the provider and confirm that Rationale degrades the `Assessment` to `stale`/`unknown` using only its own Git computation, without depending on any provider signal (see the Phase D verification plan).

**This ADR is `proposed`**, pending cross-review and human approval.

## Revisit trigger

Reopen if a future Codebase Memory version shows, with the same empirical methodology from `05-revision-and-coverage.md` repeated, that `detect_changes` stops failing in the same scenario — that would not invalidate the architecture (Git would remain the primary source for robustness), but it would allow reconsidering whether the provider's signal is worth using as an optional acceleration, never as a replacement.
