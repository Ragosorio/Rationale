# Writing Records

How to turn what you learned into candidates that the capture gate keeps and
that future agents can act on.

## Contents

- The durability test
- Choosing the kind
- Statement
- Rationale
- One decision per Record
- Bindings and relationships
- Supersedes
- Severity, id, evidence, risks, and subject
- Language
- Complete example
- Discard reasons
- Checklist

## The durability test

A candidate belongs in the canon only when both answers are yes:

1. Will it still be true after this change is merged?
2. Would a future agent changing this code act differently if it knew it?

Only then set `durability: "durable"`. The gate never assumes durability: a
missing value is discarded, and `transient` is discarded by design.

## Choosing the kind

| Kind | Choose it when | Test |
|---|---|---|
| `constraint` | Something must stay true, and breaking it causes a defect, an incident, or a broken contract | "If someone violates this, what breaks?" has a concrete answer |
| `decision` | The code takes one approach over a plausible alternative, for a reason | You can name the rejected alternative |
| `risk` | A hazard exists that the code does not make obvious | You can describe how it goes wrong |
| `exception` | A rule or convention is deliberately not followed in a bounded place | You can name the rule and where the exception ends |

When a choice produces a rule that must hold, capture the rule as a
`constraint`. Capture the choice as a separate `decision` only when someone
might revisit the choice on its own.

## Statement

The statement is the assertion future agents check their changes against.

- One assertion, in the present tense, about how the code behaves.
- Specific: name the concept, the condition, and the limit.
- Checkable against the code or its tests.
- At least 16 characters and 3 words; shorter statements are discarded.
- Not a change log. A statement that opens with "Added", "Updated",
  "Refactored", and similar verbs, with no causal word anywhere in the
  statement or rationale, is discarded as `mechanical_noise`.

| Weak | Strong |
|---|---|
| Retries were changed. | Payment retries stop after three attempts for the same charge. |
| Be careful with the cache. | A tenant's cache entries are invalidated before its permission change is committed. |
| Uses a lock. | `release` waits 50 ms before dropping the lock so in-flight readers on network file systems can finish. |

## Rationale

The rationale is the cause: why the statement must hold. Give at least one of:

