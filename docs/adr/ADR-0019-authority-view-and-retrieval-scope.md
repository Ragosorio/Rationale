# ADR-0019: Authority view, retrieval scope, and a budget over the whole response

**Status:** proposed — pending independent cross-review and human approval before `accepted`.
**Date:** 2026-10-14
**Deciders:** Claude Code (analysis and implementation); pending human approval and cross-review by another agent
**Supersedes / Superseded by:** none. It narrows how `binding_match` interprets file bindings (Phase 1.2 "a file binding governs every symbol in the file") and how `retrieval` counts its budget (E7 finding C).

ADR-0018 is not used here: that number was taken by a native structural
provider ADR written on a local branch that was never pushed (see
`docs/work-items/context-integrity-1.2.md`, §Lost work).

## Context

A team using Rationale on a production service reported, from their own canon:

1. `explain_target` presented superseded Records as current (6 of 27 for one
   service), with no status.
2. A symbol query returned the context of the whole file; a folder query
   returned nothing useful.
3. `prepare_change` responses exceeded their own budget (median ≈ 3,800 tokens
   for a 2,400 budget).
4. A stale index could not bind symbols created on the same branch.
5. A constraint written after an incident did not reach other modules that
   depend on the same invariant.

The same failures reproduce on the synthetic project in `eval/` and, for 2 and
3 and a stale index, on Rationale's own repository (`eval/results/`).

The causes, read in the code at v1.1.0:

- **Three definitions of "still governs".** `explain` filtered nothing;
  `retrieval` honored only the old Record's `superseded_by`; the canon and the
  Control Room used `canon::superseded_ids`. A Record replaced only through the
  new Record's `supersedes` (a branch merge that never touched the old file)
  stayed current.
- **Companion file bindings.** Every `path::symbol` binding also wrote a file
  binding for the same path, and a file binding governed every symbol in the
  file. A Record about `cancelShipment` governed `createShipment`.
- **No folder semantics.** A folder target matched only bindings whose
  `path_hint` was exactly the folder, which capture never wrote.
- **A budget over the packet only**, with `bytes / 4`, while the response also
  carried diagnostics and a per-binding assessment, and Claude's current
  tokenizer produces about 30% more tokens than earlier ones.
- **Stale structure served as observed.** Seen live on this repository: the
  provider returned calls into `src/providers/native/*`, files of another
  checkout indexed at the same path.

## Decision

1. **One authority view.** `lifecycle::StatusIndex` derives `active`,
   `superseded`, `revoked`, and `retired` once per query from both sides of
   every supersession (a pinned Record only when it says so itself). Only
   active Records are served as authority by `prepare_change` and
   `explain_target`; the others are served as `history` with their status and
   current replacement, and their text only on request.
2. **Scope follows the binding.** A symbol binding (confirmed `structural_id`,
   or a lexical `symbol` when the file defines it and the index does not know
   it yet) governs that symbol. A file binding governs every symbol in the file
   only when it is not the companion of a symbol binding in the same Record, or
   when it is marked `scope: file`. A folder binding (`type: directory`) governs
   everything below it. A folder query returns the Records bound inside it.
   New captures write no companion file bindings.
3. **Invariants travel along observed edges, never by similarity.**
   `critical`, `high`, or pinned constraints of what the target uses, and
   `critical` or pinned ones of what uses it, arrive as `inherited_constraints`
   with the edge.
4. **The working tree overrules the index.** Structural nodes whose file is
   gone or whose symbol no longer appears are dropped and reported as a stale
   index; a symbol the file defines but the index does not know makes
   `retrieval.status: index_incomplete`.
5. **The budget covers the whole response** with an upper-bound estimate
   (`src/tokens.rs`): ASCII at 2.7 characters per token, code at 2.0, non-ASCII
   at one each. Diagnostics and the per-binding assessment need `verbose`.
   What governs a symbol or file is never cut, nor are pinned or critical
   Records; a folder query is an overview whose other Records may be cut by
   priority. Constraints that only share words with the intent are cut first,
   and words common to a quarter of the canon do not count as shared. What is
   cut is listed in `omitted` with the call that returns it; an overflow is
   declared.

