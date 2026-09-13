---
name: rationale
description: Keeps the why of code with Rationale. Reads the rules, decisions, and explained relationships that govern code before an agent changes it, and writes back only durable knowledge afterwards. Use in a repository with a `.rationale/` directory or the `rationale` MCP server whenever you change, refactor, simplify, or delete non-trivial code; when someone asks why code exists or whether it can go; after finishing a change; when `finalize_change` returns conflicts; or when installing, adopting, or diagnosing Rationale. Not for plain Git history or code search.
license: MIT
compatibility: Needs the rationale MCP server (`rationale serve`). Codebase Memory is optional. The candidate validator needs Python 3.
metadata:
  author: Ragosorio
  source: https://github.com/Ragosorio/Rationale
---

# Rationale

Rationale is causal memory for coding agents. Git records what changed;
Rationale keeps why it still matters. Before you change code, it hands you the
Records that govern it. After the change, you send back what stays true, and
Rationale writes it to the canon (`.rationale/records/`) in the same call.
People keep authority over the rules they pin.

## Language

Reply in the language the user writes in, and switch when they switch. Keep
these verbatim in every language: tool and argument names, Record ids, field
values such as `pinned` or `agent_asserted`, paths, symbols, commands, and
quoted Record statements.

Write new Record statements and rationales in the language the existing Records
use (read two or three in `.rationale/records/`). In an empty canon, use the
user's language. One language per canon keeps duplicate detection working and
lets every reader follow the whole canon.

## Choose the operation

Match the request to one operation and read only the files listed for it. Paths
are relative to this skill's directory.

| Operation | Use when | Read |
|---|---|---|
| `preflight` | You are about to change, move, or delete non-trivial code | [references/preflight.md](references/preflight.md), [references/packet.md](references/packet.md) |
| `explain` | Code looks redundant, odd, or over-built; someone asks why it exists | [references/explain.md](references/explain.md) |
| `capture` | A change is done and tested, or the task is ending | [references/capture.md](references/capture.md), [references/records.md](references/records.md) |
| `conflicts` | `finalize_change` returned `conflicts`, or the user asks about them | [references/conflicts.md](references/conflicts.md) |
| `health` | Tools are missing, results look degraded, or the user asks whether it works | [references/health.md](references/health.md) |
| `adopt` | Setting Rationale up in a repository or seeding its first Records | [references/adopt.md](references/adopt.md), [references/records.md](references/records.md) |
| `maintain` | Stale bindings, `doctor` findings, orphaned relationships, outdated Records | [references/maintain.md](references/maintain.md) |

When the user names an operation (`/rationale capture`, `$rationale explain
src/auth.rs::resolve`), run that one. A normal coding task is `preflight`, then
the change, then `capture`.

Supporting references, loaded on demand:

- [references/concepts.md](references/concepts.md) when a term is unfamiliar.
- [references/cli.md](references/cli.md) before running any `rationale` command.
- [references/anti-patterns.md](references/anti-patterns.md) to review your own work before reporting it.
- [assets/report-templates.md](assets/report-templates.md) for the preflight verdict, capture report, conflict handoff, and health report.
- [assets/candidates.template.json](assets/candidates.template.json) as a starting point for `finalize_change` candidates.

## The loop

1. **Locate.** Find the real target, a file or `path::symbol`, with Codebase
   Memory when it is available and with search otherwise. Structure tells you
   where code is; it never decides why the code must stay.
2. **Prepare.** Call `prepare_change(target, intent)` with the change you
   actually intend. Keep the `operation_id`.
3. **State the verdict.** For each governing Record and intent conflict, say
   whether your intent respects it, contradicts it, or is undetermined, before
   the first edit.
4. **Change.** Make the smallest change consistent with the packet and run the
   relevant tests.
5. **Capture.** Call `finalize_change` with the `operation_id`, a short
   `summary`, and only the knowledge that stays true after the change. When
   nothing durable was learned, send no candidates.
6. **Report.** Say what was written, what was discarded and why, what was
   superseded, and what needs a person.

Trivial edits that no Record could govern (a typo, formatting, a comment) skip
the loop.

## Invariants

These hold in every operation. Each protects something the project depends on.

- **A `pinned` Record is the project's rule.** Do not work around it or try to
  supersede it quietly: the attempt becomes a conflict a person must decide.
  When the task requires breaking a pinned rule, stop and ask.
- **People decide conflicts, pins, and authority.** `rationale pin`, `unpin`,
  `resolve`, and `review-record` belong to people at an interactive terminal.
  Call `resolve_conflict` only after an explicit answer, with the person's
  literal words in `human_answer`, because the canon records it as their
  decision.
- **Provenance stays honest.** Never present an agent's assertion as something a
  person stated, and never invent authority, evidence, symbol resolutions, or
  provider results.
- **Uncertainty is reported, not resolved.** `orphaned` and `unknown`
  relationships, degraded coverage, and lexical intent conflicts are signals to
  state. They are not proof that an explanation is wrong or that a
  contradiction exists.
- **Memory is durable knowledge.** What changed belongs in Git and in
  `summary`. Why the code must stay this way belongs in a candidate.
- **The canon changes only through the gate.** Write Records with
  `finalize_change`, never by editing `.rationale/` files.

## Tools

The `rationale` MCP server provides five tools. Hosts may add a prefix, such as
`mcp__rationale__prepare_change` in Claude Code.

| Tool | Purpose |
|---|---|
| `prepare_change` | Opens an operation and returns the packet for one target and intent |
| `explain_target` | Records that govern a target by exact binding, and what is unknown |
| `finalize_change` | Closes the operation; the capture gate writes durable candidates as Records |
| `resolve_conflict` | Applies a person's decision on a conflict with a pinned Record |
| `health` | Git revision, working tree, provider status, and coverage |

When none of these tools is available, follow
[references/health.md](references/health.md) and tell the user. Never describe
results you did not receive.

## Validate candidates before sending them

The validator mirrors the gate's rules and checks bindings, ids, `supersedes`,
and likely duplicates against the local canon. Run it from the repository root,
passing the candidates on stdin so no stray file lands in the diff:

```bash
python3 <skill-dir>/scripts/check_candidates.py - <<'JSON'
[ { "kind": "constraint", "statement": "...", "rationale": "...",
    "durability": "durable", "bindings": ["src/billing/retry.rs::backoff"] } ]
JSON
```

`<skill-dir>` is where this file lives, usually `.claude/skills/rationale` or
`.agents/skills/rationale`. Exit code `0` means no candidate would be
discarded, `2` means at least one would be (fix it and run again), and `1`
means the input could not be read. The gate remains the authority. Without
Python, use the checklist in [references/records.md](references/records.md).
