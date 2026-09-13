---
lang: en
slug: skill
title: The rationale skill
description: One Agent Skill that teaches your agent every Rationale operation — loaded only when a task needs it, in the same folder for Claude Code and Codex.
section: Start
order: 4
---

## What it adds

The [master prompt](/docs/prompt-master) in `CLAUDE.md` and `AGENTS.md` tells
an agent *that* it must prepare and capture. The `rationale` skill tells it
*how*: which fields of the packet matter, how to state a verdict on a governing
Record, how to write a candidate the capture gate keeps, and when to stop and
hand a decision to you.

Agents share a finite context window, so the skill is built for progressive
disclosure. Until a task needs it, the agent sees only its description. When
the task matches, it reads a short `SKILL.md`, which routes to the one playbook
for the operation at hand.

## Operations

| Operation | When the agent uses it |
| --- | --- |
| `preflight` | Before it changes, moves, or deletes non-trivial code |
| `explain` | When code looks redundant or odd, or you ask why it exists |
| `capture` | When a change is done and tested |
| `conflicts` | When a candidate collides with a pinned rule |
| `health` | When tools are missing or results look degraded |
| `adopt` | When setting Rationale up and seeding the first Records |
| `maintain` | When bindings go stale or `rationale doctor` reports findings |

A normal coding task is `preflight`, then the change, then `capture`.

## Install

| Method | Where it goes | Availability |
| --- | --- | --- |
| `rationale install-agent` | `.claude/skills/rationale/` and `.agents/skills/rationale/` | Since v1.1.0; `rationale init` runs it |
| `npx skills add Ragosorio/Rationale` | The skills directories of the agents it detects | For agents Rationale does not configure |
| Manual copy | Any skills directory your agent reads | Copy `skills/rationale/` without `evals/` |

`install-agent` records a hash for every file it writes: a file you edit is
kept on reinstall and uninstall, and a skill directory that another tool
created as a symbolic link is left alone. Updating from v1.0.0 retires the
`/rationale-protocol` skill in favor of `/rationale`, unless you edited it.

## Invoke it

- **On its own.** Claude Code and Codex can load the skill when a task matches
  its description: changing non-trivial code in a repository with
  `.rationale/`, asking why code exists, finishing a change, or diagnosing
  Rationale.
- **Claude Code.** `/rationale`, or `/rationale <operation>`, for example
  `/rationale capture`.
- **Codex.** `$rationale`, or `$rationale explain src/billing/retry.rs::backoff`.

The shortcuts `/rationale-preflight`, `/rationale-explain`,
`/rationale-capture`, `/rationale-conflicts`, and `/rationale-health` stay
available for you. Agents do not pick them on their own, so the skill is the
one entry point for automatic selection.

## Your language

The skill is written in English and tells the agent to answer in the language
you write in. Tool names, Record ids, field values such as `pinned`, paths,
commands, and quoted Record statements stay verbatim in every language.

New Records follow the language your canon already uses, so duplicate
detection keeps working and the whole canon reads as one voice. An empty canon
takes your language.

## Checked before it writes

The skill ships `scripts/check_candidates.py`, a validator that applies the
capture gate's own rules before `finalize_change`: kinds, durability, text
thresholds, restated rationales, mechanical noise, bindings to real files,
likely duplicates, and `supersedes` that would become a conflict.

```bash
python3 .claude/skills/rationale/scripts/check_candidates.py - <<'JSON'
[{"kind": "constraint", "statement": "...", "rationale": "...",
  "durability": "durable", "bindings": ["src/billing/retry.rs::backoff"]}]
JSON
```

Exit code `0` means no candidate would be discarded, `2` means at least one
would be, and `1` means the input could not be read. The gate remains the
authority; a test in the repository fails if the validator drifts from it.

## What is inside

```text
skills/rationale/
├── SKILL.md              router: language, operations, the loop, invariants, tools
├── agents/openai.yaml    Codex display metadata and the MCP dependency
├── references/           one playbook per operation, plus Records, packet, CLI, anti-patterns
├── scripts/              check_candidates.py
├── assets/               report templates and a candidates template
└── evals/                trigger and behavior evals for maintainers (not installed)
```

## Customize it

Edits to installed files survive `install-agent`, but an edited file stops
receiving updates until you run `rationale install-agent --refresh-skills`. To
add your team's conventions, prefer a separate skill of your own that points to
the Rationale operations it builds on, so it stays independent of Rationale's
release cycle.

The full guide, including how the evals are run, is in the repository at
`docs/user-guide/skills.md`.
