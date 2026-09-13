# AGENTS.md

Operating instructions for any agent (Claude Code, Codex, or another) working in this repository. This document is short on purpose: it does not repeat the architecture or the full process; it only says where each thing lives and what to do first.

Reply to the person in the language they write in. Documentation, code comments you add to docs, and agent-facing text in this repository are written in English; Spanish lives only in the website's `/es/` pages under `site/`.

## What this is and where it stands

Rationale completed phases A–G and published the stable 1.0 series: autonomous capture with human authority over pinned Records, the vNext context compiler, the normalized structural provider, local activity, the Control Room, and five MCP tools (`prepare_change`, `explain_target`, `finalize_change`, `resolve_conflict`, `health`). The real core lives in `src/`; `spikes/language/` remains throwaway research. See [`docs/architecture/code-map.md`](docs/architecture/code-map.md) and [`docs/work-items/v1.0-release-verification.md`](docs/work-items/v1.0-release-verification.md).

`main` also carries unreleased work listed under *Unreleased* in [`CHANGELOG.md`](CHANGELOG.md), most notably the `rationale` Agent Skill in [`skills/rationale/`](skills/rationale/), embedded in the binary by `src/skill_bundle.rs` and installed for Claude Code and Codex.

The recorded ADRs include twelve proposals and one open partial decision
(ADR-0011); none is self-approved (`evaluation.no-self-certification`). Before
assuming one describes current behavior, check its status in
`docs/adr/index.md`.

F8 closed the P1/P2 findings of the adversarial audit. The vNext dogfood and the
stable release are recorded in `docs/work-items/`; ADRs keep their individual
status and none is self-approved.

Next: use 1.0 in real projects, fix their stale bindings with evidence, and run
`doctor --check`; see `docs/work-items/` for current limits and plans.

## Reading route by task

Do not read the three foundational documents in full for a trivial task. Read according to what you are going to do:

| Task | Read |
|---|---|
| Minor documentation change, typo fix, template tweak | This file only |
| Understand what Rationale is, what problem it solves, the data model | `Rationale_v0.5.md` |
| Decide or touch technical boundaries, components, provider integration | `Rationale_Arquitectura_Conceptual_v0.1.md` |
| Coordinate work between agents, roles, cross-review, work items | `Rationale_Proceso_Construccion_Agentes_v0.1.md` |
| Investigate Codebase Memory | `docs/research/codebase-memory/` + `Rationale_Arquitectura_Conceptual_v0.1.md §6-7` |
| Reopen or question the language choice | `docs/adr/ADR-0001-core-language.md` + `docs/research/language/` — **the ADR is `proposed`, not `accepted`; do not self-approve it** |
| Write Rust code for the core | `docs/rust/style-guide.md`, `testing-guide.md`, `security-guide.md` |
| Create or review a Rationale Record or Subject | `Rationale_v0.5.md §5, §9, §10` + `.rationale/` + `skills/rationale/references/records.md` |
| Change the agent skill, pre-made actions, or MCP tool descriptions | `docs/user-guide/skills.md`, `skills/rationale/`, `src/prompts.rs`, `src/skill_bundle.rs`, `src/mcp/server.rs` |
| Change the website | `site/README.md` and the Records bound to `site/` |
| Design the validation experiment | `Rationale_v0.5.md §30.1` |

For any non-trivial task, always read the relevant ADR in `docs/adr/` before acting.

## Session start protocol

```bash
git rev-parse --show-toplevel
git status --short
git branch --show-current
git rev-parse HEAD
```

Then: read this file, identify the reading route for your task, check `docs/work-items/` for work in progress, query Codebase Memory if your task touches more than one module (see below), and state in your plan what you will change, what you will not change, which tests you will run, and which decisions could be affected.

## Codebase Memory

Codebase Memory already indexes this repository and others (`Monorepo`, the local clone of `codebase-memory-mcp`) through the `mcp__codebase-memory-mcp__*` tools. Use it before touching several modules, contracts, storage, MCP, review, providers, security, packaging, or a critical Subject (`Proceso §7.2`). Never treat its output as absolute truth: every answer must state coverage, revision, and warnings (`Proceso §7.4`).

## Rust (proposed core language, ADR-0001)

```bash
export PATH="$HOME/.cargo/bin:$PATH"   # needed in non-interactive shells — see docs/environment/
cargo fmt --check                       # before any commit with Rust code
cargo clippy --all-targets -- -D warnings
cargo test
```

Full guides: [`docs/rust/style-guide.md`](docs/rust/style-guide.md), [`docs/rust/testing-guide.md`](docs/rust/testing-guide.md), [`docs/rust/security-guide.md`](docs/rust/security-guide.md). The code in `spikes/language/rust/` is throwaway research (a spike), not the starting point of the real core.

