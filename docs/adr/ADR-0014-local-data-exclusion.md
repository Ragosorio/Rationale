# ADR-0014: Local data exclusion in consumer projects

**Status:** proposed — pending independent cross-review and human approval before `accepted`.
**Date:** 2026-07-28
**Deciders:** Claude Code (analysis and implementation); pending human approval and/or cross-review by another agent
**Supersedes / Superseded by:** proposes replacing ADR-0012's local-exclusion guarantee. While both remain `proposed`, this ADR does **not** formally supersede that one — an unapproved proposal gains no authority over another proposal.

## Context

ADR-0012 established that all of Rationale's telemetry is local-only, and
presented as evidence that `.rationale-local/` "has been in `.gitignore` since
Phase A". That check was made inside Rationale's repository, where that
`.gitignore` is written by hand, and it was generalized to consumer projects
without verification.

The migration from `alpha.7` to `main` on copies of Monorepo and BoostAPI showed
that the generalization was false (`ADR-0012 §Validation update — 2026-07-28`).
In both pilots, three files are versioned and present on `origin/main`:

```
.rationale-local/installed-agent-files.json
.rationale-local/runs/review-decisions.ndjson
.rationale-local/runs/vertical-slice.ndjson
```

Neither `init` nor `install-agent` ever writes an exclusion into the user's
project. Two of two pilots reproduced the defect: it is the product's normal
flow, not one repository's accident.

This ADR decides how Rationale protects its local data in someone else's
repository. **The resolution of the executable in `.mcp.json` is explicitly out
of scope** and is decided in ADR-0015: it is a different trade-off (`PATH`
inheritance in MCP clients launched as graphical apps), and mixing them would
block this ADR on an unresolved problem.

## Decision

1. **`.rationale-local/` is strictly local data and must never be versioned in
   a consumer project.** It includes the NDJSON telemetry in `runs/` and the
   `installed-agent-files.json` manifest, which stores absolute paths under the
   user's `$HOME`.

2. **The exclusion is installed in `.git/info/exclude`, not in `.gitignore`.**
   `.gitignore` is a shared, versioned file of someone else's project; writing
   there is a visible modification of the repository that Rationale has no
   reason to impose on the team. `info/exclude` is local to the clone, achieves
   the same effect, and every person who runs `init` or `install-agent` gets the
   exclusion in their own clone.

3. **The exclusion is written *before* creating any content under
   `.rationale-local/`**, idempotently: if the entry already exists, it is
   neither duplicated nor is the file rewritten.

4. **The gitdir is resolved for real, not assumed to be `<root>/.git/info/exclude`.**
   In submodules and worktrees, `.git` is a *file* with a `gitdir:` pointer, not
   a directory. The correct path comes from resolving that pointer (or through
   `git rev-parse --git-common-dir`, which in a worktree points to the shared
   common directory — where `info/exclude` must live to apply to every
   worktree).

5. **Outside a Git repository, Rationale neither fails nor blocks.** Without a
   gitdir there is nothing to exclude: the step is skipped silently and the rest
   of the installation proceeds as usual.

6. **Rationale detects whether `.rationale-local/` is already tracked by Git and
   warns, but never touches the index.** Modifying the versioned state of
   someone else's project is not the installer's decision. The warning names the
   exact command and leaves running it to the human:

   ```
   notice: .rationale-local/ contains files tracked by Git.
   Rationale will not modify the index automatically.
   To stop versioning them:
       git rm -r --cached .rationale-local
   ```

7. **The manifest may contain only project-relative paths.** Today it stores
   absolute ones (`/Users/<whoever>/Desktop/BoostAPI/CLAUDE.md`). Even if the
   exclusion prevents it from being versioned, an absolute path in a file that
   has already traveled twice is data that did not need to exist. A relative path
   is enough for the only thing the manifest does: know which files it
   administers and whether they are still intact.

8. **Validation is a regression test, not a manual inspection.** A test creates
   a temporary Git repository, runs `install-agent`, and fails if
   `git status --porcelain` reports any path under `.rationale-local/` as
   tracked or not ignored. Manual inspection was exactly what failed in
   ADR-0012.

