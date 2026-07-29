<!-- rationale:begin (no editar a mano — `rationale uninstall-agent` lo revierte) -->
## Rationale — protocolo de invocación

Este proyecto usa Rationale (servidor MCP `rationale`) para preservar el
*por qué* del código. Sigue este protocolo:

You are working in a project that uses Rationale for decision context.

Use this protocol at the start of every conversation that may change code:

1. If Codebase Memory is installed, use it first to locate the target symbol,
   its callers, and the relevant files. It tells you where the code is and
   how it connects; it does not decide why the code must remain as it is.
2. Before changing non-trivial code, call Rationale's
   `prepare_change(target, intent)` with the target you found and your actual
   intended change. Read the returned constraints, authority, evidence,
   linkage, provider coverage, and intent conflicts.
3. If the packet reports a governing constraint or a conflict with your
   intent, say so explicitly. Compare the proposed change with the Record;
   do not silently proceed and do not call an undetermined conflict a proven
   semantic contradiction. Ask for clarification when the decision is not
   yours to make.
4. If code looks unnecessarily complex, redundant, or "weird", call
   `explain_target(target)` before simplifying it. The code may be a
   Chesterton fence whose reason lives in the canon.
5. Make the smallest change consistent with the approved context. Keep
   tests, evidence, and the declared project authority in view.
6. After a non-trivial change, run the relevant tests and call
   `finalize_change(...)` so observed facts and the diff become a pending
   proposal when the capture policy requires it.
7. **One decision per Record.** If the work contains several independent
   decisions, write several small Records — not one that covers everything.
   Split when the parts could be accepted, rejected, revoked, or superseded
   separately; when they answer different questions; when they have different
   authority or lifetime; or when a future reader would only need one of them.
   Do not split a single decision into fragments that mean nothing apart.
   When the working tree holds more than one decision, declare
   `governs_paths` on each `finalize_change` call so each Record binds only
   what it governs. Without it, every Record binds every changed file and the
   canon can no longer say which decision governs which code.
8. A proposal is not an approved Record. Never claim that a decision is
   approved until a human has completed `rationale review`.

When Codebase Memory is unavailable, continue with the coverage reported by
Rationale and state that limitation. Never invent a symbol resolution,
authority, approval, evidence, or provider result.

## Pre-made actions

`rationale install-agent` installs six project-scoped Claude Code skills. Type
`/rationale` to filter them in autocomplete:

- `/rationale-preflight <target> <intent>` — locate the target, call
  `prepare_change`, and state constraints or conflicts before editing.
- `/rationale-explain <target>` — call `explain_target` before simplifying a
  possible Chesterton fence.
- `/rationale-capture [statement]` — inject live Git context and guide
  `finalize_change` after the change.
- `/rationale-review` — list pending proposals and hand the interactive
  `rationale review` decision to the human. Agents cannot invoke this skill
  automatically.
- `/rationale-health` — combine MCP `health` with `rationale doctor --check`.
- `/rationale-protocol` — load this full protocol when project instructions
  were not loaded by the client.

The MCP server exposes the same source actions as prompts named `preflight`,
`explain`, `capture`, `review`, `health`, and `protocol`. Prompt discovery and
command decoration belong to each MCP client; do not assume a slash-command
name without verifying that client. In Codex, the portable path is to ask in
plain language, for example: “Prepare this change with Rationale for `<target>`
with intent `<intent>`.”
<!-- rationale:end -->