## Evidence

- Synthetic evaluation (`eval/results/v1.2.0-dev/report.md`): 15 of 15
  retrieval scenarios pass against 1 of 15 for v1.1.0; superseded or revoked
  Records served as authority 7 → 0; Records of another symbol 3 → 0; required
  Records missing 10 → 0; irrelevant Records 11 → 0; response tokens
  p50/p95/max 1,038/2,213/2,427 → 779/1,626/1,743.
- Rationale's own repository with the real provider
  (`eval/results/v1.2.0-dev/real-repo.json`, 12 targets): estimated response
  tokens p50 3,068 → 2,370 and 10 → 2 targets over the default budget (both
  declared overflows of governing knowledge); the folder query `src/providers`
  went from 0 to 3 governing Records; `src/agents.rs::install` from 5 Records
  (file-wide) to 1.
- Interleaved A/B on the same repository (`ab-latency.json`): `prepare_change`
  p50 116.9 → 110.9 ms.
- Token ratios: Anthropic's token-counting documentation (the count is an
  estimate; Claude 4.7 and later tokenize to about 30% more tokens) and a
  third-party measurement with `count_tokens` (playcode.io, July 2026: JSON
  ≈3.0, English ≈3.3, code 2.2–2.45 characters per token). See
  `docs/research/context-integrity-2026-10.md`.

## Alternatives considered

- **Label superseded Records instead of excluding them.** Rejected: Claude Code's
  own documentation says that with contradictory instructions the model "may
  pick one arbitrarily", and Chroma's context-rot study measured that
  distractors degrade accuracy. History stays available, but out of the
  authority lists.
- **Keep file bindings governing the whole file and rank symbol matches
  higher.** Rejected: ranking does not remove the other Records from the
  packet or from the verdict list; the agent still has to argue with rules that
  do not apply.
- **Embeddings or semantic similarity for cross-module invariants.** Rejected:
  remote embeddings are out of scope (`Arquitectura §27`), and similarity is
  not a justified dependency. Observed edges and explicit folder bindings are.
- **Count tokens with a tokenizer.** Not available offline; the API endpoint
  needs network and a key. The estimate is declared as an upper bound.

## Consequences

- Responses are smaller and stricter. Agents that read `diagnostics`, the
  per-binding assessment, or `intent_conflicts[].statement` must pass `verbose`
  or read the statement from the lists (documented in the changelog).
- Records captured by v1.1 with companion file bindings stop governing other
  symbols of their file. This is the intended fix; a Record that really
  governs the whole file can be re-bound to the file alone.
- Older binaries reading a canon written by this one see lexical symbol
  bindings as file bindings, folder bindings as unmatched, and retirements as
  revocations: degraded, never wrong in the other direction.

## Risks

- **A whole-file rule hidden behind a companion binding.** Mitigation:
  `scope: file`, and a file declared on its own, keep whole-file semantics;
  file and folder queries still show every Record in the file.
- **The estimate under-counts dense non-ASCII or unusual content.** Mitigation:
  non-ASCII counts one token per character; the estimate was not validated
  against `count_tokens` in this repository — see Revisit trigger.
- **Lexical symbol detection misses a definition form.** Then the Record falls
  back to the whole file, with a warning: the v1.1 behavior.

## Validation

`cargo test` (unit tests in `lifecycle`, `binding_match`, `symbols`, `tokens`,
`retrieval`; integration tests in `tests/context_integrity.rs`), the synthetic
evaluation gate in CI, and the real-repository runs above.

## Revisit trigger

A measurement with `count_tokens` showing the estimate below the real count on
Rationale responses; or a team reporting a whole-file rule that stopped
governing after upgrading; or a provider that reports index freshness, which
would replace the working-tree check.
