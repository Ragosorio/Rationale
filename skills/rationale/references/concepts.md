# Concepts

The vocabulary Rationale uses. Terms appear in tool results exactly as written
here.

## Contents

- Canon and Records
- Bindings and relationships
- Provenance, authority, and lifecycle
- Operations, packets, and the gate
- Providers and local data

## Canon and Records

The **canon** is `.rationale/` at the repository root: plain YAML, versioned
with the code and reviewed in the same pull requests. It is the only source of
truth. Everything else Rationale stores is derived or local and can be
rebuilt.

A **Record** is one durable piece of knowledge about the code, stored as
`.rationale/records/<id>.yaml`. Its `kind` is one of:

| Kind | Holds | Example statement |
|---|---|---|
| `constraint` | What must stay true; breaking it breaks something | A charge that reached the bank is never retried. |
| `decision` | Why the code is this way instead of an obvious alternative | Payment retries use a fixed three-attempt cap instead of exponential backoff. |
| `risk` | A known hazard to keep in view when changing the code | The refund worker can refund twice when the ledger write times out. |
| `exception` | A deliberate, scoped deviation from a rule or convention | `legacy_export` keeps synchronous file I/O because its callers cannot await. |

Every Record has a `statement` (the assertion), a `rationale` (the cause), a
`severity` (`critical`, `high`, `medium`, or `low`), bindings, provenance,
authority, and a lifecycle. A **Subject** in `.rationale/subjects/` can group
Records about one concern.

## Bindings and relationships

A **binding** ties a Record to the code it governs: a file (`src/retry.rs`) or
a symbol (`src/retry.rs::backoff`). A Record governs a target when one of its
bindings matches it. `prepare_change` and `explain_target` use the same
matcher, so they agree on what governs what.

Symbol bindings are confirmed through the structural provider. A binding
created while its file has uncommitted changes is `provisional`: someone else
with the same repository cannot verify it until that code is committed.

A Record can also explain a **relationship**, such as why `checkout` calls
`reserve_stock`. Rationale re-derives the state of every explained relationship
on each call and never deletes the explanation:

| State | Meaning | What to do |
|---|---|---|
| `observed` | The direct relationship exists in the current index | Treat the explanation as current |
| `indirect` | No longer direct, but a bounded path still connects both ends | Mention the path; the reason likely still applies |
| `orphaned` | The relationship cannot be located; the explanation may be stale | Report it; do not drop the Record because of it |
| `unknown` | The provider could not verify it | Report it; absence is not proof |

## Provenance, authority, and lifecycle

**Provenance** says where a Record came from:

- `agent_asserted`: written by an agent through `finalize_change`, with the
  client, session, and operation that asserted it.
- `human_stated`: declared by a person.
- `migrated`: carried over from the pre-1.0 approval workflow.

**Authority** says how far a Record can move:

- `normal`: a newer candidate can replace it by naming its id in
  `supersedes`.
- `pinned`: a rule the project fixed. Agents rely on it. A candidate that tries
  to replace it is not written and becomes a **conflict** that only a person
  resolves.

People pin and unpin with `rationale pin` and `rationale unpin`. Pinning,
unpinning, and adopting a replacement for a pinned Record require an actor
declared under `authority:` in `.rationale/config.yaml`.

The **lifecycle** of a governing Record is `active`. `superseded` and `revoked`
Records are history and no longer govern. People correct, dispute, revoke, or
supersede a Record with `rationale review-record`, which appends an audited
lifecycle event.

## Operations, packets, and the gate

`prepare_change` opens an **operation** and returns its `operation_id` with a
**packet**: the bounded context compiled for one target and one intent.
`finalize_change` closes the same operation, so the Control Room
(`rationale ui`) can show the change from the context the agent received to
the memory it captured.

The **capture gate** runs inside `finalize_change`. It checks each candidate's
shape and text, resolves its bindings, discards malformed, transient,
mechanical, and duplicate candidates with a reason, turns attempts to replace
pinned Records into conflicts, and writes the rest as Records in the same call.
There is no approval queue.

## Providers and local data

**Codebase Memory** is the optional structural provider. It answers where code
is and how it connects. Rationale reaches it only through its public tools and
reports `provider_status` and `provider_coverage` honestly. Without it, file
bindings still work and structure is reported as degraded.

Local and derived data never decide anything: `.rationale-local/` holds
activity, operation snapshots, and pending conflicts and is never committed;
the search cache lives under `~/.cache/rationale/`.
