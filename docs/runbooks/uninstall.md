# Uninstall

Uninstalling first removes the global MCP registrations that still point at the
binary, then removes the executable. It keeps the whole `.rationale/` canon.

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-uninstall.sh | sh
```

Canonical data is never deleted automatically.

This only uninstalls the binary. If `rationale init` or `install-agent`
configured an agent **inside a project** (a block in `CLAUDE.md` or
`AGENTS.md`, `.cursor/rules/rationale.mdc`, or skills under `.claude/skills/`
and `.agents/skills/rationale/`), reverting that is per project:

```bash
cd /path/to/project
rationale uninstall-agent
```

To revert only the user-level registrations for Claude Code, Codex, and Cursor,
without removing any project's blocks:

```bash
rationale uninstall-agent --global-only
```

It deletes only what `install-agent` wrote, according to
`.rationale-local/installed-agent-files.json`, and leaves any earlier user
content in those files intact. Skill files you edited are kept.

## Always safe to delete

```bash
rm -rf ~/.cache/rationale                    # the derived layer of ALL projects — see cache-reset.md
rm -f /path/to/project/.mcp.json             # only if an old version wrote it and you used it just for Rationale
rm -rf /path/to/Rationale/target             # build artifacts
```

Each project's `.rationale-local/` (local activity and operations, never
versioned) is also safe to delete:

```bash
rm -rf /path/to/project/.rationale-local
```

## Never delete without thinking (it is the canon, versioned in Git)

```text
.rationale/records/      # Records — deleting them loses real authority
.rationale/subjects/     # the conceptual identity of each governed behavior
.rationale/approvals/
.rationale/bindings/
```

If you really want to remove Rationale from a project entirely:

```bash
rationale uninstall-agent          # removes protocol blocks and skills it wrote
git rm -r .rationale/              # stays in Git history, recoverable
git commit -m "remove Rationale from this project"
```

**Never run `rm -rf .rationale/` followed by a force-push** — that would destroy
decisions with no way to recover them. `git rm` keeps the history intact.

## Pre-1.0 proposals

Since 1.0, normal work creates no proposals. If a project still has
`.rationale/proposals/` from an earlier version, run them through
`rationale migrate` first: valid ones become Records and noisy ones are archived
with their reason. A proposal never had authority, so deleting the ones you do
not want is safe; `git rm` keeps them in history.
