# ADR-0016: User-scoped MCP registration and convergent migration

**Status:** proposed — pending independent cross-review and human approval.
**Date:** 2026-07-29
**Deciders:** Codex (research and implementation); human decision pending
**Supersedes / Superseded by:** proposes replacing ADR-0015 if this ADR reaches `accepted`. While both remain `proposed`, neither governs the other.

## Context

ADR-0015 chose the logical command `rationale` inside `.mcp.json` and
`.cursor/mcp.json`. Its own `Revisit trigger` required reopening the decision if
Cursor could not resolve the command from a graphical application.

The real validation failed on 2026-07-29. Cursor loaded the
`.cursor/rules/rationale.mdc` rule and saw `.cursor/mcp.json`, but reported the
`rationale` server as disconnected. The graphical process did not resolve
`~/.local/bin/rationale` with the `PATH` it had available; the local CLI did
respond.

Codebase Memory solves the same boundary by registering its MCP server once in
each client's user configuration and using the absolute path of the installed
binary. Its project files do not mix a personal path with shared configuration.

Codex's migration also had an independent defect: Rationale considered the work
done if `codex mcp list` contained the name `rationale`, even when the command
still pointed at an old build.

## Decision

1. MCP registration for Claude Code, Codex, and Cursor is **per user**. The
   installer uses the absolute path of the installed binary in `~/.claude.json`,
   in the official configuration managed by `codex mcp`, and in
   `~/.cursor/mcp.json`.
2. Project files contain instructions and skills, not the path of the installed
   server. `install-agent` removes legacy entries from `.mcp.json` and
   `.cursor/mcp.json` only when they keep a shape Rationale recognizes as its
   own.
3. Every installation is convergent: it compares command and arguments, not only
   the existence of the name. An obsolete entry is migrated to the current
   binary.
4. Global uninstall removes only entries that still point at the binary being
   uninstalled.
5. The verifiable support in beta.3 remains Claude Code, Codex, and Cursor.
   Codebase Memory's client table serves as a design precedent, not as evidence
   that Rationale already supports all its clients.
6. When a Codebase Memory version does not persist `root_path` across processes,
   Rationale stores in `.rationale-local/` the public name returned by
   `index_repository`, together with the canonical root. It stores no node IDs
   and does not access the provider's storage.

Note added after 1.0: project files now also carry the `rationale` Agent Skill
(`.claude/skills/rationale/`, `.agents/skills/rationale/`), still without any
machine-specific path.

## Evidence

- Cursor showed `rationale` configured but disconnected while the local CLI
  answered.
- Simulating a typical GUI application `PATH` does not find `rationale`; the
  installed absolute path does exist.
- Codebase Memory's global Cursor configuration uses an absolute path.
- `codex mcp get rationale` can detect an obsolete command that
  `codex mcp list` does not distinguish.

## Consequences

- Installing or updating repairs the three supported clients without editing
  each repository.
- Restarting the client is still necessary.
- Two users of the same repository can have different installation paths
  without producing diffs.
- A checkout that was cloned without Rationale installed gets instructions, not a
  nonexistent server.

## Risks

- Global formats are external contracts, and a future version could change them.
- Every member must run the installer once.
- The initial scope does not replicate the dozens of clients Codebase Memory
  supports. Adding them requires detection, non-destructive merging, reversal,
  and real validation per client.

## Validation

Before publishing beta.3:

1. Install on an isolated HOME with pre-existing configurations.
2. Migrate a Codex entry with an obsolete command.
3. Migrate a beta.2 project, preserving instructions and other servers.
4. Restart Cursor and run `health` through real MCP. **Passed on
   2026-07-29:** `user-rationale` appeared `ready`, the four tools were
   available, and `health` returned complete coverage.
5. Run the formatter, Clippy, tests, and the release clean room.

## Revisit trigger

Reopen if a supported client stops accepting its global configuration, if a user
needs two binaries at the same time, or if another client is added without an
explicit detection, merge, reversal, and real-test strategy.