## Source hierarchy

When sources conflict, in this order (`Proceso §2`):

```
1. Tests and reproducible behavior
2. Rationale_v0.5.md
3. Approved ADRs
4. Current conceptual architecture
5. Approved Rationale Records
6. Verified research notes
7. The issue or task plan
8. Code comments
9. Agent inferences
```

An inference never silently overrides an approved decision.

## What NOT to do (`Rationale_Arquitectura_Conceptual_v0.1.md §27`)

- Choose a language without an ADR.
- Copy Codebase Memory internals.
- Read its private SQLite database as a contract.
- Introduce a SaaS.
- Add mandatory remote embeddings.
- Create twenty services.
- Create a daemon before measuring the need.
- Block changes with inferences.
- Approve Records automatically.
- Hide partial coverage.
- Declare success based only on the same agent's opinion.
- Skip documentation.
- Change the architecture without an ADR.
- Package before validating the core.
- Optimize before instrumenting.

The original list also said not to start the landing page; that applied until the
core was validated, and the website now lives in `site/`.

## Roles and cross-review

This project is built with **Claude Code and Codex in cross-review** (`Proceso §13`): one agent implements, the other tries to falsify the proposal (looking for counterexamples, race conditions, stale revisions, false confidence, hidden cost, cross-platform problems, missing tests or docs), and the human approves or rejects. For critical changes, the minimum separation is Research/Plan → Implementation → Independent Review → Evaluation → human approval (`Proceso §5`). The same agent may implement and self-review; that does not replace independent review.

Possible roles (`Proceso §4`): Research, Architecture, Implementation, Review, Evaluation, Documentation. An agent may take several in sequence, but must say so.

### Meeting technical conditions is not authorization

An approved plan that says "when validation passes, merge" describes an order;
**it does not grant permission in advance**. Meeting the technical condition
hands the decision back to the human; it does not execute it for them. Before
merging to `main`, publishing, or modifying any real repository — the project's
or a pilot's — you need explicit authorization **given after** the evidence, for
that specific action. An authorization does not extend to the next step or the
next repository.

The real finding that motivated this rule (2026-07-28, pre-beta dogfood): after
closing validation #1 of ADR-0015, the agent ran `merge --ff-only` to `main` on
its own, reading the agreed plan as standing authorization. The resulting state
was correct and was not reverted, but the decision was not the agent's to make.
The risk is not the merge itself: the same reasoning applied one step later would
have touched repositories with a remote.

## Branch and commit conventions

```
research/<topic>
spike/<topic>
feature/<topic>
fix/<topic>
docs/<topic>
release/<version>
```

Coherent commits that pass the relevant tests, without unrelated refactors, with a causal message (e.g. `feat(revision): reject exact context when provider is behind`).

## Quality gates and Definition of Done

A work item is not complete if anything applicable is missing (`Proceso §16`):

```
[ ] Code
[ ] Tests
[ ] Formatter
[ ] Lint
[ ] Security check
[ ] Documentation
[ ] ADR (if an architectural decision was made)
[ ] Research artifact (if there was research)
[ ] Metrics
[ ] Independent review
[ ] Reindex verification (Codebase Memory)
[ ] Rationale finalize
[ ] Clean git status
```

Nothing important may live only in an agent's conversation (`Proceso §1`). Every decision ends up in code, a test, a document, an ADR, a research note, an experiment result, or a Rationale Record.

If an agent finds a contradiction: stop the affected assumption, record evidence, create a work item, reproduce it, identify affected documents, propose an ADR — never hide it or resolve it silently (`Proceso §25`).

If an agent does not know something: write `Unknown` + `Evidence` + `Risk` + `Next experiment` — never fill the gap with a convincing explanation (`Proceso §26`).

## Links

- [`Rationale_v0.5.md`](Rationale_v0.5.md)
- [`Rationale_Arquitectura_Conceptual_v0.1.md`](Rationale_Arquitectura_Conceptual_v0.1.md)
- [`Rationale_Proceso_Construccion_Agentes_v0.1.md`](Rationale_Proceso_Construccion_Agentes_v0.1.md)
- [`docs/adr/index.md`](docs/adr/index.md)
- [`docs/work-items/EPIC-CBM-ANALYSIS.md`](docs/work-items/EPIC-CBM-ANALYSIS.md)
- [`docs/research/codebase-memory/`](docs/research/codebase-memory/)
- [`docs/research/language/spike-protocol.md`](docs/research/language/spike-protocol.md)
- [`docs/user-guide/skills.md`](docs/user-guide/skills.md)

<!-- rationale:begin (managed by Rationale; `rationale uninstall-agent` removes this block) -->
## Rationale — invocation protocol

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
<!-- rationale:end -->
