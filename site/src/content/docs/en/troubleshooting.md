---
lang: en
slug: troubleshooting
title: Troubleshooting
description: Diagnose provider coverage, MCP silence, stale links, and review state without guessing.
section: Verify
order: 8
---

## `health` says the provider is unavailable

Check that `codebase-memory-mcp` is installed and on `PATH`. Rationale still
works, but symbol resolution and automatic bindings have lower coverage. Read
the warning in the packet instead of treating unavailable as complete.

## `serve` looks silent

That is expected when started manually: stdio waits for JSON-RPC traffic and
keeps stdout clean. Send JSON messages per line. Do not add a banner, logging,
or Chestie output to stdout.

## A constraint is missing

Run `rationale health`, then inspect the Record’s severity, approval, bindings,
and linkage. `medium` is visible; zero bindings is reported as unresolved. Use
`rationale doctor --check` to find legacy Records with missing paths, invalid
severity, dangling Subjects, or no approval.

## The agent wants to simplify odd code

Ask it to call `explain_target` first. A strange branch may be a Chesterton
fence whose reason is stored in an approved Record.

## A proposal was captured twice

Do not delete one by hand. Review the pending proposals, compare their evidence
and bindings, and reject or correct the duplicate through the interactive
review path.

## A GUI client reports the MCP server as unavailable

Cursor or a desktop app launched from the Dock reports `rationale` as
unavailable, while Codex and a terminal Claude Code work fine.

The MCP configuration deliberately declares the logical command `rationale`
rather than a personal absolute path, so the file can be committed and shared.
That command only resolves if the client can see the directory holding the
binary. On macOS, an application launched from the Dock inherits the `launchd`
environment, not your shell's, so `~/.local/bin` — where the installer places
the binary — is invisible to it.

`install-agent` warns about this and prints the remedy. Either launch the client
from a terminal, or expose the binary where GUI applications can see it:

```bash
sudo ln -sf ~/.local/bin/rationale /usr/local/bin/rationale
```

Nothing in the project configuration needs to change.
