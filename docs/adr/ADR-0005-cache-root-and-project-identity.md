# ADR-0005: Cache root and project identity

**Status:** proposed — pending independent cross-review before `accepted`.
**Date:** 2026-07-25
**Deciders:** Claude Code (analysis and implementation); pending human approval and/or cross-review by another agent
**Supersedes / Superseded by:** none

## Context

ADR-0004 decides that the derived layer lives in SQLite. It remains to decide **where** on the filesystem it lives and **how** it is named per project. `Rationale_Arquitectura_Conceptual_v0.1.md §10.2` proposes `<user-cache>/rationale/projects/<project-id>/`, with the exact path "pending a decision during implementation", and suggests `~/Library/Caches/Rationale/` on macOS or "a configurable root compatible with XDG".

`src/configuration.rs` already resolves a `project_id` (from `config.yaml` or, by default, the directory name), but today it is only used to display it in `health` — no real cache path exists yet to name.

## Decision

1. **Cache root:** `~/.cache/rationale/projects/<sanitized-project-id>/` on macOS and Linux — not the native macOS path (`~/Library/Caches/`). Windows remains a documented, non-blocking gap (see Risks).
2. **Sanitizing the project name:** the absolute path of the `project_root`, with separators replaced by hyphens — the same scheme observed in Codebase Memory.
3. **The logical `project_id`** (the value used inside Records and logs, not the cache folder name) remains the mechanism already implemented: `config.yaml → project.id`, falling back to the directory name. It does not change with this ADR.

## Evidence

- **A real precedent, not only a theoretical one:** Codebase Memory (the same tool domain: local code analysis, derived cache) uses exactly `~/.cache/codebase-memory-mcp/<sanitized-path>.db`, confirmed by direct file inspection in `docs/research/codebase-memory/07-storage-and-cache.md` — not a native macOS path. Seven real projects indexed on that machine confirm the naming pattern (`Users-roor.osorio-Desktop-Monorepo.db`, etc.).
- Using `~/.cache/` uniformly (instead of `~/Library/Caches/` on macOS versus `~/.cache/` on Linux) avoids adding a new dependency (`dirs` or another per-platform path-resolution crate) just for this decision — consistent with the principle of minimizing dependencies (`Proceso §19`).
- The canonical/derived separation (`v0.5 §4.19`, Subject `storage.canonical-vs-derived`) means **correctness never depends on this path** — losing or moving the cache only triggers a rebuild, never the loss of a real decision. That reduces the risk of any imperfect path choice.

## Alternatives considered

- **`~/Library/Caches/Rationale/` on macOS, XDG on Linux (native per-platform paths)**: discarded for now — it would require the `dirs` (or `directories`) crate without a concrete reason yet beyond "follow the OS convention". It will be reconsidered if Phase J (packaging) finds a real integration requirement with system tools (Finder, Spotlight indexing, etc.) that depends on the native location.
- **A cache inside the repository (`.rationale-local/` already exists for this purpose)**: discarded for the SQLite layer specifically — `.rationale-local/` is already used for ephemeral run logs (Phase D), but putting a larger SQLite index there contradicts the folder's original intent and complicates selective `.gitignore`. It stays separate.
- **A project ID based on a hash of the Git remote or the root commit** (more stable than a directory name under renames or moves): evaluated, not discarded — it is a real pending improvement, but it does not block this ADR because correctness does not depend on the stability of `project_id` (only its readability in logs). It is noted as a future improvement, not as a decision of this document.

## Consequences

- A new module or an extension of `configuration.rs` computes the cache path: `cache_root(project_root) -> PathBuf`.
- If the user moves or renames the project folder, the derived cache under the old path is orphaned (this ADR never deletes it automatically) — acceptable because it is 100% regenerable, but it leaves garbage on disk in the long run. A cleanup policy for orphaned caches is out of scope for Phase E.
- Windows will use a different path when implemented (`%LOCALAPPDATA%\rationale\projects\...` is the natural candidate, not yet verified) — it does not block Phase E, which is developed and validated on macOS.

## Risks

- **An explicit Windows gap**: this ADR does not solve the cache path on Windows. The same risk pattern is already recorded for file locking (`docs/dependencies/inventory.yaml known_gaps`) — it is added here as a second gap of the same nature, to be resolved together with Phase J (packaging), not before.
- A name collision if two different projects sanitize to the same path (extremely unlikely with full absolute paths, but not mathematically impossible with symbolic links) — mitigation deferred; the same risk Codebase Memory already accepts implicitly, with no incidents reported in the Phase B research.

## Validation

Validated in Phase E3 with a test that computes the cache path for Rationale's own repository and for the vertical-slice fixture, confirming that they do not collide and that both are rebuildable after deleting the cache directory.

**This ADR is `proposed`**, pending cross-review and human approval.

## Revisit trigger

Reopen when Phase J (packaging) needs to solve Windows for real, or if a concrete case appears where the stability of `project_id` under folder renames causes a measurable (not only theoretical) problem.
