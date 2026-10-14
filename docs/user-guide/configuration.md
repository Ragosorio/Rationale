# Configuration, files, and privacy

## Project layout

```text
.rationale/
├── config.yaml   # project identity, declared authority, optional advice
├── doctor-baseline.json  # known findings accepted for `doctor --check` (optional)
├── records/      # canonical Records
├── subjects/     # conceptual identity
├── schemas/      # local schemas
├── migrations/   # format migrations
├── approvals/    # reserved canon structure
└── archive/      # pre-1.0 proposals archived by `migrate`

.rationale-local/            # never versioned
├── activity/                # one NDJSON file per session (ADR-0017)
├── operations/              # snapshots of each prepare_change
├── conflicts/               # pending conflicts with pinned Records
├── repairs/                 # `rationale retire` batches, for --undo
├── hooks/                   # Claude Code hook state per session
└── installed-agent-files.json
```

Rationale adds `.rationale-local/` to `.git/info/exclude` before its first
write. The SQLite search cache lives in `~/.cache/rationale/` and can be
regenerated.

Agent files written by `install-agent` are versioned with the project so the
whole team gets them:

```text
CLAUDE.md, AGENTS.md          # delimited protocol block
.cursor/rules/rationale.mdc   # Cursor rule
.claude/skills/rationale/     # the rationale skill for Claude Code
.claude/skills/rationale-*/   # Claude Code shortcuts
.agents/skills/rationale/     # the rationale skill for Codex
```

Claude Code hooks go to `.claude/settings.local.json`, Claude Code's
machine-local settings file, which is not versioned: each person installs them
on their machine. See [Claude Code](claude-code.md).

`installed-agent-files.json` records a hash for every file Rationale owns, so
reinstalling updates untouched files, keeps your edits, and uninstalling removes
exactly what Rationale wrote.

## Declared authority

```yaml
authority:
  "user:your-name <you@example.com>":
    role: architecture-owner
```

The actor is your Git identity. Anyone not listed here is a contributor: they
can use Rationale normally, but cannot pin, unpin, or adopt a replacement for a
pinned rule. `rationale rule add` records any person as the `human_stated`
author of a rule; with `--pin` it needs a declared actor.

## Execution advice

```yaml
advice:
  enabled: false        # true adds execution_advice to every prepare_change
  models:               # Claude Code aliases for each tier
    fast: haiku
    balanced: sonnet
    deep: opus
```

Earlier binaries ignore the section. See
[Claude Code](claude-code.md#model-and-effort-advice).

## Environment variables

- `RATIONALE_PROVIDER=none`: disables the structural provider.
- `RATIONALE_ACTIVITY=off`: disables local activity and snapshots.
- `RATIONALE_NO_MASCOT=1`: silences Chestie.
- `RATIONALE_SKIP_AGENT_CONFIG=1`: skips agent configuration in `init` and in
  the installer.

For the installer and `rationale update`:

- `RATIONALE_CHANNEL`: `stable` (the default since 1.0, `GET /releases/latest`)
  or `preview` (the most recent release, pre-releases included).
- `RATIONALE_VERSION`: pins a specific version, for example to roll back.
- `RATIONALE_INSTALL_DIR`: changes the binary's directory.
- `RATIONALE_DOWNLOAD_BASE`: installs from a mirror with the Releases layout;
  requires `RATIONALE_VERSION`.

## Privacy

Rationale is local-first and does not send repositories, prompts, Records, or
secrets anywhere. Local activity stores identifiers and the declared intent
(one line, at most 280 characters), never Record content, code, or
conversations; it keeps 14 days, 500 sessions, and 200 operations. The Control
Room listens only on `127.0.0.1` and writes nothing.

Even so, `.rationale/` can contain internal decisions and should be treated as
part of the repository. Exclude `.env` files, keys, tokens, dumps, and personal
data according to your team's policy.

## Uninstall without losing decisions

```bash
rationale uninstall-agent
rationale uninstall-agent --global-only
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-uninstall.sh | sh
```

Uninstalling removes the binary and what Rationale wrote into the agents'
configuration, but keeps `.rationale/`. Read
[`docs/runbooks/uninstall.md`](../runbooks/uninstall.md) before deleting the
canon by hand.
