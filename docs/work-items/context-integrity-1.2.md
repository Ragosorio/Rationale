# Context integrity (1.2): evidence, before and after

**Status:** implemented on branch `feature/context-integrity`; not released.
**Date:** 2026-10-14. **Owner of the decision to release:** the project's human owner.

## Why

A team using Rationale on a production service reported, from their own canon
and pull requests, what worked (93% of pull requests touching `src/` carried
Records; supersessions were clean; small files got reasons the code does not
show; retrieval was fast) and nine problems. This work treated them as
hypotheses, reproduced them where it could, and fixed what it reproduced. It
did not access that team's repository or data: every reproduction here is
synthetic or on Rationale's own repository, and says which.

## The reported problems, verified

| # | Reported | Verified here | How |
|---|---|---|---|
| 1 | `explain_target` presents superseded Records as current (6 of 27) | **Confirmed** in code and on the synthetic project | `explain` filtered nothing; retrieval missed one-sided supersessions |
| 2 | A symbol query returns the whole file; a folder query nothing useful | **Confirmed** on both, and live on this repository (`src/pipeline.rs::explain` received a decision about Codebase Memory project handles) | Companion file bindings; no folder semantics |
| 3 | `prepare_change` exceeds its budget (median ≈ 3,800) | **Confirmed** on this repository: estimated median 3,068, 10 of 12 targets over 2,400 | Budget over the packet only, with bytes/4 |
| 4 | 20% of operations never closed | **Mechanism confirmed**: no listing, no count, pruning deleted open ones, no way to close without capture. The 20% rate itself was not reproduced | Code reading; operations suite |
| 5 | A stale index could not bind symbols created on the same branch | **Confirmed** on the synthetic project; plus a related live finding: the index served nodes from files of another checkout | Capture fell back to file bindings |
| 6 | Constraints from incidents do not reach modules sharing the invariant | **Confirmed** on the synthetic project | No dependency or folder scope |
| 7 | 116 active Records for a retired module | **Mechanism confirmed** at scale 12 on the synthetic project; doctor reported each path separately and nothing could repair them | Synthetic `src/comunicaciones` |
| 8 | A critical secret-rotation exception left orphaned | **Confirmed** on the synthetic project: nothing linked an exception to its rule | Synthetic `exception.legacy-kms-skips-rotation` |
| 9 | No pinned Records nor rules stated by a person | **Confirmed in kind** on Rationale's own canon (40 active Records, none pinned or human-stated), and no command let a person state a rule | `doctor` observation |

Antecedents checked:

