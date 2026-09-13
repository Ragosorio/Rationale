# Report templates

Default shapes for the reports people read most. Translate the labels into the
user's language. Keep ids, field values, paths, commands, and quoted
statements verbatim. Drop a line that has nothing to report rather than
writing "none" everywhere.

## Preflight verdict

```markdown
**Target:** `src/billing/retry.rs::backoff` · operation `op_1a2b3c`
**Coverage:** provider `successful` / `complete` · snapshot `exact`

| Record | Authority · provenance | Verdict | Why |
|---|---|---|---|
| `constraint.payments-idempotent` | pinned · human_stated | respects | The cap applies before a charge can reach the bank. |
| `decision.retry-cap-five` | normal · agent_asserted | contradicts | The user asked for three attempts; capture will supersede it. |

**Uncertain:** `refunds → charge` is `orphaned`; its explanation may be stale.
**Plan:** Lower the cap to three in `backoff` and update `retry_test.rs`.
```

## Capture report

```markdown
**Written**
- `decision.retry-cap-three` (decision · normal) — bound to `src/billing/retry.rs::backoff`

**Superseded**
- `decision.retry-cap-five` → replaced by `decision.retry-cap-three`

**Discarded**
- Candidate 2 — `transient`: it described this change, not something that stays true.

**Needs a person**
- Consider pinning `decision.retry-cap-three`: `rationale pin decision.retry-cap-three`
```

When nothing durable was learned, say so in one line and state that no
candidates were sent.

## Conflict handoff

```markdown
A candidate tried to replace a pinned rule, so nothing was written. This
decision is yours.

**Conflict:** `conflict_7f3c`
**Pinned rule** (`constraint.payments-idempotent`): "A charge that reached the bank is never retried."
**New assertion:** "Charges may be retried once when the bank response times out."
**Question:** "Which statement should govern src/billing/retry.rs::backoff?"

- `keep_pinned` — the pinned rule stays; the new assertion is dropped.
- `adopt_new` — the new assertion replaces the rule and becomes pinned. Requires your Git identity under `authority:` in `.rationale/config.yaml`.

Reply with your choice, or run `rationale resolve conflict_7f3c keep-pinned` (or `adopt-new`) in your terminal.
```

## Health report

```markdown
**Works:** MCP tools respond · Git revision `4cfddca` · canon readable
**Degraded:** Codebase Memory `unavailable` — symbol bindings and structure are not verified
**Not checked:** Control Room (not started)
**Next:** Install Codebase Memory, then restart the agent.
```
