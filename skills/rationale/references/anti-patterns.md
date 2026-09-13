# Anti-patterns

Review your work against this list before reporting. Each entry names a
failure, why it hurts, and what to do instead.

## Contents

- Preflight
- Capture
- Authority
- Reporting

## Preflight

| Anti-pattern | Why it hurts | Instead |
|---|---|---|
| Editing first and preparing later | The packet can no longer shape the plan; conflicts surface after the damage | Prepare before the first edit to non-trivial code |
| A vague intent (`refactor`, `cleanup`) | Conflict detection has nothing to compare against | Name the behavior you will change |
| Ignoring `governance_verdict_required` | A governing rule is broken without anyone saying so | State a verdict for each Record before editing |
| Treating `polarity: opposed` as proof | The lexical hint is noisy in both directions | Judge the Record's statement against the code |
| Treating `orphaned` or `unknown` as safe to ignore | The explanation may still be right, and the provider may be wrong | Report it as uncertainty |
| Guessing a symbol for `target` | Governance is read for the wrong code | Find the symbol in the code first |
| Simplifying odd code without `explain_target` | Removes a fence whose reason is on file | Explain first |

## Capture

| Anti-pattern | Why it hurts | Instead |
|---|---|---|
| A change log as a candidate ("Added a retry cap") | Git already has it; the gate discards it as `mechanical_noise`, or it clutters the canon | Put it in `summary`; capture why the cap must exist |
| A rationale that repeats the statement | No cause is recorded; the gate discards it as `rationale_restates_statement` | Give the consequence, the incident, or the rejected alternative |
| One candidate covering several decisions | Its parts cannot be superseded or revoked separately | One decision per Record |
| Splitting one decision into fragments | The fragments mean nothing on their own | Keep a single decision whole |
| Binding a directory or an unrelated file | The Record governs the wrong code, or nothing | Bind the narrowest file or symbol the rule governs |
| Marking a note `durable` to get it written | The canon fills with facts that stop being true | Use `durable` only for what holds after this change |
| Capturing guesses | Future agents act on invented reasons | Capture what the code, tests, or a person established |
| Writing Records in a different language than the canon | Duplicate detection weakens and readers lose the thread | Match the language of the existing Records |
| Editing `.rationale/records/*.yaml` by hand | Skips the gate, provenance, and conflict checks | Go through `finalize_change` |
| Skipping capture after learning a real constraint | The next agent rediscovers it, or breaks it | Capture it; send no candidates only when nothing durable was learned |

## Authority

| Anti-pattern | Why it hurts | Instead |
|---|---|---|
| Working around a pinned Record | Defeats the rule the project fixed | Stop and ask |
| Superseding a pinned Record to improve its wording | Creates a conflict a person must decide, for no real change | Ask the person, or leave it as is |
| Choosing a conflict option for the person | Records an agent's choice as a human decision | Present both options and wait for their answer |
| Paraphrasing `human_answer` | The audit trail no longer shows what the person said | Pass their literal words |
| Running `pin`, `unpin`, `resolve`, or `review-record` | These carry the person's authority | Give them the exact command |
| Declaring yourself under `authority:` | Pins stop meaning anything | Propose the block with the person's identity |

## Reporting

| Anti-pattern | Why it hurts | Instead |
|---|---|---|
| "Rationale approved this change" | Rationale compiles context; it does not approve changes | Name the governing Records and your verdict |
| Hiding discarded candidates | The user believes knowledge was saved when it was not | Report each discard and its reason |
| Presenting degraded coverage as complete | Creates false confidence | State provider status and coverage |
| Replying in English to a user who writes in another language | Breaks the conversation | Reply in the user's language; keep identifiers verbatim |