9. **`install-agent` normalizes legacy manifest entries whose destination is
   recognizable.** It is not enough for *new* entries to be relative: an
   `alpha.7` project that was moved or copied keeps absolute paths pointing at
   the earlier location, the guard rejects them — correctly — and the rejection
   aborts `uninstall-agent` entirely.

   Normalization **does not relax the guard**. It rewrites an entry only when
   its path ends in a known managed destination (`CLAUDE.md`, `AGENTS.md`,
   `.mcp.json`, the Cursor rule, or a `SKILL.md` under the skills directory),
   derived from `TARGETS` and `prompts::ACTIONS` as the single source. An
   arbitrary path — `~/Documents/notes.md` — matches none of them, is kept
   intact, and the guard still rejects it. The resulting destination always
   stays inside the `project_root` because it is relative.

   This is what makes `install-agent` truly the migration path: it repairs the
   administrative state, not only the blocks. Without it, a moved pilot would be
   permanently unable to uninstall.

   **Declared blast radius:** if the legacy manifest pointed at *another*
   project's `CLAUDE.md`, after normalizing it points at this one's. It is not a
   privilege escalation: `uninstall` only removes Rationale's delimited block,
   and if this project is installed that file already had its own entry. It is
   stated rather than left implicit.

   It remains **outside** this decision that an unrecognizable entry still aborts
   the whole operation instead of being skipped. With normalization, that case
   is no longer produced by moving a project and instead signals a corrupted or
   tampered manifest, where failing loudly is defensible.

## Evidence

- **Reproduction in two real pilots**, on copies, without touching the
  originals: `install-agent` run twice on Monorepo and BoostAPI. Both started
  from a `.rationale-local/` already versioned with the same three files, and
  both left it modified in the working tree.
- **Real exposure, not potential**: `git branch -r --contains` places the
  commits that introduced those files on `origin/main` in both repositories
  (`812f7fe`, `b78357d` in Monorepo; `0346b91`, `bcfc9fe` in BoostAPI).
- **Neither project had a `rationale` entry in its `.gitignore`** — confirmed
  by direct inspection. Nothing in the product writes it.
- **The leaked content includes behavioral data**: `review-decisions.ndjson`
  records `time_to_confirm_ms` per Record (up to ~300 s) and the decision taken.
  `installed-agent-files.json` records absolute paths under `$HOME`.
- **The migration itself is correct and not in question**: the two passes of
  `install-agent` left `CLAUDE.md`, `AGENTS.md`, and `.mcp.json` byte-identical
  to each other, with a single managed block and no deletions. The defect is
  exclusively about excluding local data.

## Alternatives considered

- **Writing the entry into the project's `.gitignore`.** Discarded: it is a
  visible, versioned modification of a file the team shares, to solve a problem
  local to each clone. Rationale would impose a repository change to protect its
  own artifacts. `info/exclude` gets the same result without touching anything
  shared. It remains an option if a case appears where the exclusion must reach
  someone who never runs Rationale — none exists today: without running
  Rationale there is no `.rationale-local/` to exclude.
- **Running `git rm -r --cached .rationale-local` automatically when tracked
  files are detected.** Discarded: it changes the versioned state of someone
  else's project without consent, and in a product whose thesis is "do not tear
  down a fence without knowing why it is there", doing it silently would be
  contradictory. It warns and hands over the command.
- **Writing nothing under `.rationale-local/` until an exclusion exists.**
  Discarded as a general policy: it would turn a hygiene problem into a
  functional block, and ADR-0012 §Decision already establishes that
  instrumentation is not postponed. This ADR's Decision #3 (exclude *before*
  writing) achieves the effect without blocking anything.
- **Stopping the `review_decision` emission.** Discarded: it is legitimate data
  for the metrics in `v0.5 §30`. The problem was not that it was generated but
  that it was published — and that it never passed ADR-0012's allowed-fields
  filter. Applying that filter to every emitter, not only `RunLog`, is work for
  the review of ADR-0012, not for this ADR.

## Consequences

- Any new emitter of data under `.rationale-local/` inherits the protection
  without an additional decision: the exclusion covers the directory, not
  specific files.
- The pilot repositories already affected **do not fix themselves**. They
  require manual remediation with `git rm -r --cached .rationale-local`, once
  per repository. The historical data remains in earlier commits; see ADR-0012
  §Validation update for why history is not rewritten.
- Rationale writes inside `.git/`, which it did not do before. It is limited to
  `info/exclude`, the only write allowed there.
- The manifest changes format (relative paths). `uninstall-agent` reads that
  file: it must tolerate old manifests with absolute paths, or the pilots already
  installed would lose the ability to uninstall cleanly.

## Risks

- **`info/exclude` does not propagate.** Someone who clones and never runs
  `init` or `install-agent` will not have the exclusion. Mitigation: they will
  not have `.rationale-local/` either, because only Rationale creates it. The
  risk is nil in practice and becomes real only if someone copies the directory
  by hand — the same case ADR-0012 §Risks already considers.
