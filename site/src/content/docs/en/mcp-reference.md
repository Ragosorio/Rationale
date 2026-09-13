---
lang: en
slug: mcp-reference
title: MCP reference
description: The five tools and six prompts at the agent boundary — inputs, outputs, and what MCP will never decide.
section: Operate
order: 8
---

## `health`

Project identity, Git revision, working tree, provider status, and coverage. A
missing Codebase Memory is an explicit degraded result, never invented
coverage.

## `prepare_change`

Input: `target` (`path` or `path::symbol`) and the agent's real `intent`.
Optional: `max_tokens` (default 2400), `max_nodes` (12), `max_relationships`
(16), `mode: baseline` to skip intent-conflict detection.

Output: an `operation_id` and a packet with

- `critical_constraints` and `decisions` that govern the target, each with
  `authority`, `provenance`, and how it matched;
- `relationships` explained by Records, with derived `state` and the path when
  `indirect`;
- `structure`: the selected nodes and edges around the target, each tagged with
  its role and any explaining Record ids;
- `relevant_code`, `intent_conflicts`, `known_risks`, `known_unknowns`,
  `warnings`, the consistency `snapshot`, and `budget_overflow` when
  authoritative context exceeded the ceiling.

The budget is a ceiling, never a target: an empty neighborhood produces a small
packet, and governing knowledge is never cut to fit.

## `explain_target`

Why a target exists: the Records that govern it by exact binding, its Subject,
and what is known versus unknown. It uses the same matcher as `prepare_change`,
so the two tools never disagree.

## `finalize_change`

Input: `operation_id`, `summary`, and `candidates`. Each candidate requires
`kind`, `statement`, `rationale`, `durability`, and `bindings`; it may add
`supersedes`, `severity`, `id`, `risks`, `evidence`, and `subject`. The diff
base is the declared `base_revision`, else the HEAD that `prepare_change` saw,
else HEAD.

Output: `committed` Records, `discarded` candidates with a reason, `conflicts`,
`superseded` Records, the derived state of relationships the new Records
explain, observed `signals`, and the mechanical capture of the change.

Discard reasons include `transient`, `duplicate`, `missing_rationale`,
`rationale_restates_statement`, `mechanical_noise`, `no_meaningful_binding`,
and `legacy_contract` (a pre-1.0 call without candidates).

## `resolve_conflict`

Input: `conflict_id`, `decision` (`keep_pinned` or `adopt_new`), and
`human_answer` — the person's literal answer, kept for audit. Without
`human_answer` the tool refuses. `adopt_new` requires the Git actor to be
declared in `.rationale/config.yaml`.

## Prompts

`prompts/list` returns six actions from the same source as the Claude Code
skills:

| Prompt | Purpose | Arguments |
| --- | --- | --- |
| `preflight` | Prepare context and state governing Records before editing. | `target`, `intent` |
| `explain` | Explain a possible Chesterton fence. | `target` |
| `capture` | Close a change with durable candidates. | optional `statement` |
| `conflicts` | Present conflicts with pinned Records and hand the decision to a person. | none |
| `health` | Diagnose MCP, provider, Git, and canon health. | none |
| `protocol` | Load the master protocol. | none |

A retired prompt such as `review` answers with the action that replaced it. An
unknown prompt is a JSON-RPC error and never ends the session.

## Transport

Newline-delimited JSON-RPC over stdio, protocol `2024-11-05`. One persistent
provider session per server process. Diagnostics go to stderr; stdout carries
only MCP messages.