- the consequence of violating it ("otherwise the processor blocks the merchant
  for an hour");
- the requirement, contract, or incident behind it ("the processor
  rate-limits a merchant after four failed attempts");
- the alternative that was rejected and why ("exponential backoff outlasted the
  webhook timeout").

It must add information. A rationale equal to the statement, or contained in
it, is discarded as `rationale_restates_statement`. Causal words such as
*because*, *otherwise*, *so that*, *must*, or *never* come naturally here.

## One decision per Record

Split into separate candidates when the parts:

- could be replaced or revoked independently;
- answer different questions;
- have different lifetimes;
- would be needed by different future readers.

Keep a single candidate when the parts only make sense together. Splitting
"retries stop after three attempts because the processor blocks merchants
after four" into one Record about three and another about four produces
fragments that mean nothing apart.

A correct split: "payment requests carry an idempotency key" (it stops
duplicate charges) and "webhook signatures are compared in constant time" (it
stops timing attacks) are two decisions with different reasons, code, and
lifetimes.

## Bindings and relationships

`bindings` anchor the Record to the code it governs. A candidate without at
least one binding that resolves to a real file is discarded as
`no_meaningful_binding`.

- Use `path::symbol` when the rule governs a function, type, or method, for
  example `"src/billing/retry.rs::backoff"`. The provider confirms the symbol;
  when it cannot, the file binding still counts.
- Use a file path when the rule governs the whole file.
- The structured form works too:
  `{"path": "src/billing/retry.rs", "symbol": "backoff"}`.
- Bind only the code the rule governs. Several bindings are fine when the rule
  spans files; a directory or an unrelated file is not.
- Paths are relative to the repository root. Bindings into `.rationale/` or
  `.rationale-local/` are ignored.
- Bindings to files with uncommitted changes are stored as `provisional` until
  the code is committed.

When the Record explains why a relationship exists, add `relationships`:

```json
"relationships": [
  { "source": "src/checkout.rs::checkout", "kind": "calls", "target": "src/inventory.rs::reserve_stock" }
]
```

Valid `kind` values are `calls`, `uses`, `writes`, `imports`, `defines`,
`implements`, `tests`, `configures`, `depends_on`, `http_calls`, `contains`,
and `decorates`. A relationship that resolves also anchors the Record.

## Supersedes

Name an existing Record's id in `supersedes` when the new candidate replaces
it, typically because the packet showed a Record your change made false.

- The replaced Record must be `active`. An unknown or inactive id is ignored
  with a warning, and both Records coexist.
- An agent cannot replace a `pinned` Record. The candidate is not written and
  becomes a conflict for a person (`references/conflicts.md`).
- The new statement must differ from the old one. A candidate whose normalized
  statement equals an active Record's is discarded as `duplicate`, even when it
  supersedes that Record. When only the location changed, state the rule
  precisely for its new code, for example by naming the new symbol.
- Do not supersede a Record just to reword it when its meaning did not change.

## Severity, id, evidence, risks, and subject

| Field | Guidance |
|---|---|
| `severity` | `critical` (security, money, data loss), `high` (user-visible breakage), `medium` (the default; the gate writes it with a warning when absent), `low` (local cost) |
| `id` | Optional. `<kind>.<slug>` with lowercase letters, digits, hyphens, and dots, for example `constraint.payments-idempotent`. The prefix must equal `kind`. An id already in use is discarded; omit `id` and the gate generates one |
| `evidence` | Where the knowledge comes from, for example `[{"type": "test", "path": "tests/billing/retry_test.rs", "note": "stops_after_three_attempts"}]`. `type` defaults to `reference` |
| `risks` | Short consequences of violating the Record, for example `["Merchant blocked by the processor for one hour"]` |
| `subject` | Optional grouping, for example `{"id": "billing.retries", "title": "Payment retries", "type": "domain"}`. Reuse a Subject from `.rationale/subjects/` when one fits. `type` is one of `project`, `workspace`, `package`, `service`, `domain`, or `target` |

## Language

Write `statement`, `rationale`, and `risks` in the language the existing
Records use; in an empty canon, use the user's language. Ids and field values
keep their fixed form (`constraint.payments-idempotent`, `durable`).

## Complete example

```json
{
  "kind": "constraint",
  "id": "constraint.payment-retries-capped-at-three",
  "statement": "Payment retries stop after three attempts for the same charge.",
  "rationale": "The processor blocks a merchant for one hour after four failed attempts, so a fourth retry turns one failed payment into an outage for every customer of that merchant.",
  "durability": "durable",
  "severity": "high",
  "bindings": ["src/billing/retry.rs::backoff"],
  "supersedes": ["decision.retry-cap-five"],
  "evidence": [{ "type": "test", "path": "tests/billing/retry_test.rs", "note": "stops_after_three_attempts" }],
  "risks": ["Merchant blocked by the processor for one hour"]
}
```

## Discard reasons

| `reason` | Cause | Fix |
|---|---|---|
| `malformed_candidate` | Not an object, or a field has the wrong type | Send the documented shape |
| `invalid_kind` | `kind` is not one of the four kinds | Use `constraint`, `decision`, `risk`, or `exception` |
| `durability_not_declared` | `durability` is missing or not `durable`/`transient` | Declare `durable` when it passes the durability test |
| `transient` | Declared `transient` | Nothing to fix; it belongs in `summary` |
| `statement_not_meaningful` | Statement under 16 characters or 3 words | State the full assertion |
| `missing_rationale` | Rationale under 16 characters or 3 words | Give the cause |
| `rationale_restates_statement` | Rationale equals, or is contained in, the statement | Give the consequence, the source, or the rejected alternative |
| `mechanical_noise` | Statement opens like a change log and nothing carries a cause | Capture why, or leave it to Git |
| `invalid_severity` | Severity is not one of the four levels | Use `critical`, `high`, `medium`, or `low` |
| `invalid_id` | Id does not match `<kind>.<slug>` | Fix the id or omit it |
| `id_kind_mismatch` | Id prefix differs from `kind` | Make them match |
| `no_meaningful_binding` | No binding or relationship resolved to a real file | Bind existing code |
| `id_already_exists` | The requested id belongs to another Record | Use a new id, and add `supersedes` if it replaces that Record |
| `duplicate` | An active Record already states the same thing (see `duplicate_of`) | Nothing to write; cite the existing Record |
| `legacy_contract` | A pre-1.0 call shape without `candidates` | Send `candidates` |
| `write_failed` | The canon could not be written | Report the `detail`; do not retry blindly |

## Checklist

```text
For each candidate
- [ ] Passes the durability test; durability is "durable"
- [ ] kind fits the knowledge
- [ ] statement: one specific, checkable assertion, not a change log
- [ ] rationale: gives the cause and does not repeat the statement
- [ ] exactly one decision
- [ ] bindings: the narrowest existing file or path::symbol
- [ ] supersedes: only active Records this one replaces; pinned ones go to a person
- [ ] severity chosen on purpose
- [ ] language matches the canon
- [ ] validator reports no errors
```
