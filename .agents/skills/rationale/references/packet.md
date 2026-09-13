# Reading the packet

`prepare_change` returns an `operation_id` and a `packet`, plus `diagnostics`
and, when a Record governs the target, an `assessment` of it. Fields with
nothing to report may be omitted.

## Contents

- Identity and snapshot
- Governing knowledge
- Relationships and structure
- Conflicts, risks, and unknowns
- Size and budget

## Identity and snapshot

| Field | Meaning | Act on it |
|---|---|---|
| `operation_id` | The operation this packet opened | Pass it to `finalize_change` |
| `target`, `resolved_target` | What you asked for, and what Rationale resolved it to | If they name different code, locate the target again |
| `intent` | The intent used for conflict detection | Confirm it is the change you will make |
| `snapshot.git_revision` | HEAD when the packet was compiled | Cite it when reporting |
| `snapshot.consistency` | `exact`, `working-tree-ahead` (uncommitted changes), `structural-index-behind` (HEAD moved since the index), or `unresolved` (Git state unknown) | Mention anything other than `exact` when it affects the change |
| `snapshot.provider_status` | `successful`, `degraded`, or `unavailable` | Report anything other than `successful` |
| `snapshot.provider_coverage` | `complete`, `partial`, or `unknown` | Report anything other than `complete`; missing structure is not "nothing there" |

## Governing knowledge

`critical_constraints` and `decisions` hold the Records selected for the
target. Each entry carries:

| Field | Meaning |
|---|---|
| `id`, `statement` | The Record and its assertion; quote both verbatim |
| `rationale` | Why the decision was made (on `decisions`) |
| `severity` | `critical`, `high`, `medium`, or `low` (on `critical_constraints`) |
| `authority` | `pinned` or `normal` |
| `provenance` | `agent_asserted`, `human_stated`, or `migrated` |
| `governs_target` | `true` when a binding matches the target, so the Record governs your change |
| `match_kind` | How the binding matched, for example `file-contains-symbol` for a file binding that contains the target symbol |

`primary_reason` is the rationale of the leading governing Record, when there is
one. `governance_verdict_required: true` means at least one Record governs the
target or conflicts with the intent: state a verdict for each before editing.

Governing knowledge is never truncated to fit the budget.

## Relationships and structure

`relationships` lists relationships that a Record explains:

| Field | Meaning |
|---|---|
| `source`, `kind`, `target` | The edge, for example `checkout` `calls` `reserve_stock` |
| `state` | `observed`, `indirect`, `orphaned`, or `unknown` (see `references/concepts.md`) |
| `path` | The connecting path when the state is `indirect` |
| `record_id`, `statement`, `rationale`, `authority`, `provenance` | The Record that explains why the edge exists |
| `detail` | How the state was derived |

`structure` is the bounded neighborhood from the provider:

- `nodes[]`: `name`, `qualified_name`, `file_path`, `start_line`, `label`, a
  `role` (`target`, `explained`, `caller`, `callee`, `dependency`,
  `dependent`, `test`, or `context`), and `record_ids` when a Record explains
  the node.
- `edges[]`: `source`, `kind`, `target`, `state`, and `record_ids` when a
  Record explains the edge.
- `considered_nodes`, `considered_relationships`, `truncated`, `index_state`,
  and `provider` say how much was considered and whether the neighborhood was
  cut to its ceilings (`max_nodes`, default 12; `max_relationships`, default
  16). Relationships explained by Records are never cut.

`relevant_code` holds the target's source (`file_path`, `qualified_name`,
`start_line`, `end_line`, `source`, `truncated`). Read the file itself when
`truncated` is `true`. `affected_targets` lists the files and symbols the packet
considered affected.

## Conflicts, risks, and unknowns

`intent_conflicts[]` compares your intent with Records:

| Field | Meaning |
|---|---|
| `record_id`, `statement`, `severity`, `authority` | The Record involved |
| `governs_target` | Whether the Record is bound to the target |
| `detection` | `governs-target`: bound to the target, a verifiable fact. `lexical-overlap`: shared terms only, a hint |
| `polarity` | A lexical guess at direction; noisy both ways and never a verdict |
| `shared_terms` | The words that matched |
| `epistemic_note` | What Rationale did and did not verify |

`known_risks` lists risks attached to the selected Records. `known_unknowns`
lists what Rationale could not establish. `warnings` and `diagnostics` explain
degraded steps such as a missing provider, an unresolved symbol, or a stale
mapping. Report them rather than hiding them.

## Size and budget

`token_estimate` is the packet's size. `max_tokens` (default 2400) is a ceiling,
not a target, so a small neighborhood produces a small packet. When governing
knowledge alone exceeds the ceiling, `budget_overflow` says so and that
knowledge is still delivered in full.
