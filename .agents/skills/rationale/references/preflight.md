# Preflight

Read the Records that govern a target before changing it, and say how your
intent relates to them. The purpose is simple: no rule the project depends on
gets broken without anyone noticing.

## Contents

- Checklist
- 1. Locate the target
- 2. Call `prepare_change` with the real intent
- 3. Read what governs the target
- 4. State a verdict for each governing Record and conflict
- 5. Report uncertainty
- 6. Decide the next step
- When the tools are unavailable

## Checklist

Copy this checklist into your working notes and complete it in order:

```text
Preflight
- [ ] 1. Target located, provider coverage noted
- [ ] 2. prepare_change called with the real intent; operation_id kept
- [ ] 3. Governing Records and intent conflicts read
- [ ] 4. Verdict stated for each one
- [ ] 5. Uncertainty reported
- [ ] 6. Next step decided: proceed, adjust, or ask
```

## 1. Locate the target

Name the narrowest code you are about to change: `path/to/file.ext::symbol`
when a symbol is the unit of change, `path/to/file.ext` for a file-wide change.
Paths are relative to the repository root.

With Codebase Memory, search the graph for the symbol and trace its callers
before settling on the target; an unexpected caller can carry Records of its
own. Without it, use text search and note that structural coverage is
unavailable. Use only symbol names you have seen in the code.

When a change touches several non-trivial targets, prepare each one and keep
the `operation_id` of the primary target for capture.

## 2. Call `prepare_change` with the real intent

Write the intent as the change you will make: the behavior, the object, and the
condition that matters.

- Too vague: `refactor retry logic`
- Useful: `cap payment retries at three attempts and stop once the bank accepted the charge`

The intent drives conflict detection, so a vague intent hides the conflicts you
need to see. Leave `mode` unset: sending `intent` already enables detection.

```text
prepare_change(target: "src/billing/retry.rs::backoff",
               intent: "cap payment retries at three attempts and stop once the bank accepted the charge")
```

Keep the returned `operation_id`. `finalize_change` closes the same operation.

## 3. Read what governs the target

[packet.md](packet.md) explains every field. The ones that shape the decision:

- `retrieval.status` first: `found`, `none_found`, `index_incomplete`, or
  `target_missing` tell you what kind of answer you got.
- `critical_constraints` and `decisions` with `governs_target: true` govern
  your change. Note each one's `authority`, `provenance`, and `why`.
- `inherited_constraints` are invariants that reach the target through code it
  uses: they bind your change as much as its own Records.
- `history` lists Records that no longer govern. Do not apply them.
- `relationships` explain why edges around the target exist, with their state.
- `intent_conflicts` flag Records that may oppose your intent.
  `detection: governs-target` means the Record is bound to the target (a fact);
  `detection: lexical-overlap` means shared words only (a hint).
- `governance_verdict_required: true` means you owe a verdict before editing.

## 4. State a verdict for each governing Record and conflict

For each one, say:

- **respects**: the change keeps the Record true. Give the reason in one line.
- **contradicts**: the change would make the Record false.
- **undetermined**: the packet and code do not settle it. Say what is missing.

Judge against the Record's statement and the code, not the `polarity` hint,
which is noisy in both directions. Use the preflight template in
`assets/report-templates.md`.

## 5. Report uncertainty

State each of these when present. None of them proves anything on its own.

- relationships in state `orphaned` or `unknown`;
- `snapshot.provider_status` other than `successful`, or
  `snapshot.provider_coverage` other than `complete`;
- `known_unknowns` and `warnings`;
- `budget_overflow`: governing knowledge exceeded the token ceiling and was
  still delivered in full, so the packet is larger than requested;
- `omitted`: sections cut to fit the budget, each with the call that brings it
  back; fetch what your change needs;
- `stale_bindings` or an `orphaned_exception` flag on a Record;
- a `resolved_target` that is not the code you meant.

## 6. Decide the next step

| Situation | Next step |
|---|---|
| Nothing governs the target | Proceed. Capture may still be needed afterwards. |
| You decide not to change the code | Close the operation: `finalize_change(operation_id, abandon=true, summary="why")`. |
| Every verdict is `respects` | Proceed with the smallest change consistent with the packet. |
| Contradicts a `normal` Record | Proceed only if the user's request requires it. Tell the user, and plan a capture candidate that names the Record in `supersedes` and explains the new reason. |
| Contradicts a `pinned` Record | Stop. Explain the rule and how the intent collides with it, and ask the person whether to change the plan or pursue a replacement they will decide. |
| `undetermined` on a `critical` or `high` Record | Ask before editing, naming exactly what you could not determine. |
| `undetermined` on other Records | Say so, proceed carefully, and check the Record again after the change. |
| The target did not resolve | Locate it again. Do not edit code whose governance you could not read. |

## When the tools are unavailable

When `prepare_change` fails or the `rationale` server is missing, tell the user
and follow `references/health.md`. Do not describe governing Records you have
not received.
