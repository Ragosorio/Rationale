You are working in a project that uses Rationale to keep the *why* of its code:
the constraints, decisions, risks, and exceptions that explain why the code is
the way it is. Rationale serves them through its MCP server (`rationale`) and
stores them as Records in `.rationale/`, versioned with the code.

Reply in the language the user writes in. Keep tool names, arguments, Record
ids, field values, paths, and commands exactly as they are.

Use this protocol at the start of every conversation that may change code:

1. If Codebase Memory is installed, use it first to locate the target symbol,
   its callers, and the relevant files. It tells you where the code is and
   how it connects; it does not decide why the code must remain as it is.
2. Before changing non-trivial code, call Rationale's
   `prepare_change(target, intent)` with the target you found and your actual
   intended change. Keep the returned `operation_id`. Read the constraints
   and decisions that govern the target, the explained relationships with
   their structural state (`observed`, `indirect`, `orphaned`, `unknown`),
   risks, unknowns, provider coverage, and intent conflicts.
3. If the packet reports a governing Record or a conflict with your intent,
   say so explicitly. Compare the proposed change with the Record; do not
   silently proceed and do not call an undetermined conflict a proven semantic
   contradiction. A `pinned` Record is a rule the project fixed: do not work
   around it. Ask for clarification when the decision is not yours to make.
4. If code looks unnecessarily complex, redundant, or "weird", call
   `explain_target(target)` before simplifying it. The code may be a
   Chesterton fence whose reason lives in the canon.
5. Make the smallest change consistent with that context. Keep tests,
   evidence, and the declared project authority in view.
6. After a non-trivial change, run the relevant tests and call
   `finalize_change` with the `operation_id`, a short `summary`, and
   `candidates`: only knowledge that will stay true after this change — why
   the code is the way it is and what must be preserved. Write statements and
   rationales in the language the existing Records use. Rationale discards
   noise and duplicates and writes the rest as canonical Records in the same
   call; there is no approval queue. When nothing durable was learned, send no
   candidates. Report what was written and what was discarded, with reasons.
7. **One decision per Record.** If the work contains several independent
   decisions, send several small candidates — not one that covers everything.
   Split when the parts could be replaced or revoked separately; when they
   answer different questions; when they have a different lifetime; or when a
   future reader would only need one of them. Do not split a single decision
   into fragments that mean nothing apart. Bind each candidate only to the
   code it governs, and name the Records it replaces in `supersedes`.
8. If `finalize_change` returns `conflicts`, a candidate tried to replace a
   pinned Record and was not written. Stop, show the human both statements and
   the question, and only after their explicit answer call
   `resolve_conflict(conflict_id, keep_pinned | adopt_new, human_answer)`.
   Never decide for them, never pin or unpin a Record yourself, and never
   present what an agent asserted as something a human stated.

When Codebase Memory is unavailable, continue with the coverage reported by
Rationale and state that limitation. Never invent a symbol resolution,
authority, human decision, evidence, or provider result.

## The rationale skill

`rationale install-agent` installs the `rationale` skill next to this
protocol: a playbook for each operation (preflight, explain, capture,
conflicts, health, adopt, maintain), a guide to writing Records the capture
gate keeps, report templates, and a validator for candidates. It lives in
`.claude/skills/rationale/` for Claude Code and `.agents/skills/rationale/`
for Codex; other agents can install it with
`npx skills add Ragosorio/Rationale`. Load it when this protocol does not
answer your question.

## Shortcuts

In Claude Code, `/rationale` loads the skill, optionally with an operation
(`/rationale capture`). People can also type these shortcuts; agents do not
invoke them on their own:

- `/rationale-preflight <target> <intent>` — locate the target, call
  `prepare_change`, and state governing Records or conflicts before editing.
- `/rationale-explain <target>` — call `explain_target` before simplifying a
  possible Chesterton fence.
- `/rationale-capture [statement]` — inject live Git context and close the
  change with durable candidates.
- `/rationale-conflicts` — list pending conflicts with pinned Records and hand
  the decision to the human.
- `/rationale-health` — combine MCP `health` with `rationale doctor`.

The MCP server exposes the same actions as prompts named `preflight`,
`explain`, `capture`, `conflicts`, `health`, and `protocol`. Prompt discovery
and command decoration belong to each MCP client; do not assume a
slash-command name without verifying that client. In Codex, invoke the skill
with `$rationale`, or ask in plain language, for example: “Prepare this change
with Rationale for `<target>` with intent `<intent>`.”

People follow agent work live with `rationale ui`, pin the rules no agent may
replace with `rationale pin <record-id>`, and decide conflicts with
`rationale resolve`.
