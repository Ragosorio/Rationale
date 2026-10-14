# CLI reference

The binary's help is the executable reference, and `--help` never changes
anything:

```bash
rationale --help
rationale <command> --help
```

## Setup

| Command | Purpose |
|---|---|
| `init [--skip-agent-config]` | Creates `.rationale/` and configures the detected agents, including the Claude Code hooks. |
| `install-agent [--dry-run] [--refresh-skills] [--global-only] [--no-hooks] [--stop-hook <block\|remind\|off>]` | Registers the MCP server per user and writes the protocol, the `rationale` skill, the shortcuts, and (for Claude Code) the hooks in `.claude/settings.local.json`. Idempotent. |
| `uninstall-agent [--global-only]` | Reverts only what `install-agent` wrote, hooks included. |
| `update` | Installs the latest release of the channel through the local helper. |

## Context

| Command | Purpose |
|---|---|
| `health` | Project, Git revision, working tree, provider status and coverage, canon size, and open operations. |
| `prepare <target> [--intent "…"] [--repo-path <path>] [--advise]` | Compiles the context packet for a symbol, file, or folder. `--advise` adds execution advice. |
| `serve [--client <claude-code\|codex\|cursor>]` | Persistent MCP server over stdio. |
| `ui [--port <n>] [--no-open]` | Read-only Control Room on `127.0.0.1`. |
| `hook <session-start\|pre-edit\|post-tool\|stop> [--mode block\|remind\|off]` | Answers one Claude Code hook (JSON on stdin and stdout). `install-agent` wires these; see [Claude Code](claude-code.md). |

## Operations

| Command | Purpose |
|---|---|
| `operations [--all] [--json]` | Lists operations `prepare_change` opened and nobody finalized or abandoned, oldest first; `--all` includes closed ones. Open for more than 24 hours means stale; none is ever closed automatically. |
| `operations abandon <operation-id> --reason "…"` | Closes an operation without capture. It writes no memory and is never recorded as finalized. |

## Human authority

| Command | Purpose |
|---|---|
| `rule add <kind> "statement" --why "…" --bind <path[::symbol]> [--bind …] [--severity …] [--pin] [--exception-to <id>] [--supersedes <id>] [--id <kind.slug>]` | States a rule, invariant, decision, risk, or exception as a person: provenance `human_stated` with your Git actor, through the same gate as an agent's candidate. A folder binding states an invariant below it. |
| `pin <record-id> [--reason "…"]` | Pins a Record: agents use it but cannot replace it. |
| `unpin <record-id> [--reason "…"]` | Returns a pinned Record to normal authority. |
| `conflicts [--json]` | Lists pending conflicts with pinned rules. |
| `resolve <conflict-id> <keep-pinned\|adopt-new>` | Decides a conflict. |
| `review-record <record-id>` | A Record's lifecycle: correct, dispute, revoke, supersede, authority, evidence. |

`rule add`, `pin`, and `unpin` require an interactive terminal, so an agent
cannot sign as a person. `pin`, `unpin`, `rule add --pin`, and replacing a
pinned rule require an actor declared in `.rationale/config.yaml`. `resolve`
requires an interactive terminal; `adopt-new` also requires a declared actor.

## Migration and maintenance

| Command | Purpose |
|---|---|
| `doctor [--check] [--fail-on <error\|warning\|info>] [--json] [--update-baseline] [--no-baseline] [--repair]` | Canon integrity with a severity and a fix per finding. `--check` exits 1 only for findings at or above `--fail-on` (default `error`) that are not in `.rationale/doctor-baseline.json`. `--update-baseline` accepts the current findings as known debt. `--repair` asks about each repairable finding. |
| `retire <path> --reason "…" [--moved-to <path>] [--apply [--yes]] [--json]` | Plans what to do with the active Records bound to a removed file or folder: retire, unbind, rebind, or leave to a person (pinned). Writes nothing without `--apply`; `--apply` asks for confirmation in a terminal. |
| `retire --undo <batch-id> [--force] [--json]` | Restores the Records of a retire batch byte for byte, except files changed afterwards. |
| `migrate [--dry-run] [--json]` | Passes pre-1.0 proposals through the capture gate. Never deletes anything. |
| `review` | Legacy: confirms pre-1.0 proposals one by one. |

## Exit codes

`0` success; `1` the command failed, or `doctor --check` found a new finding at
or above the threshold, or `retire --undo` left files that changed after the
batch; `2` invalid usage. `hook` always exits `0` on valid usage so a hook
never breaks a session.

## Common options

```bash
rationale health --project-root /path/to/project
rationale prepare "src/lib.rs::function" --intent "what I want to change"
rationale prepare src/billing --advise
rationale doctor --check --fail-on warning
rationale retire src/legacy --reason "Replaced by src/notifications"
rationale install-agent --project-root /path/to/project --dry-run
rationale ui --port 9800 --no-open
```

`--no-mascot` or `RATIONALE_NO_MASCOT=1` silences Chestie, the mascot. Output
of the commands added in 1.2 is in English; older commands still print Spanish.
The flags, JSON fields, and exit codes documented here are the stable contract.
`scripts/check-doc-commands.py` fails CI when a documented command or flag does
not exist in the built binary.
