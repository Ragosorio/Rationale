# Capture

Close a change by writing back what stays true. Memory written here reaches
every future agent that touches this code, so it has to be accurate, durable,
and anchored to the code it governs.

## Contents

- Checklist
- 1. Confirm the change is finished
- 2. Gather facts
- 3. Separate durable knowledge from the change log
- 4. Draft and validate candidates
- 5. Call `finalize_change`
- 6. Read the result and report

## Checklist

Copy this checklist into your working notes and complete it in order:

```text
Capture
- [ ] 1. Change finished; test results known
- [ ] 2. Facts gathered from Git
- [ ] 3. Durable knowledge separated from the change log
- [ ] 4. Candidates drafted and validated
- [ ] 5. finalize_change called
- [ ] 6. Result read and reported
```

## 1. Confirm the change is finished

Run the tests that cover the change and note their real outcome, including
failures you could not fix. Knowledge that depends on code that does not work
yet is not durable.

## 2. Gather facts

```bash
git status --short
git diff --stat
git diff
```

A shortcut such as `/rationale-capture` may have injected this already. Keep
three things apart: what you observed (the diff, test output, the packet), what
the user asked for, and what you concluded.

## 3. Separate durable knowledge from the change log

Ask these questions about the change, the conversation, and the packet:

1. What must a future change here preserve, and what breaks if it does not?
2. Why this approach instead of the obvious alternative?
3. What hazard did you find that the code does not make obvious?
4. What deliberate exception did you make, and where does it stop applying?
5. Which Record from the packet is no longer true after this change?

Answers that stay true after the change become candidates. Everything else
(what was edited, the steps you took, temporary state, test counts) goes in
`summary`, which is reported but never becomes memory.

When every answer is "nothing", send no candidates. That is a correct outcome,
and no memory is written.

## 4. Draft and validate candidates

Follow `references/records.md` for kinds, statements, rationales, bindings, and
`supersedes`, starting from `assets/candidates.template.json`. Then run the
validator from the repository root:

```bash
python3 <skill-dir>/scripts/check_candidates.py - <<'JSON'
[ ...your candidates... ]
JSON
```

Fix every error. Read every warning and decide: `supersedes_pinned` means the
candidate will become a conflict a person must decide, so tell the user before
sending it.

When the user supplied a statement (for example `/rationale-capture "..."`),
use it only if it expresses a real decision, and still give it a rationale and
bindings.

## 5. Call `finalize_change`

```text
finalize_change(
  operation_id: "<from prepare_change>",
  summary: "Capped payment retries at three attempts in backoff and updated the retry tests.",
  target: "src/billing/retry.rs::backoff",
  candidates: [ ... ]
)
```

Without an `operation_id` (no preflight ran), the mechanical diff is taken
against `base_revision` when you pass one, and against HEAD otherwise.

## 6. Read the result and report

| Field | Meaning | What to do |
|---|---|---|
| `summary` | Counts of committed, discarded, and conflicts | Lead the report with them |
| `committed[]` | Records written: `id`, `kind`, `authority`, `path`, `bindings`, `superseded` | Report each id |
| `discarded[]` | `index`, `reason`, `detail`, `statement`, and `duplicate_of` for duplicates | Report each with its reason |
| `conflicts[]` | Candidates that tried to replace a pinned Record | Stop and follow `references/conflicts.md` |
| `superseded[]` | Records this capture replaced | Report them |
| `relationships[]` | Derived state of relationships the new Records explain | Report `orphaned` or `unknown` as uncertainty |
| `signals[]` | Facts observed in the change | Informational; they never write memory |
| `warnings` | Ignored bindings, missing severity, ignored `supersedes` | Report the ones that change what was written |

Fix a discard when it exposes a mistake: a restated rationale, a binding to a
path that moved, an id already in use. Correct the candidate and call
`finalize_change` again with only the corrected candidates. A `transient`,
`mechanical_noise`, or `duplicate` discard usually means there was nothing new
to remember; leave it.

Report with the capture template in `assets/report-templates.md`.
