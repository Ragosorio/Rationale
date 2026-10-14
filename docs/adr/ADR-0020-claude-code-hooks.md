# ADR-0020: Claude Code hooks as the mechanical backstop of the protocol

**Status:** proposed — pending independent cross-review and human approval before `accepted`.
**Date:** 2026-10-14
**Deciders:** Claude Code (analysis and implementation); pending human approval and cross-review by another agent
**Supersedes / Superseded by:** none. It extends ADR-0016 (user-scoped registration) to a project-local, machine-local file.

## Context

The protocol in `CLAUDE.md` and the `rationale` skill are context: the model
may follow them or not. In the reported project, a fifth of the operations
agents opened were never closed, and nothing showed them. Claude Code documents
that instructions are not enforcement and that hooks are the mechanism for
behavior that must happen (code.claude.com/docs/en/memory, /hooks; consulted
2026-10-14).

Impeccable (github.com/pbakaus/impeccable, Apache-2.0), a widely used skill
pack, pairs its skill with SessionStart, PostToolUse, and Stop hooks installed
in `.claude/settings.local.json` with recorded consent; Beads uses a
SessionStart hook to re-prime agents after compaction. Both are author
practice, not measurements.

## Decision

1. `rationale hook <event>` answers four Claude Code hooks from their JSON on
   stdin: `session-start` (canon summary, open operations from earlier
   sessions, new doctor errors), `pre-edit` (once per file and session, a
   reminder to prepare a change governed by pinned, critical, or high rules or
   exceptions), `post-tool` (records the operations the session opened), and
   `stop` (asks once to finalize or abandon an operation the session opened).
2. A hook never blocks an edit (`policy.no-inferred-blocks`) and never fails
   the session: any internal error exits 0 without output. Only Stop may ask
   the agent to continue, once per operation; `--mode remind` only tells the
   person and `--mode off` disables it.
3. `install-agent` (and `init`) write the hooks when Claude Code is detected,
   to `.claude/settings.local.json` — machine-local, outside Git — with the
   installed binary's absolute path, merged with the file's other content.
   `--no-hooks` skips them; `uninstall-agent` removes exactly Rationale's
   entries, recognized by the program they run.

## Evidence

- Integration tests drive each hook with the documented input and check the
  output, idempotent install, coexistence with foreign hooks, removal, and that
  invalid JSON is never rewritten (`src/hooks.rs`, `tests/context_integrity.rs`).
- Claude Code 2.1.296 headless: the SessionStart hook ran and its output was
  accepted; the MCP server connected. The model-dependent hooks were not
  observed live because the CLI's login had expired.
- Install lifecycle under a sandbox HOME (`eval/results/v1.2.0-dev/install-lifecycle.txt`):
  hooks installed with the absolute path, and removed with the file.

## Alternatives considered

- **A Claude Code plugin with `hooks/hooks.json`.** Official and easy to
  disable, but it duplicates the skill and MCP registration that
  `install-agent` already manages, and plugin-provided MCP tool names change
  (`mcp__plugin_<plugin>_<server>__…`). The matcher used here covers both
  names, so a plugin remains possible later.
- **Project `.claude/settings.json` (versioned).** Rejected: teammates without
  Rationale installed would get a failing hook on every event.
- **Blocking edits without a prepared operation (PreToolUse `deny`).**
  Rejected: that is a block on an inference.
- **Instructions only.** The status quo the report measured.

## Consequences

- One more file in the project that Rationale writes and removes; it is not
  versioned, so each person installs it.
- Downgrading to a release without `hook` requires `uninstall-agent` first, or
  Claude Code reports a failing hook on every event (documented in the
  uninstall runbook).

## Risks

- **Hook latency.** `pre-edit` reads the canon on every governed edit
  (≈30 ms with 1,000 extra Records, ≈150 ms with 5,000, synthetic); the
  timeouts are 5–10 s.
- **Noise.** Mitigated by once-per-file and once-per-operation reminders and
  by silence in repositories without Rationale.

## Validation

As in Evidence; the live end-to-end run with a model is pending.

## Revisit trigger

A live run where the Stop request loops or annoys more than it helps; Claude
Code changing the hook schema; or a team preferring the plugin distribution.
