---
lang: en
slug: concepts
title: Core concepts
description: The small model behind Rationale — Records, bindings, provenance, authority, relationships, and operations.
section: Start
order: 2
---

## Record

A **Record** is one durable piece of knowledge about the code: a `constraint`
(what must stay true), a `decision` (why it is this way), a `risk`, or an
`exception`. It has a statement, a rationale that gives the cause, a severity,
and its lifecycle. Records live as YAML in `.rationale/records/` and are
versioned with the project.

**One decision per Record.** If two parts could be replaced or revoked
separately, they are two Records.

## Binding

A binding ties a Record to the code it governs: a file (`src/retry.rs`) or a
symbol (`src/retry.rs::backoff`). Symbol bindings are confirmed by the
structural provider and stored with a portable id — no machine-specific path.
Bindings created from uncommitted code are marked `provisional`.

## Provenance

Every Record says where it came from:

- `agent_asserted` — written by an agent through `finalize_change`, with the
  client, session, and operation that asserted it;
- `human_stated` — declared by a person;
- `migrated` — carried over from the pre-1.0 approval workflow.

Provenance is never upgraded silently: an agent's assertion is not presented as
a human statement.

## Authority

- `normal` — the default. A newer Record may supersede it explicitly.
- `pinned` — a rule the project fixed. Agents use it, but any attempt to
  supersede it becomes a **conflict** that only a person resolves.

Pinning, unpinning, and adopting a replacement over a pinned Record require an
actor declared under `authority:` in `.rationale/config.yaml`.

## Relationship rationale

A Record can also explain **why a relationship exists** — why `checkout` calls
`reserve_stock`, for example. Each time context is compiled, Rationale derives
the relationship's structural state from the provider:

| State | Meaning |
|---|---|
| `observed` | The direct relationship exists in the current index. |
| `indirect` | It is no longer direct, but a bounded path still connects both ends. |
| `orphaned` | It cannot be located; the explanation may be stale. It is never deleted. |
| `unknown` | The provider could not verify it. Absence is not proof. |

## Operation

`prepare_change` opens an **operation** with an `operation_id`. It records what
was considered and selected (nodes, relationships, Records, packet size) as a
local snapshot. `finalize_change` closes the same operation, so the Control Room
can show the whole change from context to captured memory.

## Canonical versus derived

`.rationale/` is the only source of truth. The SQLite search cache under
`~/.cache/rationale/`, operation snapshots, and activity in `.rationale-local/`
are derived or local, and can be deleted without losing a decision.
