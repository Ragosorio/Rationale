# ADR-0017: Local activity stream and operation snapshots

**Status:** proposed — pending independent cross-review and human approval before `accepted`.
**Date:** 2026-09-12
**Deciders:** Claude Code (analysis and implementation), commissioned by the project owner (Rationale vNext brief, 2026-09-12); pending human approval and/or cross-review by another agent
**Supersedes / Superseded by:** none. It narrows ADR-0012 §Decision 3 for two new emitters — the activity stream and operation snapshots — and retires `RunLog`. While ADR-0012 and this ADR remain `proposed`, this one gains no authority over that one: it declares the tension instead of resolving it silently.

## Context

The vNext brief asks for Rationale to be observable while an agent works: an
activity view with the client, the intent, the target, what was considered and
selected, the packet size, and a graph that reacts to events (`ActivityEvent`).
Phase D's `RunLog` (`runs/vertical-slice.ndjson`) had only latency, revision,
consistency, provider status, and bytes: not enough for any of that.

ADR-0012 §Decision 3 forbids agent prompts and Record content in local telemetry
by default, and its *revisit trigger* requires an explicit decision — not a
silent extension — to record something forbidden. The intent declared in
`prepare_change` resembles a prompt. In addition:

- vNext's operation snapshots (`.rationale-local/operations/`) already store the
  intent, the target, and the selected subgraph, because `finalize_change` and
  the UI need them.
- Pending conflicts (`.rationale-local/conflicts/`) store the two statements in
  tension, because `resolve_conflict` must be able to continue.
- The guard test ADR-0012 §Validation promised ("fails if there is an unbounded
  free-text field") was never written.
- New writers under `.rationale-local/` did not apply the Git exclusion before
  writing; only `init` and `install-agent` installed it (ADR-0014 §Decision 3).

## Decision

1. **Activity stream** in `.rationale-local/activity/<session-id>.ndjson`:
   append-only, one file per process (`rationale serve`, each CLI invocation),
   schema `rationale/activity/1`. Every event carries `schema_version`,
   `session_id`, `seq`, `trace_id`, `operation_id`, `timestamp` (RFC 3339 UTC
   with milliseconds), `actor`, `project`, `kind`, and `payload`. One file per
   session keeps Claude Code, Codex, and Cursor from competing for the same file.
2. **Inventory of allowed content:** identifiers (session, trace, operation,
   Record, and conflict ids; node and edge keys; paths; qualified names; the
   target spec, ≤200 characters), states, counts, latencies, sizes, and the
   client's version and name. **A single free text:** the declared intent, on one
   line and ≤280 characters. **Forbidden:** code and snippets, diffs,
   statements, rationales, evidence, conflict questions, human answers, and
   summaries. The content of a Record or a conflict travels by reference: the UI
   reads it from the canon.
3. **Operation snapshots** (`.rationale-local/operations/<op>.json`): functional
   derived state, not telemetry. They store the subgraph (names, paths, keys,
   roles, Record ids), the selection, the actor, the base revision, and the
   intent; never the target's code. Retention: 200 operations.
4. **Exclusion before writing:** vNext's writers — the activity writer, and the
   pipeline before an operation snapshot or a conflict — install the
   `.rationale-local/` exclusion (ADR-0014) before their first content in each
   project, even when activity is disabled.
5. **Opt-out:** `RATIONALE_ACTIVITY=off` disables the stream and the operation
   snapshots, because both store the intent. They are active by default: the
   activity view is part of the product. Without a snapshot, `finalize_change`
   cannot link the operation and uses the declared base or HEAD.
6. **Retention:** sessions older than 14 days are deleted, and never more than
   500 files remain. Age is used, not only count, so that a burst of short
   sessions (a test suite) does not evict the real history.
7. **`RunLog` is retired.** `review-decisions.ndjson` (the legacy review flow) is
   not changed by this ADR.

## Evidence

- The vNext brief (2026-09-12): the `ActivityEvent` model, required events, and
  an activity view with intent and target.
- `src/activity.rs`: payloads are built only with `activity::payload::*`; the
  test `payloads_are_bounded_and_carry_content_only_by_reference` walks every
  constructor with 10,000-character inputs containing ANSI sequences and fails
  on an unbounded string, control bytes, unbounded lists, or content keys
  (`statement`, `rationale`, `detail`, `question`, `source`…).
- `git_exclusion_is_installed_before_the_first_event` checks that, in a freshly
  initialized repository, `.git/info/exclude` contains `.rationale-local/` and
  that `git status` does not see the activity.

## Alternatives considered

- **Keeping only `RunLog`:** the UI could not show what the agent is doing or
  light up the selected subgraph. Discarded: it contradicts the brief.
- **Including statements in events:** it would duplicate canon content in a
  second, unversioned store, against minimization (v0.5 §4.11). The UI can
  already read the canon.
- **A single `activity.ndjson`:** several agent processes would write at once;
  cross-process locks would have to be coordinated for no reason.
- **Disabled by default (opt-in):** the activity view would be empty in the
  normal flow. Active by default is preferred, with minimal content, local,
  excluded from Git, and with an explicit opt-out.

## Consequences

- The UI observes sessions of several agents without coordination between
  processes.
- Any new event must go through `activity::payload`, so the guard test covers it
  automatically.
- An intent written by a person stays on local disk, bounded, for up to 14 days.

## Risks

- **The intent may contain sensitive text.** Mitigation: one line, ≤280
  characters, local-only, excluded from Git, 14-day retention, and
  `RATIONALE_ACTIVITY=off`.
- **Paths and qualified names reveal the code's structure.** They are already in
  the versioned canon and in Git; events add no code content.
- **Projects with `.rationale-local/` already versioned:** `info/exclude` does
  not untrack files. `install-agent` keeps warning with the remediation command
  (ADR-0014 §Decision 6).

## Validation

Unit tests in `src/activity.rs` (order and `seq`, merging sessions, *tailing*
complete lines, exclusion, opt-out, retention, minimization) and the MCP
integration test of an operation's lifecycle, which checks the sequence of
`prepare_change` and `finalize_change` events.

## Revisit trigger

Reopen if the UI needs free text beyond the intent, if transmitting activity off
the machine is proposed (that also requires ADR-0012's opt-in decision), or if
activity must be shared between clones.