- **An unexpected `.git`.** Repositories with `core.worktree`, nested
  submodules, or non-standard setups could resolve a gitdir other than the
  expected one. Mitigation: if resolution does not produce a writable directory,
  the step is skipped with a warning; installation never fails.
- **The warning is ignored.** The human may not run `git rm --cached` and keep
  the tracked files indefinitely. Accepted mitigation: the alternative is acting
  on someone else's index, which is worse. The warning repeats on every run while
  the condition persists.

- **A manifest with absolute paths outside the project aborts `uninstall-agent`
  entirely.** Observed while verifying this ADR on a copy of BoostAPI: if the
  project was moved or copied after installing, the manifest's absolute entries
  point at the earlier location, `resolve_managed_entry_path` rejects them —
  correctly; it is the guard that prevents a tampered manifest from touching
  arbitrary files — and the rejection **cancels the whole uninstall**, legitimate
  entries included.

  It is a **pre-existing** defect, not introduced by this ADR: the guard and its
  abort behavior predate it. This ADR reduces it going forward (a relative path
  survives moving the project) but does not remove it for manifests already
  written. It is documented here instead of being taken as covered: this ADR's
  Consequence claims `uninstall-agent` compatibility with legacy manifests, and
  that claim holds only while the project has not been moved. Accepting a
  compatibility claim without bounding it would be exactly the mistake ADR-0012
  made.

  **Resolved by Decision #9**, added after detecting this: instead of touching
  the guard, `install-agent` normalizes legacy entries whose destination is
  recognizable, so the "moved project" case no longer produces external entries.
  Verified end to end on a copy of BoostAPI with the manifest pointing at the
  original location.

  What was deliberately **not** resolved: an unrecognizable entry still aborts
  the whole operation. After Decision #9 that case is no longer produced by a
  moved project but by a corrupted or tampered manifest, where failing loudly is
  the defensible response. If a real case of a legitimate unrecognizable entry
  appears, it is reopened.

## Validation

Implemented and verified. Regression tests in `src/agents.rs`, all on real
temporary Git repositories:

1. `install_leaves_no_local_data_visible_to_git` — fails if `git status
   --porcelain --untracked-files=all` reports any path under
   `.rationale-local/` after installing (Decision #8).
2. `exclude_entry_is_idempotent_and_preserves_existing_rules` — a second pass
   without rewriting, the entry not duplicated, the user's earlier rules intact.
3. `exclude_resolves_the_real_gitdir_in_a_worktree` — a real worktree, `.git`
   as a file with a pointer, the exclusion landing in the common directory.
4. `install_warns_when_local_data_is_already_tracked` — a warning with the exact
   command, and a check that the index was **not** modified.
5. `install_outside_a_git_repository_does_not_fail` (Decision #5).
6. `manifest_stores_project_relative_paths` — no absolute path.
7. `uninstall_still_reads_a_legacy_absolute_path_manifest` — backward
   compatibility.
8. `install_migrates_a_moved_projects_legacy_manifest_and_uninstall_then_works`
   — the full scenario of Decision #9: `uninstall` fails before migrating,
   `install-agent` normalizes, `uninstall` completes.
9. `migration_never_normalizes_an_arbitrary_path` — `~/Documents/notas.md` is
   kept intact and the guard still rejects it.
10. `migration_recognizes_every_managed_destination` — walks `TARGETS` and
    `prompts::ACTIONS` so a new agent or action cannot be left out of the
    normalization without anyone noticing.

**A deliberate deviation from the original validation plan**, which asked for
static fixtures in `tests/fixtures/alpha7-consumer/`: they were discarded in
favor of building the repositories with `git init` inside each test. A static
fixture cannot carry a real versioned `.git/` inside this repository, so it
could not reproduce the only thing that matters here — which files are *tracked
by the index* — which is precisely the condition that failed in the pilots. The
programmatic tests cover that state; the fixtures could not.

Additional end-to-end verification, outside the suite: `install-agent` run twice
on fresh copies of Monorepo and BoostAPI. Blocks byte-identical between passes,
the exclusion not duplicated, zero absolute paths in `CLAUDE.md` and
`AGENTS.md`, and the warning emitted in both for the three already-tracked
files.

**Explicitly, validation cannot consist of manually inspecting Rationale's
repository.** That was ADR-0012's mistake: verifying in the development
repository and generalizing to consumers.

## Revisit trigger

Reopen if a consumer appears where `info/exclude` is not enough — for example, a
flow where `.rationale-local/` must be shared deliberately among team members
(joint debugging, decision audits). That would require an explicit decision about
which fields are publishable, not a silent relaxation of this exclusion.
