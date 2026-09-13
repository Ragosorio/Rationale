---
lang: en
slug: control-room
title: Control Room
description: rationale ui — the working subgraph, causal memory, and live agent activity, served read-only from your machine.
section: Operate
order: 5
---

## Open it

```bash
rationale ui
rationale ui --port 9800 --no-open
```

The Control Room listens only on `127.0.0.1` (port `9748` by default; without
`--port` it tries the next ones if busy) and opens your browser. The interface
is embedded in the binary. It observes the same local state as the CLI and the
MCP server and **never writes**: `rationale serve` remains the agent boundary.

## Graph

The working subgraph of recent operations, rendered in 3D:

- node color is the **role** in the change — target, caller, callee,
  dependency, dependent, test, context;
- edge color is the **structural state** — observed, indirect, orphaned,
  unknown;
- a causal ring marks nodes and relationships **explained by a Record**.

Select a node to read the Records that govern it, its relationships, and the
operations where it appeared. Select a relationship to see why it exists and
how its state changed over time.

## Activity

Every agent session, in real time: which client connected (Claude Code, Codex,
Cursor, or the CLI), each operation's intent and target, provider latency,
packet size, captured and discarded candidates, conflicts, and explanations at
risk. Events arrive over Server-Sent Events; no refresh needed.

## Memory

The canon as a browser: filter by kind, status, and authority; read statement,
rationale, provenance, bindings, and explained relationships; see pending
conflicts with pinned Records. Newly captured Records are marked as they land.

## System

Project, Git revision, canon counts, stream state, and the session table. It
also restates the privacy boundary below.

## What is stored, and where

Activity is one NDJSON file per session in `.rationale-local/activity/`;
operation snapshots live in `.rationale-local/operations/`. Both are excluded
from Git automatically and never leave the machine. They hold identifiers, the
declared intent (one line, at most 280 characters), and references to Records —
never Record content, code, or conversations (ADR-0017). Retention keeps the
last 14 days and at most 500 sessions and 200 operations.

`RATIONALE_ACTIVITY=off` disables activity and snapshots entirely.

## Security

The server accepts `GET` and `HEAD` only, validates the `Host` header against
loopback names (DNS-rebinding defense), bounds request size and time, sends a
restrictive Content-Security-Policy, and serves only embedded assets — never a
filesystem path.