| Antecedent | Finding |
|---|---|
| `install-agent --help` ran installation actions | **Already fixed** (pull request #5, July). Verified in a sandbox for 13 help invocations on v1.1.0 and this branch; the help test now derives its command list from `--help` so a new command cannot regress it |
| Migration to a native structural provider | **Not in the repository.** It was built in 21 local commits (ADR-0018, `src/providers/native/`) between 2026-09-13 and 09-15, never pushed; the checkout no longer exists. Codebase Memory's index of this path still lists those files. See §Lost work |
| Documented commands versus binaries | **Drift found and closed:** `install-agent --refresh-skills` was accepted but missing from its help. `scripts/check-doc-commands.py` now fails CI on any documented command or flag the binary lacks |
| Partial validations presented as a release | This work separates local tests, CI, live client runs, a packaged binary, and a published artifact below, and claims only what ran |

## Targets set after the baseline, before optimizing

Baseline committed in `75dd279` (`eval/results/baseline-v1.1.0/`). Targets:

- Superseded or revoked Records served as authority: 0 (baseline 6–7).
- Records of another symbol served as authority: 0 (baseline 3).
- Required Records missing: 0 (baseline 10). Irrelevant Records: ≤ 2 (baseline 8–11).
- Synthetic retrieval scenarios passing: all.
- Every response within its budget, or a declared overflow with only
  governing knowledge left.
- Warm `prepare_change` latency: no regression beyond 10% on the real
  repository, measured interleaved.
- Capture of a new symbol, open operations, doctor (4 of 4), retirement with
  byte-exact undo, and two-way compatibility with v1.1.0: pass.

## Results

Synthetic project (`eval/results/v1.2.0-dev/report.md`, `report.json`):

| Metric | v1.1.0 | this branch |
|---|---|---|
| Retrieval scenarios passed | 1 / 15 | 15 / 15 |
| Superseded or revoked Records served as authority | 7 | 0 |
| Records of another symbol served as authority | 3 | 0 |
| Required Records missing | 10 | 0 |
| Irrelevant Records served | 11 | 0 |
| Response tokens p50 / p95 / max (upper-bound estimate) | 1,038 / 2,213 / 2,427 | 779 / 1,626 / 1,743 |
| New-symbol capture governs only its symbol | no | yes |
| Open operations visible and abandonable | no | yes |
| Doctor detects (of 4) | 1 | 4 |
| Retired module repaired reversibly | no | yes |
| Canon written by each binary read by the other | — | yes |

Rationale's own repository with the real Codebase Memory, 12 targets, fresh
index (`real-repo.json`):

| Metric | v1.1.0 | this branch |
|---|---|---|
| Estimated response tokens p50 / p95 / max | 3,068 / 3,973 / 4,731 | 2,370 / 2,455 / 2,456 |
| Targets over the 2,400 default | 10 | 2 (both declared overflows of governing knowledge) |
| `src/providers` (folder) Records | 0 | 3 |
| `src/agents.rs::install` Records | 5 (file-wide) | 1 |

Latency, interleaved A/B on the same repository and provider
(`ab-latency.json`, 72 calls each): `prepare_change` p50 109.0 → 102.5 ms, p95
136.7 → 128.5 ms. Sequential runs on the same laptop varied by more than that
under load (load averages from 3 to 30 during this work); only interleaved
numbers are compared.

Scale, synthetic canon with extra Records (single runs, p50; the machine was
loaded):

| Extra Records | prepare (symbol) | explain | prepare (folder) | pre-edit hook | session-start hook |
|---|---|---|---|---|---|
| 1,000, before the perf commit | 186 ms | 26 ms | 166 ms | 30 ms | 57 ms |
| 5,000, before the perf commit | 549 ms | 117 ms | 1,644 ms | 129 ms | 257 ms |
| 5,000, after | 68 ms | 26 ms | 97 ms | 154 ms | 190 ms |

The hooks are short-lived processes and do not benefit from the in-process
parse cache.

## What changed

See `CHANGELOG.md` (Unreleased) and ADR-0019 and ADR-0020. In short: one
authority view with history; symbol, file, and folder scope; lexical binding
for symbols the index does not know; stale structure dropped; inherited
constraints along observed edges; a budget over the whole response with
`omitted`; open/finalized/abandoned operations; doctor severities and a CI
baseline; `retire`; `rule add`; Claude Code hooks; opt-in execution advice;
and the evaluation itself, gated in CI.

## Compatibility

- Canons written by v1.1.0 are read unchanged (the synthetic canon is in the
  v1.1 format, and the compatibility suite writes with each binary and reads
  with the other). No migration is required.
- v1.1.0 reading a canon written by this branch: lexical symbol bindings act as
  file bindings, folder bindings do not match, retirements are revocations,
  `exception_to` and `doctor-baseline.json` are ignored and preserved.
- MCP responses changed shape compatibly but are stricter: `diagnostics` and
  the per-binding assessment need `verbose`; `intent_conflicts[].statement` is
  omitted; `primary_reason` is null when it repeats an entry's rationale.
  Agents following the updated skill read the new fields.
- Downgrading a project with Claude Code hooks to a binary without `hook`
  needs `uninstall-agent` first.
- The installed v1.1.0 on this machine was never modified: every test used
  copies, sandboxes, and an isolated HOME. Its user-level registrations still
  point at `/opt/homebrew/bin/rationale`.

## How it was verified

| Kind | What ran | Result |
|---|---|---|
| Local tests | `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` (426 tests, macOS; 386 at v1.1.0) | pass |
| Evaluation | `eval/run_eval.py` against v1.1.0 and this branch; `eval/real_repo.py`; `eval/ab_latency.py` | results above |
| Documentation | `scripts/check-docs.sh` with `check-doc-commands.py` (286 documented invocations) | pass |
| CI | GitHub Actions on the pull request (Linux, macOS, Windows; site; ui) | see the pull request |
| Real client | Claude Code 2.1.296 headless: SessionStart hook output accepted, MCP connected | partial — the CLI's login had expired, so no model turn ran |
| Packaged binary | `scripts/package.sh` artifact installed by the shell installer from a local mirror over an existing v1.1.0 install and project, then install-agent, uninstall-agent, uninstall (`install-lifecycle.txt`) | pass |
| Browser | the website's interactive demo, in a local build | see the site section of the pull request |
| Published artifact on a clean machine | **not done**: publishing a Release needs authorization, and no new macOS user was created | pending |

## Lost work

Claude Code session logs for this repository show 21 commits made between
2026-09-13 and 2026-09-15 and never pushed: a native structural provider
(ADR-0018, `src/providers/native/`, tree-sitter extraction and type-aware Rust
and Python resolution) and a rewrite of the skill with an `audit` operation.
The last session ended at its usage limit before the push it was asked to do,
and the checkout at `~/Desktop/Rationale` no longer existed on 2026-10-14. This
work started from `origin/main` (v1.1.0). If that checkout is recovered (for
example from the Trash or a backup), its commits can be rebased onto this
branch; ADR-0018 is kept free for it.

## Pending, and what needs authorization

- **Merge** the pull request to `main` — the owner's decision.
- **Release** 1.2.0 (tag, Release, installers, `docs/RELEASE_VERSION`, site
  status text) — needs explicit authorization.
- **Clean-machine verification** of the published artifact with a new macOS
  user — after a Release.
- **Live Claude Code run** of the PreToolUse, PostToolUse, and Stop hooks — needs
  `claude` logged in on this machine.
- **Token estimator calibration** against `count_tokens` — needs an API key.
- **The reporting team's canon**: run `rationale doctor` and
  `rationale retire src/<communications module> --reason …` (plan only) on
  their repository to size the real repair — needs their authorization.
- **Rationale's own canon**: none of its 40 active Records is pinned or
  human-stated, and `decision.landing-beta-release-status` still describes a
  public beta. Pinning and revising them are the owner's decisions.
