# Control Room

`rationale ui` opens a local, read-only control room: the agents' working
subgraph, the memory that explains it, and every session's activity live.

```bash
rationale ui
rationale ui --port 9800 --no-open
```

It listens only on `127.0.0.1` (port `9748` by default; without `--port` it
tries the next ports if that one is busy). The interface is embedded in the
binary. It observes the same local state as the CLI and the MCP server and never
writes: `rationale serve` remains the boundary with agents.

## Views

- **Graph.** The subgraph of recent operations, in 3D. A node's color is its
  role in the change (target, caller, callee, dependency, dependent, test,
  context); an edge's color is its structural state. A ring marks what the canon
  explains. Select a node or relationship to read its Records, relationships,
  and history.
- **Activity.** Every session — Claude Code, Codex, Cursor, or the CLI — with
  its operations, provider latency, packet size, what was captured, what was
  discarded, conflicts, and explanations at risk. It arrives through
  Server-Sent Events.
- **Memory.** The canon, filterable by kind, status, and authority, plus pending
  conflicts.
- **System.** Project, Git revision, canon counts, stream status, and the
  sessions table.

## Data it shows

- Activity: `.rationale-local/activity/<session>.ndjson`.
- Operations: `.rationale-local/operations/`.
- Canon: `.rationale/records/`; conflicts: `.rationale-local/conflicts/`.

Activity stores identifiers, the declared intent (one line, at most 280
characters), and references to Records — never Record content, code, or
conversations (ADR-0017). `RATIONALE_ACTIVITY=off` disables it entirely.

## Security

Only `GET` and `HEAD`; the `Host` header is validated against loopback names
(a defense against DNS rebinding); headers are bounded in size and time; a
restrictive Content-Security-Policy; embedded assets only, never file-system
paths.

## From source

Without `ui/dist`, the binary serves a page that explains how to build the
interface. Release binaries always include it.

```bash
npm --prefix ui ci
npm --prefix ui run build
cargo build --release
```

`build.rs` watches `ui/dist` (or `ui/` if it does not exist yet). If a `target/`
directory was built before `ui/` existed, touch `build.rs` once so the assets
are embedded again.
