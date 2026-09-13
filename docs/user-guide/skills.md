# The `rationale` skill

The protocol in `CLAUDE.md` and `AGENTS.md` tells an agent *that* it should
prepare and capture. The `rationale` skill tells it *how*, in depth, without
loading everything into every conversation. It follows the open
[Agent Skills](https://agentskills.io/specification) format, so the same folder
works in Claude Code, Codex, and other agents that read skills.

## Why a skill

Agents share a finite context window. A skill costs about one description until
a task needs it; then the agent reads `SKILL.md`, and it opens a playbook only
for the operation at hand. That is progressive disclosure, and it lets the skill
carry far more guidance than a protocol block could afford. Every conversation
still has the short protocol; the skill adds depth on demand.

The design follows the published guidance from Anthropic and OpenAI on writing
skills: a description that says what the skill does and when to use it, a
`SKILL.md` well under 500 lines that routes instead of explaining everything,
references one level deep with a table of contents when they grow, explicit
checklists for fragile workflows, deterministic scripts for checks that code can
do exactly, and evals written before instructions change.

## Layout

```text
skills/rationale/
├── SKILL.md                      router: language rule, operations, the loop, invariants, tools
├── agents/openai.yaml            Codex display metadata and the MCP dependency
├── references/
│   ├── preflight.md              locate, prepare, state a verdict, decide
│   ├── packet.md                 every field of the prepare_change packet
│   ├── explain.md                Chesterton's fence before simplifying
│   ├── capture.md                separate durable knowledge, call finalize_change, report
│   ├── records.md                kinds, statements, rationales, bindings, supersedes, discard reasons
│   ├── conflicts.md              hand a pinned-rule conflict to a person
│   ├── health.md                 what works, what is degraded, what was not checked
│   ├── adopt.md                  set up a repository and seed the first Records
│   ├── maintain.md               stale bindings, doctor findings, outdated Records
│   ├── concepts.md               vocabulary
│   ├── cli.md                    which commands an agent may run and which belong to people
│   └── anti-patterns.md          failures to check before reporting
├── scripts/check_candidates.py   validator that mirrors the capture gate
├── assets/
│   ├── candidates.template.json  starting point for finalize_change candidates
│   └── report-templates.md       preflight verdict, capture report, conflict handoff, health report
└── evals/                        trigger and behavior evals for maintainers (not installed)
```

## Operations

| Operation | When the agent uses it | Playbooks |
|---|---|---|
| `preflight` | Before changing, moving, or deleting non-trivial code | `preflight.md`, `packet.md` |
| `explain` | Code looks redundant or odd; someone asks why it exists | `explain.md` |
| `capture` | A change is done and tested | `capture.md`, `records.md` |
| `conflicts` | `finalize_change` returned conflicts | `conflicts.md` |
| `health` | Tools are missing or results look degraded | `health.md` |
| `adopt` | Setting Rationale up and seeding the canon | `adopt.md`, `records.md` |
| `maintain` | Stale bindings, `doctor` findings, orphaned relationships | `maintain.md` |

## Install

| Method | Where it goes | Notes |
|---|---|---|
| `rationale install-agent` | `.claude/skills/rationale/` and `.agents/skills/rationale/` | Since v1.1.0; `rationale init` runs it. Every file is recorded with a hash; edited files are kept on reinstall and uninstall |
| `npx skills add Ragosorio/Rationale` | The directories of the agents it detects | For agents Rationale does not configure. The `skills` CLI links agent directories to one copy |
| Manual copy | Any skills directory your agent reads | Copy `skills/rationale/` without `evals/` |

When `.claude/skills/rationale/` is a symbolic link created by another tool,
`install-agent` reports it and leaves it alone.

## Invoke it

- **Automatically.** Claude Code and Codex select the skill when a task matches
  its description: changing non-trivial code in a repository with `.rationale/`,
  asking why code exists, finishing a change, or diagnosing Rationale.
- **Claude Code.** `/rationale`, or `/rationale <operation>`.
- **Codex.** `$rationale`, or `$rationale <operation> <target>`.

The five Claude Code shortcuts (`/rationale-preflight`, `/rationale-explain`,
`/rationale-capture`, `/rationale-conflicts`, `/rationale-health`) stay
available for people. Agents do not invoke them on their own, so there is one
clear entry point for automatic selection.

## Language

The skill is written in English and tells the agent to reply in the language the
user writes in. Tool names, Record ids, field values, paths, commands, and
quoted Record statements stay verbatim. New Records use the language the
existing canon uses, so duplicate detection keeps working and the canon reads
as one voice; an empty canon takes the user's language.

## The candidate validator

`scripts/check_candidates.py` checks candidates before `finalize_change` with
the gate's own rules: kinds, durability, text thresholds, restated rationales,
mechanical noise, severity, id format, bindings to real files, relationship
kinds, duplicates against the local canon, and `supersedes` that would become a
conflict.

```bash
python3 .claude/skills/rationale/scripts/check_candidates.py - <<'JSON'
[{"kind": "constraint", "statement": "...", "rationale": "...",
  "durability": "durable", "bindings": ["src/billing/retry.rs::backoff"]}]
JSON
```

| Exit code | Meaning |
|---|---|
| `0` | No candidate would be discarded (warnings may remain) |
| `2` | At least one candidate would be discarded |
| `1` | The input could not be read |

The gate stays the authority. A Rust test fails if the validator's keyword
lists or thresholds drift from `src/canon.rs`.

## Customize it

Edits to installed skill files survive `install-agent`: each file keeps its own
hash, so your edit is preserved and the rest of the skill still updates. An
edited file no longer receives updates until you run
`rationale install-agent --refresh-skills`, which does not touch proven edits
and adopts files with unknown provenance.

To add your team's conventions, prefer a separate skill of your own that tells
agents when to apply them and points to the Rationale operations it builds on.
That keeps your knowledge independent of Rationale's release cycle.

## Evals

`skills/rationale/evals/` is for maintainers and is never installed:

- `triggers.csv` — explicit, implicit, contextual, boundary, and negative
  prompts in English and Spanish, with whether the skill should load.
- `scenarios.json` — realistic situations with expected and forbidden
  behavior, such as a pinned constraint, a conflict handoff, a degraded
  provider, or a user writing in Spanish.
- `fixtures/` — inputs for the validator.

Run every scenario with and without the skill, on each host you support, and
keep a change only when it beats the baseline. The procedure is in
[`skills/rationale/evals/README.md`](../../skills/rationale/evals/README.md).
