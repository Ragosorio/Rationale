---
lang: en
slug: cli-reference
title: CLI reference
description: Every rationale command — setup, context, the Control Room, human authority, migration, and maintenance.
section: Operate
order: 8
---

## Setup

| Command | Purpose |
| --- | --- |
| `init [--skip-agent-config]` | Create the `.rationale/` canon, or keep it intact when it exists, and configure detected agents. |
| `install-agent [--dry-run] [--refresh-skills] [--global-only]` | Register the MCP server per user and write the protocol and skills into the project. Idempotent. |
| `uninstall-agent [--global-only]` | Revert exactly what `install-agent` wrote. |
| `update` | Install the latest release of your channel through the installed helper. |

## Context

| Command | Purpose |
| --- | --- |
| `health` | Project identity, Git revision, working tree, provider status, and coverage. |
| `prepare <target> [--intent "…"] [--repo-path <path>]` | Compile the same context packet an agent receives. |
| `serve [--client <claude-code\|codex\|cursor>]` | Run the persistent MCP stdio server. |
| `ui [--port <n>] [--no-open]` | Open the read-only Control Room on `127.0.0.1`. |

## Human authority

| Command | Purpose |
| --- | --- |
| `pin <record-id> [--reason "…"]` | Fix a Record: agents use it but cannot replace it without your decision. |
| `unpin <record-id> [--reason "…"]` | Return a pinned Record to normal authority. |
| `conflicts [--json]` | List agent assertions that tried to replace a pinned rule. |
| `resolve <conflict-id> <keep-pinned\|adopt-new>` | Decide a conflict. `adopt-new` requires declared authority. |
| `review-record <record-id>` | Correct, dispute, revoke, supersede, change authority, or add evidence to a Record, with an audited lifecycle event. |

`pin` and `unpin` require an interactive terminal and an actor declared under
`authority:` in `.rationale/config.yaml`, and ask you to type the Record id to
confirm. `resolve` also requires an interactive terminal; `adopt-new`
additionally requires a declared actor. An agent can never run them for you.

## Migration and maintenance

| Command | Purpose |
| --- | --- |
| `migrate [--dry-run] [--json]` | Pass pre-1.0 pending proposals through the capture gate: valid ones become Records with `migrated` provenance, noisy ones are archived with their reason. Nothing is deleted. |
| `review` | Legacy: confirm pre-1.0 proposals one by one. Normal work never creates proposals. |
| `doctor [--check] [--repair] [--json]` | Find invalid severities or authorities, Records without bindings, broken paths, dangling Subjects, and unmigrated proposals. `--check` exits 1 on findings; `--repair` asks per finding. |

## Common options

```bash
rationale health --project-root /path/to/project
rationale prepare "src/auth.rs::resolve" --intent "accept expired tokens for one minute"
rationale install-agent --project-root /path/to/project --dry-run
```

`--project-root` is accepted by every project command except `init`, which runs
from the project root. `--no-mascot` (or `RATIONALE_NO_MASCOT=1`) silences
Chestie. `--help` on any command is side-effect free.

## Environment

| Variable | Effect |
| --- | --- |
| `RATIONALE_PROVIDER=none` | Disable the structural provider. |
| `RATIONALE_ACTIVITY=off` | Disable local activity and operation snapshots. |
| `RATIONALE_SKIP_AGENT_CONFIG=1` | Skip agent configuration in `init`. |
| `RATIONALE_CHANNEL=stable\|preview` | Release channel for the installer and `update` (default `stable`). |
| `RATIONALE_VERSION`, `RATIONALE_INSTALL_DIR` | Pin an installer version or change the binary directory. |

## Output boundary

Commands with a machine contract keep JSON on stdout; diagnostics and Chestie
go to stderr. `serve` never prints anything to stdout except MCP messages.
