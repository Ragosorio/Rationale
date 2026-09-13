# CLI reference

The binary's help is the executable reference:

```bash
rationale --help
rationale <command> --help
```

## Setup

| Command | Purpose |
|---|---|
| `init [--skip-agent-config]` | Creates `.rationale/` and configures the detected agents. |
| `install-agent [--dry-run] [--refresh-skills] [--global-only]` | Registers the MCP server per user and writes the protocol, the `rationale` skill, and the shortcuts into the project. Idempotent. |
| `uninstall-agent [--global-only]` | Reverts only what `install-agent` wrote. |
| `update` | Installs the latest release of the channel through the local helper. |

## Context

| Command | Purpose |
|---|---|
| `health` | Project, Git revision, working tree, provider status and coverage. |
| `prepare <target> [--intent "…"] [--repo-path <path>]` | Compiles the context packet for a path or symbol. |
| `serve [--client <claude-code\|codex\|cursor>]` | Persistent MCP server over stdio. |
| `ui [--port <n>] [--no-open]` | Read-only Control Room on `127.0.0.1`. |

## Human authority

| Command | Purpose |
|---|---|
| `pin <record-id> [--reason "…"]` | Pins a Record: agents use it but cannot replace it. |
| `unpin <record-id> [--reason "…"]` | Returns a pinned Record to normal authority. |
| `conflicts [--json]` | Lists pending conflicts with pinned rules. |
| `resolve <conflict-id> <keep-pinned\|adopt-new>` | Decides a conflict. |
| `review-record <record-id>` | A Record's lifecycle: correct, dispute, revoke, supersede, authority, evidence. |

`pin` and `unpin` require an interactive terminal, an actor declared in
`.rationale/config.yaml`, and confirmation by typing the id. `resolve` requires
an interactive terminal; `adopt-new` also requires a declared actor.

## Migration and maintenance

| Command | Purpose |
|---|---|
| `migrate [--dry-run] [--json]` | Passes pre-1.0 proposals through the capture gate. Never deletes anything. |
| `review` | Legacy: confirms pre-1.0 proposals one by one. |
| `doctor [--check] [--repair] [--json]` | Canon integrity. `--check` exits 1 when there are findings; `--repair` asks about each one. |

## Common options

```bash
rationale health --project-root /path/to/project
rationale prepare "src/lib.rs::function" --intent "what I want to change"
rationale install-agent --project-root /path/to/project --dry-run
rationale ui --port 9800 --no-open
```

`--no-mascot` or `RATIONALE_NO_MASCOT=1` silences Chestie, the mascot. The CLI
offers no way for an agent to pin Records or decide conflicts. Command output is
currently written in Spanish; the flags, JSON fields, and exit codes documented
here are the stable contract. If a published version shows different commands,
report the drift before updating the documentation.
