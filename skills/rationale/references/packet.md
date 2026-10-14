# Reading the packet

`prepare_change` returns an `operation_id`, a `packet`, the `latency_ms`, and,
when an active Record governs the target, a compact `assessment` of it
(`record_id`, `state`, `assessment_reason`). Pass `verbose: true` only to debug:
it adds `diagnostics` and the per-binding assessment, and it costs tokens.
Fields with nothing to report may be omitted.

## Contents

- First: what kind of answer this is
- Identity and snapshot
- Governing knowledge
- Inherited constraints
- History
- Relationships and structure
- Conflicts, risks, and unknowns
- Size, budget, and what was left out
- Execution advice

## First: what kind of answer this is

Read `retrieval` before anything else:

| `retrieval.status` | Meaning | Act on it |
|---|---|---|
| `found` | Active Records apply | Read them and state a verdict |
| `none_found` | No active Record governs the target | Proceed; an empty canon is not proof that nothing matters |
| `index_incomplete` | The file defines the symbol but the structural index does not know it (stale index, or a symbol new on this branch) | Records bound by file or symbol still apply; callers, callees and inherited constraints were not verified. Say so |
| `target_missing` | The path no longer exists | If `records_bound_to_missing_path` > 0, Records still describe removed code: tell the person; `rationale retire` is theirs to run |

`retrieval.scope` is `symbol`, `file`, or `folder`. A symbol query returns the
Records bound to that symbol plus file-level rules of its file; a file query
returns every Record bound in the file; a folder query returns the Records
bound inside it, highest priority first.

## Identity and snapshot

| Field | Meaning | Act on it |
|---|---|---|
| `operation_id` | The operation this packet opened | Pass it to `finalize_change`, or abandon it |
| `target`, `resolved_target` | What you asked for, and what Rationale resolved it to | If they name different code, locate the target again |
| `intent` | The intent used for conflict detection | Confirm it is the change you will make |
| `snapshot.git_revision` | HEAD when the packet was compiled | Cite it when reporting |
| `snapshot.consistency` | `exact`, `working-tree-ahead` (uncommitted changes), `structural-index-behind` (HEAD moved since the Record was bound), or `unresolved` | Mention anything other than `exact` when it affects the change |
| `snapshot.provider_status`, `provider_coverage` | Structural provider health | Report anything other than `successful` / `complete`; missing structure is not "nothing there" |

## Governing knowledge

`critical_constraints` and `decisions` hold **active** Records only. A Record
that was superseded, revoked, or retired is never here (see History). Each
entry carries:

| Field | Meaning |
|---|---|
| `id`, `statement`, `rationale` | The Record, its assertion, and why; quote statements verbatim |
| `severity` | `critical`, `high`, `medium`, or `low` (on `critical_constraints`) |
| `authority` | `pinned` (a person fixed it) or `normal` |
| `provenance` | `human_stated`, `agent_asserted`, or `migrated` |
| `governs_target` | `true` when a binding matches the target |
| `match_kind`, `why` | How it matched: `structural` (symbol), `unconfirmed-symbol` (symbol checked in the file, not yet in the index), `file-contains-symbol` (a file-level rule), `file-exact`, `relationship-endpoint`, `directory-scope` (a folder binding contains the target), `directory-member` (folder query) |
| `replaces` | Records it superseded: do not apply those |
| `exceptions` | Active exceptions that relax this rule (on constraints) |
| `stale_bindings` | Some bindings point at paths that no longer exist: the Record may describe removed code |
| `flags`, `note` | `orphaned_exception`: an exception whose rule no longer governs; ask a person before relying on it |

`governance_verdict_required: true` means at least one active Record governs
the target: state a verdict for each before editing. `primary_reason` appears
only when that reason is not already in an entry above.

Governing knowledge is never truncated to fit the budget.

## Inherited constraints

`inherited_constraints` lists `critical`, `high`, or pinned constraints that do
not bind the target but govern code it depends on, or that depends on it,
through an observed edge (`via`, for example `issueInvoice calls toCents`).
An invariant like "money is integer cents" reaches billing through the call to
the shipping helper that holds it. They are as binding as the rule itself:
respect them or say why they do not apply. Similarity never produces one.

## History

`history` lists Records bound to the target that no longer govern: `id`,
`kind`, `status` (`superseded`, `revoked`, `retired`), and `replaced_by` (the
current replacement, following the whole chain). Never treat them as current.
`explain_target(target, include_history=true)` returns their text when the
past decision matters.

## Relationships and structure

`relationships` lists relationships that a Record explains: `source`, `kind`,
`target`, `state` (`observed`, `indirect`, `orphaned`, `unknown`; see
`references/concepts.md`), `path` when indirect, and the Record that explains
the edge. They are never cut.

`structure` is the bounded neighborhood from the provider: `nodes[]` with a
`role` (`target`, `explained`, `caller`, `callee`, `dependency`, `dependent`,
`test`, `context`), `edges[]`, and how much was considered. Nodes the working
tree does not contain are dropped and reported in `warnings` as a stale index:
do not trust the provider's view of code that is gone.

`relevant_code` holds the target's source; read the file when `truncated` is
`true` or when the section was omitted.

## Conflicts, risks, and unknowns

`intent_conflicts[]` compares your intent with selected Records: `record_id`
(its text is in the lists above), `detection` (`governs-target`: a verifiable
binding fact; `lexical-overlap`: shared terms only, a hint), `polarity` (a
lexical guess, never a verdict), `shared_terms`, and `epistemic_note`.

`known_risks` lists risks attached to the selected Records. `known_unknowns`
lists what Rationale could not establish. `warnings` explain degraded steps.
Report them rather than hiding them.

## Size, budget, and what was left out

`max_tokens` (default 2400) caps **the whole response**, measured with an
upper-bound estimate (`budget.estimated_tokens`, `budget.estimator:
upper-bound`). It is a ceiling, not a target. When the response would exceed
it, Rationale drops content in this order: affected targets, constraints that
only share words with your intent, history, structure, code, inherited
constraints that are not critical or pinned, and — in a folder query — folder
Records that are not critical or pinned, lowest priority first. Each dropped
section is listed in `omitted` with `count`, `ids`, and `expand`: the call that
brings it back. What governs a symbol or file is never dropped, nor are pinned
and critical Records; when they alone exceed the ceiling, `budget_overflow`
says so. A folder query is an overview: drill into a subfolder or file for the
rest.

## Execution advice

With `advise: true` (or `advice.enabled` in the project config),
`execution_advice` suggests a model tier (`fast`, `balanced`, `deep`, mapped to
a Claude Code alias in `model`), a reasoning `effort`, and the `context` size,
with `reasons`. It is a heuristic over this packet, not a measurement. The
person's choice wins: delegate with the Agent tool's `model` and `effort` if
you use subagents, or ask before switching the session.
