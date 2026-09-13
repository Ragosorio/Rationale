# Work item: manifest-action-not-convergent

**Status: `implemented — pending integration and pilot validation`.**
It blocked `v0.1.0-alpha.8`. It did not block the pilot migrations.

## Problem

`install-agent` reports that everything is up to date while it rewrites the
manifest.

On the second run over an already installed project, the report says
`instrucciones ya presentes y al día` ("instructions already present and up to
date") and `skill al día en …` ("skill up to date in …") for all nine entries —
and yet `.rationale-local/installed-agent-files.json` changes: every entry goes
from `"action": "created"` to `"action": "modified"`.

The cause is that `record_entry` / `record_owned_entry` are called with
`outcome.action`, which describes the file's state **in this run** (it existed →
`Modified`), not what happened when Rationale installed it. The first pass
creates the file and records `Created`; the second finds it existing and
overwrites that historical fact with `Modified`.

The report to the user and the effect on disk contradict each other.

## Goal

A second run of `install-agent` over an unchanged project must be
**byte-identical** in every file it touches, the manifest included.

The fix must explicitly do one of two things:

1. **Preserve the original semantics**: if an entry already exists for that
   path, keep its `action` instead of overwriting it — `action` means "what
   Rationale did the first time", and that does not change when reinstalling.
2. **Redefine what `action` means** and document it on the type itself, if it
   is decided that it should reflect the latest run.

Closing this while leaving the field's meaning ambiguous is not acceptable.

## Non-goals

- `ReversalStrategy` and the reversal logic do not change. `action` is already
  documented as something that "never decides how it is reverted"
  (`src/agents.rs`, comment in `uninstall`), and that is correct — which is why
  this defect is inert and does not corrupt uninstalls.
- ADR-0014's exclusion and Decision #9's migration are not touched.

## Base revision

`3830191` (`main`, after merging the pre-beta dogfood).

## Evidence

Observed in the real pilot repository Monorepo, 2026-07-28, during the
migration to the portable configuration:

- Pass 1: the six skills are recorded with `action: created`.
- Pass 2: the report says `skill al día` for all six, and yet the manifest
  changes — `created` → `modified` in all nine entries. The `md5` of
  `.mcp.json`, `CLAUDE.md`, and `AGENTS.md` is identical; the manifest's is not.
- Pass 3: **identical to pass 2**. It converges; it is not perpetual churn.

A bounded impact, which is why it did not block the Monorepo commit: the
affected file is local-only and, since ADR-0014, outside the index, so the
phantom diff no longer reaches Git in a migrated project. In a project that
still versions it, it does produce a diff with no real change.

## Risks

- **If not fixed:** the `install-agent` report stops being reliable as a
  description of what it did. It is the surface a user looks at to decide
  whether something changed, and it already lied once.
- **If fixed badly:** keeping the `action` of an earlier entry without
  validating that it corresponds to the same managed file could carry incorrect
  historical metadata forward. `resolve_managed_entry_path`'s validation already
  covers that case and must keep applying.

## Plan

1. Decide between options 1 and 2 of the Goal, and write the decision in the
   doc-comment of `FileAction`.
2. Implement it in `record_entry` / `record_owned_entry`.
3. Add the convergence test (see Tests).
4. Verify on a copy of a pilot that the second pass dirties nothing.

## Tests

A new test that **requires complete convergence on the second run**: run
`install` twice on a temporary repository and assert that the serialized
manifest is byte-identical between the two. It must fail with the current code —
if it does not fail, it is not testing this.

The existing idempotency tests (`instructions_block_is_idempotent`,
`mcp_json_migrates_a_legacy_absolute_command_to_the_logical_one`) do not cover
the manifest: that is why this defect reached a real pilot without anyone
seeing it.

## Docs

`CHANGELOG.md` under "Unreleased", if the fix redefines `action`.

## Success criterion

Running `install-agent` twice in a row over an unchanged project leaves the
working tree identical, and the convergence test guarantees it against
regression.

## Resolution (2026-07-28)

Implemented in `fix/manifest-action-convergence`. **There were two causes, not
one.** This work item's original diagnosis named only the first; the second
appeared when the tests required byte-identity, and it would not have been seen
by comparing fields one by one.

**Cause 1 — overwriting the historical `action`.** `record_entry` and
`record_owned_entry` received `outcome.action`, which describes the file's state
*in the current run* (it existed → `Modified`), and wrote it over the historical
fact. The first pass recorded `Created`; the second flipped it to `Modified`.

**Cause 2 — reordering entries through `remove` + `push`.** Preserving `action`
was not enough. On a second pass only *some* entries are re-recorded — skills
are always rewritten; instructions and MCP only when they changed — and with
`remove` + `push` those few jumped to the end, displacing the rest. `AGENTS.md`
moved from position 9 to position 3 without any data changing.

**Fix.** `action` is defined as a historical fact — how Rationale acquired
administration of the file the first time — and documented in the doc-comment of
`FileAction`, which was the requirement of not leaving the field ambiguous.
`upsert_entry` replaces `remove` + `push`: it updates the existing entry **in its
position**, and appends only what is new. The hash, `reversal`, and path
normalization are updated; `action` never is. The migration from absolute paths
also preserves it.

**Evidence.** Six new tests, written before the fix:

| Test | Covers |
|---|---|
| `fresh_install_manifest_is_byte_identical_on_the_second_run` | A fresh install converges |
| `legacy_manifest_migrates_once_then_converges` | An alpha.7 manifest: migrates once, then byte-identical |
| `a_historically_created_entry_stays_created` | `created` survives two reinstalls |
| `a_historically_modified_entry_stays_modified` | `modified` survives even if the file is deleted and the pass observes a creation |
| `uninstall_after_convergence_still_preserves_user_content` | Reversal still keeps the user's content |
| `convergence_does_not_weaken_the_arbitrary_path_guard` | Arbitrary paths are still rejected |

The first two **failed with the partial fix** (cause 1 only) and are the ones
that found the reordering. Full suite: 247 tests, Clippy and fmt clean.

Pending to close: integration into `main` and an on-disk convergence check on the
Monorepo pilot.
