---
lang: en
slug: architecture
title: Factual architecture
description: How Git, Codebase Memory, the canon, the context compiler, MCP, local activity, and the Control Room fit together.
section: Project
order: 13
---

## Components

| Module | Responsibility |
| --- | --- |
| `pipeline` | Orchestrates `prepare`, `explain`, `health`, and `finalize`. |
| `providers` | A normalized structural model; the Codebase Memory adapter speaks only its public MCP tools. A fixture provider backs deterministic tests. |
| `context` + `retrieval` | Select the structural neighborhood, derive relationship state, and compile the packet under a token ceiling. |
| `canon` + `storage` | The capture gate, provenance, authority, supersession, conflicts, migration, and atomic YAML writes under a lock. |
| `relationships` | Explanations of relationships and their derived state. |
| `operations` + `activity` | Operation snapshots and the per-session activity stream (ADR-0017). |
| `mcp::server` | Newline-delimited JSON-RPC over stdio: five tools, six prompts. |
| `ui` | A std-only localhost HTTP server with REST views and Server-Sent Events. |
| `agents` + `prompts` + `skill_bundle` | Convergent, reversible agent registration; the protocol and shortcuts from one source; the embedded `rationale` skill written file by file with a hash. |
| `doctor` | Canon integrity checks and guided repair. |

## Canonical versus derived

YAML under `.rationale/` is the only authority. The SQLite search cache,
operation snapshots, and activity can be rebuilt or deleted. No component reads
a provider's private storage (`constraint.no-provider-internal-access`).

## Boundaries that hold

- The provider informs, the canon decides. Absence of an edge is not proof.
- Agents write memory, people own authority: pinning, unpinning, and replacing
  a pinned rule are human acts with declared authority.
- The Control Room observes and never writes; the MCP server is the only agent
  boundary.
- Local traces stay local and minimal: identifiers and a one-line intent.

## Where to read more

The repository keeps the conceptual architecture, a factual code map in
`docs/architecture/`, and the ADRs in `docs/adr/` with their approval status.
