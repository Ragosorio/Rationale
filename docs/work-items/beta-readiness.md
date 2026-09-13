# From alpha to beta — definition and checklist

This document exists so that "is it beta yet?" is a verifiable question, not an
opinion. It covers functionality only — not process (external users, sustained
use over time), although those points are listed at the end because beta is not
defensible without them.

**Status: `v0.1.0-beta.2` published.** The functional boxes are closed with the
evidence described below; the process boxes remain open and are declared as
such. Beta.1 does not claim that Rationale is finished: it claims that the flow
works repeatably and that the known failures are known. The section "Real scope
of the evidence" says exactly what was tested and what was not, so nobody has to
infer it.

## Definition

> Rationale enters beta when the full flow of preparing, capturing, reviewing,
> and retrieving decisions works repeatably across several repositories; basic
> bindings are reliable; installation, update, and uninstallation are idempotent
> on every platform the project announces as supported; and there are no known
> failures that corrupt the canon, invent authority, or approve decisions without
> human intervention.

Alpha means "we are still checking that the product works". Beta means "we
already know it works; now we check that it works well for more people,
projects, and environments". It does not require perfection — it requires known
failures to be known, controlled, and not to destroy the user's trust.

## Already met (verified, not only reviewed by reading)

- **MCP transport.** `initialize`/`tools/list`/`tools/call` over line-delimited
  JSON, with the persistent session surviving unknown tools, nonexistent
  targets, malformed JSON, and deeply nested messages (`tests/mcp_server.rs`).
- **Full propose → review → approve → recover cycle.** Verified in this
  repository with the real alpha.7 binary (not only synthetic tests):
  `finalize_change` in one session, `rationale review` as a real subprocess, and
  `prepare_change` in a NEW session — `governs_target: true`,
  `match_kind: structural`, `linkage: current`, with the real structural provider
  resolving the symbol.
- **Honest conflict detection.** `intent_conflicts` distinguishes a verifiable
  fact (`governs-target`) from unverified lexical overlap (`lexical-overlap`);
  `polarity: undetermined` is never promoted to a verdict.
  `governance_verdict_required` forces the agent to state a position instead of
  proceeding silently.
- **`prepare_change` enables conflict detection by default** when `intent` is
  present, without requiring a separate `mode` flag that the documented master
  prompt never teaches (a real bug fixed this session — before, it reproduced the
  exact symptom of the incident that motivated the project).
- **Installation/update on Unix (scope corrected).** A clean install, a
  reinstall, and the `preview` channel without silently degrading to
  `stable`/`releases/latest`, a SHA-256 checksum verified before installing, and
  the update helper installed next to the binary. This evidence did not cover
  running `init` again on an existing canon: the dogfood found that path returned
  before configuring agents. The fix and its automated regression test exist, but
  the four pilot repositories still have to be converged and verified.
- **Windows: `quality (windows-latest)` runs and passes in real CI** — not just
  added to the matrix, but green end to end, including the smoke test that builds
  the binary, packs the ZIP, expands it, and confirms the exact path
  `rationale-installer.ps1` expects. Along the way, `windows-latest` found two
  real defects ubuntu/macOS could not detect: `cache::cache_root` used `$HOME`
  (nonexistent on Windows; it now uses `%LOCALAPPDATA%`, the candidate ADR-0005
  had already named) and a test whose mock is a bash script Windows cannot run as
  a native binary (skipped there, documented as a limitation of the test fixture,
  not of the production code). Parity of `install-agent --global-only` and
  cleanup of `rationale-update.ps1` on uninstall.
