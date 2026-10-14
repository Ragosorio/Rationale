# Core concepts

Rationale separates the memory agents write, the authority people keep, and the
structural state the provider reports.

## Entities

- **Record:** one durable piece of knowledge — a `constraint`, `decision`,
  `risk`, or `exception` — with a statement, a rationale that gives the cause, a
  severity, bindings, evidence, and a lifecycle. One decision per Record.
- **Binding:** ties a Record to the code it governs: a symbol, a file, or a
  folder. A symbol binding governs that symbol; a file binding governs every
  symbol in the file; a folder binding governs everything below it, which is
  how an invariant that spans modules is stated. The provider confirms
  symbols and they are stored with a portable id; a symbol the index does not
  know yet is bound lexically after checking that the file defines it.
  Bindings created from uncommitted code are `provisional`.
- **Relationship binding:** explains why a relationship between two nodes
  exists.
- **Subject:** the conceptual identity of a behavior or boundary, so that a
  decision is not tied to a single file by accident.
- **Evidence:** a verifiable reference that supports the statement.
- **Assessment:** a derived evaluation of epistemic status, authority,
  applicability, linkage, and revision consistency.
- **Operation:** what `prepare_change` opens and `finalize_change` closes, with a
  local snapshot of what was considered and selected.
- **Conflict:** a candidate that tried to replace a pinned Record. It is not
  written until a person decides.
- **Exception:** a Record of kind `exception` that relaxes a rule under stated
  conditions and names it in `exception_to`. When that rule stops governing,
  the exception is flagged as orphaned instead of silently applying to the
  replacement.

## Provenance and authority

Every Record declares its **provenance**: `agent_asserted` (with client,
session, and operation), `human_stated`, or `migrated` (from the pre-1.0
approval workflow). It is never upgraded silently.

It also declares its **authority**: `normal` by default, or `pinned` when the
project fixed it. A `pinned` Record governs like a normal one, but no agent can
replace it: the attempt becomes a conflict. Precedence is `pinned` over
`normal`, and an explicit `supersedes` over coexistence; Git SHAs are never
used to order Records.

## Lifecycle: current versus history

A Record is `active`, `superseded` (a newer Record replaced it, declared on
either side), `revoked`, or `retired` (a person retired the code it was bound
to with `rationale retire`). Only active Records are served as authority.
The others are history: packets list them with their status and current
replacement, and `explain_target` returns their text on request, so agents
never receive a superseded decision next to its replacement as if both
governed.

## What an answer is

Every packet says what kind of answer it is in `retrieval.status`: `found`,
`none_found` (an absence in the canon, not proof that nothing matters),
`index_incomplete` (the file defines the symbol but the structural index does
not know it yet), or `target_missing` (the path is gone; it reports how many
active Records are still bound to it). A query on a symbol, a file, and a
folder are different scopes (`retrieval.scope`).

Invariants also travel along dependencies: a `critical`, `high`, or pinned
constraint on code the target uses arrives as an inherited constraint, with the
observed edge that brought it. Similarity never brings one.

## Structural state of relationships

| State | Meaning |
|---|---|
| `observed` | The direct relationship exists in the current index. |
| `indirect` | It is no longer direct, but a bounded path still connects both ends. |
| `orphaned` | It cannot be located; the explanation may be stale. It is never deleted. |
| `unknown` | The provider could not verify it. Absence is not proof. |

## Canon and derived data

The versioned canon lives in `.rationale/`. The SQLite cache, operation
snapshots, and local activity are derived or local and can be deleted without
losing decisions. Deleting a Record does erase history: revoke or supersede it
with `rationale review-record` so the lifecycle keeps the reason.

## Responsibilities

- Codebase Memory provides location, symbols, and structural relationships.
- Rationale provides why it matters, what must stay true, and who may move it.
- The agent prepares context and captures durable knowledge, following the
  protocol and the [`rationale` skill](skills.md).
- The person pins the rules that matter, states their own rules with
  `rationale rule add`, decides conflicts, and applies retirements.

Read the original contract in [`Rationale_v0.5.md`](../../Rationale_v0.5.md)
and the 1.0 model change in
[`docs/work-items/vnext-implementation-plan.md`](../work-items/vnext-implementation-plan.md).
