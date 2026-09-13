# Conflicts

A conflict exists when a candidate tried to supersede a `pinned` Record. The
candidate was not written; it waits for a person's decision. This is where the
project's authority is at stake, so the rules here are strict and the reason is
concrete: pinning exists so that no agent can move certain rules. An agent that
chose for the person would defeat the pin, and the canon would record an
agent's choice as a human decision.

## What you may and may not do

| You may | You may not |
|---|---|
| Show the pinned rule, the new assertion, and the question verbatim | Recommend an option unless the person asks for your analysis |
| Explain what each option does | Choose an option, or read silence or a vague reply as a choice |
| Call `resolve_conflict` after an explicit answer | Paraphrase the answer in `human_answer` |
| Give the person the `rationale resolve` command | Run `rationale resolve`, `pin`, or `unpin` yourself |

## Steps

1. Collect the conflicts from the `conflicts` array of `finalize_change`, or run
   `rationale conflicts --json`.
2. For each conflict, present `conflict_id`, the pinned Record
   (`pinned_record_id` and `pinned_statement`), the `candidate_statement`, and
   the `question`. Use the conflict template in `assets/report-templates.md`.
   Translate labels into the user's language; quote statements verbatim.
3. Explain the options:
   - `keep_pinned`: the pinned rule stays and the new assertion is dropped.
   - `adopt_new`: the new assertion replaces the rule and inherits `pinned`.
     It requires the person's Git identity to be declared under `authority:`
     in `.rationale/config.yaml`.
4. Wait for an explicit answer that names one option.
5. Then either:
   - the person runs `rationale resolve <conflict-id> keep-pinned` or
     `rationale resolve <conflict-id> adopt-new` in an interactive terminal; or
   - you call `resolve_conflict(conflict_id, decision, human_answer)` with
     `decision` set to `keep_pinned` or `adopt_new` and `human_answer` set to
     the person's literal words.
6. Report what the tool returned. Do not call the conflict resolved before it
   confirms.

## Edge cases

- **Ambiguous answer** ("whatever you think", "probably the new one"): ask
  again and name both options.
- **The person wants different wording than either statement**: apply their
  decision first, then capture a new candidate with the wording they want.
- **`adopt_new` is refused for missing authority**: tell the person which actor
  must be declared. Do not edit `.rationale/config.yaml` for them.
- **Several conflicts**: handle them one at a time. One answer covers only the
  conflict it names, unless the person says it applies to all of them.