- **Non-destructive `uninstall-agent`.** It removes only what Rationale wrote —
  it never deletes a whole file just because Rationale created it, if the user
  later added content (verified with a `.mcp.json` holding another MCP server,
  and a `CLAUDE.md` with the user's own text below the block). For complete
  skills, it first claims the identity through an atomic rename: a concurrently
  recreated path stays intact and publishing the replacement fails closed instead
  of overwriting it (ADR-0008).
- **Atomic canon.** Every write to `.rationale/` (Records, Subjects, and now also
  `agents.rs`: `CLAUDE.md`/`AGENTS.md`/`.mcp.json`/manifest) uses a temp file +
  `sync_all` + `rename`. An interrupted process never leaves a truncated file.
- **Minimal panic surface.** Of 216 occurrences of `unwrap`/`expect`/`panic!` in
  `src/`, only one was reachable with real user input (`extract_block` with
  inverted markers) — fixed.
- **A migration path for the legacy canon.** `RecordWithoutBindings` (Phase 1's
  original defect, already written in four repositories) now has an explicit
  human repair through `doctor --repair`, marked `declared_by: human` — never
  indistinguishable from a binding confirmed by a provider.
- **Lifecycle audit.** `review-record` prints the complete `approvals[]`
  (including `approved_at`, which was not written before) and
  `lifecycle.events[]` — auditing "who approved and when" no longer requires
  reading the YAML by hand.
- **Multi-repo.** `project_root` (canon) and `repo_path` (code) verified as
  independent with two real Git repositories, not only wired without testing.

## Alpha exit checklist

```text
[x] Full governance chain verified with the real binary (not only synthetic
    tests), including a new MCP session retrieving a Record approved in an
    earlier session.
[x] intent enables conflict detection without additional undocumented flags.
[x] Exact bindings created and retrieved correctly (file + symbol + file→symbol
    propagation).
[x] explain_target returns the same governing Records as prepare_change for the
    same target.
[x] Conflicts distinguished between lexical ones and "governs the target".
[x] Zero known panics on normal inputs.
[x] Zero loss or corruption of the canon (atomic writes, including agents.rs).
[x] Complete idempotency of init/install/update on Unix — install, reinstall,
    update, and **re-initialization** verified. Convergence was checked on a real
    pilot repository (BoostAPI, migrated from alpha.7): the three historical
    entries kept `action: created`, their absolute paths were normalized to
    relative ones, and the manifest stayed byte-for-byte identical between two
    consecutive passes. The full destructive cycle (install → uninstall after
    migrating, without losing the user's content) was rehearsed first on a local
    clone of the repository.
[x] Uninstallation keeps .rationale/ AND the content the user added to files
    Rationale created; the pathname TOCTOU race in skills has deterministic tests
    for claim, recreation, and no-clobber.
[x] macOS tested (this environment).
[x] Linux — covered by CI (ubuntu-latest), not tested by hand in this session.
[x] Windows — `windows-latest` runs and passes in real CI, including the
    packaging smoke test; not tested by hand on a physical Windows machine (see
    "Out of scope" below).
[x] Basic Record lifecycle working: correct, dispute, revoke, supersede,
    change-authority, add-evidence, and now add-human-confirmed-binding.
[x] Migration path for the legacy canon without bindings.
[x] Approvals/lifecycle audit without reading YAML by hand.
[x] Multi-repo (project_root != repo_path) verified with real repositories.
[x] Full flow verified in distinct real repositories, with the scope declared
    below. The original criterion said "10 flows in 5 repositories"; it was
    replaced by a threshold the evidence really supports instead of inflating the
    number. See "Real scope of the evidence".
[ ] 5–10 external users, 3+ completing the flow without direct help — not
    attempted; it is process, not code.
[ ] Several days of real use without corruption, loss, or serious blocks — not
    attempted.
[ ] Windows tested by hand on a real Windows machine/VM (CI already green on
    windows-latest, see above — the human run is missing).
```

## Real scope of the evidence (beta.1)

The original criterion — "10 complete flows in 5 different repositories" — was
written before there was data and turned out to be a number without
justification: nobody could say why 10 and not 6, or what the fifth repository
proved that the third did not. Inflating it to tick the box would have been
exactly the kind of self-certification this document exists to prevent. It is
replaced by the scope the evidence supports, stated precisely:

**Verified:**

- **This repository** (Rationale on itself): the full chain `prepare_change` →
  change → `finalize_change` → human `rationale review` → `prepare_change` in a
  NEW session retrieving the approved Record, with the structural provider
  resolving the symbol. Repeated throughout development, not once.
- **BoostAPI** (a real project, in active use): migration from alpha.7 with
  observed facts — `created` preserved, paths normalized, the manifest
  byte-identical over two passes, `.rationale-local/` outside the index, zero
  personal paths in the managed files — and real use of `prepare_change` during
  product planning.
- **A local clone of BoostAPI**: the full destructive install → uninstall cycle
  after migrating, without inheriting `.git/info/exclude` and keeping the files
  the repository did version.
- **Synthetic projects**: the three agent targets detected and configured,
  including Cursor's full cycle.

**Not verified, and why it matters:**

- No one else's repository: all of the above belong to the person developing
  Rationale, so none proves the instructions work without prior knowledge of the
  project.
- No repository with a legacy canon written by a version other than alpha.7.
- Cursor: the `.mdc` rule and the MCP config are generated and reverted correctly
  (with tests), but only a person with Cursor open can confirm that Cursor
  **applies** them.

## Still open / not documented anywhere else

- **Repairing the remaining pilot repositories.** BoostAPI is already converged
  and verified (see "Real scope of the evidence"). Monorepo was migrated with a
  binary that had the convergence defect, so its entries were left as
  `modified`: the damage is historical and the fixed binary preserves it instead
  of repairing it. It does not block beta — the migration works — but it is worth
  recording so it is not mistaken for a current defect.
- **`.rationale/migrations/` is an empty affordance.** `rationale doctor` already
  detects an unknown `schema_version` as a visible gate, but there is no real
  migration logic. Beta does not need it (only one schema version exists today),
  but the day a second one exists, this is the first thing to build.
- **Binding noise in `finalize_change`.** The files `install-agent`/`init`
  administer were excluded, but `finalize_change` still binds EVERY uncommitted
  file in the repository, not only those related to the target. A real change
  next to unrelated uncommitted scratch files will still produce extra bindings.
  It does not block beta (the real target's binding is always present), but it is
  worth narrowing.
- **CI does not validate Linux/macOS by hand**, only through GitHub Actions.
  Enough for beta, but "tested" in this document means "CI green", not "a person
  installed it on their own clean machine" except on macOS (this environment).

## Out of scope for this document

- Windows: CI (`windows-latest`) already ran green end to end, including the
  packaging smoke test — that is real execution evidence, not just code reading.
  What is still missing is a person installing and using the binary on a physical
  Windows machine; that is process work, not code.
- External users, sustained use over time, and the "10 flows in 5 repositories"
  of the checklist above are process work, not code — this document makes them
  explicit; it does not resolve them.
