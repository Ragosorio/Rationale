# Daily workflow

## Before the change

The agent prepares the target with its real intent. From the CLI you can see
the same packet:

```bash
rationale prepare "src/auth/authorization.rs::resolve" --intent "change how permissions are resolved"
```

The JSON packet goes to stdout and diagnostics to stderr. Look above all at:

- `critical_constraints` and `decisions`, with their `authority` and `provenance`;
- `relationships`, with their state (`observed`, `indirect`, `orphaned`, `unknown`);
- `intent_conflicts` — a lexical signal, not a proven contradiction;
- coverage `snapshot` and `warnings`, and `budget_overflow` when it appears.

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

All these actions require an interactive terminal and leave auditable lifecycle
events.

## Keeping the canon healthy

```bash
rationale doctor --check
```

Run it in CI or before a release. When bindings go stale after a refactor, ask
the agent to run the skill's `maintain` operation: it supersedes outdated
Records through the capture gate and leaves pinned Records and lifecycle
changes to you.

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
