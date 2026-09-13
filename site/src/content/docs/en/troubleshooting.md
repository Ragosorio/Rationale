---
lang: en
slug: troubleshooting
title: Troubleshooting
description: Diagnose provider coverage, MCP registration, missing memory, conflicts, and the Control Room without guessing.
section: Verify
order: 10
---

## `health` says the provider is unavailable

Check that `codebase-memory-mcp` is installed and on `PATH`. Rationale keeps
working, but symbol bindings and the structural neighborhood lose coverage.
The packet says so in `snapshot` and `warnings`; treat `unavailable` as
unknown, never as complete.

If Codebase Memory lost the project it had indexed, Rationale detects the stale
mapping, forgets it, and re-resolves it through the provider's public tools on
the next call.

## The agent does not see Rationale's tools

Run `rationale install-agent` again and restart the client. Registration is
per user and uses the absolute path of the installed binary, so GUI apps work
without your shell `PATH`. If you moved the binary, the next `install-agent`
migrates the registration to the new path. In Claude Code, `/rationale-health`
combines the MCP `health` tool with `rationale doctor` to show what works and
what is degraded.

## `serve` looks silent

Expected when started by hand: it waits for JSON-RPC on stdin and keeps stdout
clean. Send one JSON message per line.

## A candidate was discarded

Read the reason in `finalize_change`'s `discarded` list. The common ones:

- `transient` or `durability_not_declared` — it described this change, not
  something that stays true;
- `rationale_restates_statement` — the rationale must give the cause;
- `no_meaningful_binding` — bind it to a file or symbol that exists;
- `duplicate` — the same knowledge already exists under the id shown.

## A Record does not appear in `prepare_change`

Check its bindings with `rationale doctor --check`: a binding to a path that no
longer exists makes the Record stale for that target, and a Record without
bindings cannot govern anything. Revoked and superseded Records are history,
not governance.

## `finalize_change` returned a conflict

A candidate tried to supersede a pinned Record, so it was not written. Run
`rationale conflicts` to see both statements, then
`rationale resolve <conflict-id> keep-pinned|adopt-new`. Only a declared actor
can adopt the new assertion.

## The agent wants to simplify odd code

Ask it to call `explain_target` first. A strange branch may be a Chesterton
fence whose reason is stored in a Record.

## The Control Room is empty

It shows operations and activity of this project. If `RATIONALE_ACTIVITY=off`
was set when the agent worked, there is nothing to show. Otherwise, run a
`prepare_change` and the graph appears within a second.

## `rationale ui` serves an instructions page

That binary was built without the embedded interface (a source build without
`ui/dist`). Release binaries always include it. From source:
`npm --prefix ui ci && npm --prefix ui run build`, then rebuild.

## Pre-1.0 proposals are still pending

Run `rationale migrate --dry-run`, then `rationale migrate`. Valid proposals
become Records with `migrated` provenance; noisy ones are archived with their
reason under `.rationale/archive/proposals/`.
