# Daily workflow

## Before the change

The agent prepares the target with its real intent. From the CLI you can see
the same packet:

```bash
rationale prepare "src/auth/authorization.rs::resolve" --intent "change how permissions are resolved"
```

The JSON packet goes to stdout and diagnostics to stderr. Look above all at:

- `retrieval.status` and `retrieval.scope`: what kind of answer it is;
- `critical_constraints` and `decisions`, with their `authority`, `provenance`, and `why`;
- `inherited_constraints`, invariants reaching the target through code it uses;
- `history`, Records that no longer govern (never applied as current);
- `relationships`, with their state (`observed`, `indirect`, `orphaned`, `unknown`);
- `intent_conflicts` — a lexical signal, not a proven contradiction;
- coverage `snapshot` and `warnings`, `omitted` (what the budget left out and
  how to get it), and `budget_overflow` when it appears.

A folder works as a target (`rationale prepare src/billing`): its packet lists
the Records bound inside it, highest priority first.

Keep the `operation_id`: `finalize_change` uses it to close the operation.

In Claude Code, `/rationale` or `/rationale-preflight <target> <intent>` drives
this step explicitly; in Codex, `$rationale`. The skill's
[packet reference](../../skills/rationale/references/packet.md) explains every
field.

## During the change

If the code looks strange, the agent calls `explain_target` before simplifying
it. It never treats a provider inference as authority.

## After the change

The agent calls `finalize_change` with the `operation_id`, a `summary`, and the
`candidates`: only knowledge that will stay true. Rationale discards noise,
transient notes, rationales that repeat the statement, and duplicates — always
with a reason — and writes the rest as Records in the same call. Without
candidates, no memory is written.

Review what was captured the way you review code: `.rationale/records/` travels
in the same pull request.

If the agent prepared a change and did not make it, it closes the operation
with `finalize_change(operation_id, abandon=true, summary="why")`: no memory,
and never recorded as finalized. `rationale operations` lists the ones left
open; in Claude Code a Stop hook asks the agent to close the ones its session
opened.

## While you work

```bash
rationale ui
```

The [Control Room](control-room.md) shows each operation live: what the agent
received, what it captured, what it discarded, and which explanations are at
risk.

## Human authority

Pin a rule that no agent may replace:

```bash
rationale pin <record-id> --reason "payments invariant"
```

If an agent tries to replace it, `finalize_change` returns a conflict and writes
nothing. You decide:

```bash
rationale conflicts
rationale resolve <conflict-id> keep-pinned    # or adopt-new, with declared authority
```

To correct, dispute, revoke, supersede, change the authority of, or add evidence
to an existing Record:

```bash
rationale review-record <record-id>
```

State a rule yourself, with your name on it (provenance `human_stated`):

```bash
rationale rule add constraint "Amounts are integer cents from quote to invoice." \
  --why "Float rounding double-charged customers in an incident." \
  --bind src/shipping --bind src/billing --pin
```

All these actions require an interactive terminal and leave auditable lifecycle
events.

## Keeping the canon healthy

```bash
rationale doctor                    # findings with severity and a fix each
rationale doctor --update-baseline  # accept today's findings as known debt (commit the file)
rationale doctor --check            # in CI: fails only on new errors
```

When bindings go stale after a refactor, ask the agent to run the skill's
`maintain` operation: it supersedes outdated Records through the capture gate
and leaves pinned Records and lifecycle changes to you.

When a module is removed, its Records keep describing it as current until
someone retires them. `doctor` groups them into one `retired-scope` finding;
`retire` plans the repair and applies it only when you say so:

```bash
rationale retire src/comunicaciones --reason "Replaced by src/notifications"   # plan only
rationale retire src/comunicaciones --reason "Replaced by src/notifications" --apply
rationale retire --undo <batch-id>                                             # if you change your mind
```

Records bound only to the removed code are retired, Records also bound to
current code lose only the removed bindings, pinned Records are left for you,
and the whole change is a reviewable `git diff` of `.rationale/`.

## Projects that predate 1.0

If `.rationale/proposals/` still has pending proposals:

```bash
rationale migrate --dry-run
rationale migrate
```

Valid proposals become Records with `migrated` provenance; noisy ones are
archived with their reason in `.rationale/archive/proposals/`. Nothing is
deleted. `rationale review` is still available to confirm them one by one if
you prefer.
