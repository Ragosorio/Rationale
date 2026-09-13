# CLI for agents

Most work happens through the MCP tools. Run `rationale` commands only when a
playbook calls for them or the user asks. Some commands are reserved for people
because they carry the project's authority.

Add `--no-mascot` for clean output, and `--project-root <path>` when the current
directory is not the repository root (`init` does not accept it; run it from the
root).

## Read-only: safe to run

| Command | Use |
|---|---|
| `rationale --version` | Confirm the binary and its version |
| `rationale health` | The same data as the `health` tool |
| `rationale prepare <target> --intent "<intent>"` | The packet an agent would receive, when MCP is unavailable |
| `rationale conflicts --json` | Pending conflicts with pinned Records |
| `rationale doctor`, `doctor --json`, `doctor --check` | Canon integrity findings; `--check` exits 1 when there are findings |
| `rationale install-agent --dry-run` | What agent configuration would change |
| `rationale migrate --dry-run --json` | What migrating pre-1.0 proposals would do |
| `rationale <command> --help` | Usage, without side effects |

## Changes the project: run when the user asks

| Command | Effect |
|---|---|
| `rationale init` | Creates `.rationale/` (keeps an existing one) and configures detected agents |
| `rationale install-agent` | Registers the MCP server per user; writes protocol blocks and skills |
| `rationale uninstall-agent` | Reverts exactly what `install-agent` wrote; edited skills are kept |
| `rationale migrate` | Moves pre-1.0 proposals through the capture gate |
| `rationale update` | Installs the latest release of the configured channel |
| `rationale ui` | Serves the read-only Control Room on `127.0.0.1` and keeps running, so let the person start it |

## Reserved for people

These require an interactive terminal, and several require an actor declared in
`.rationale/config.yaml`. Give the person the exact command instead of running
it.

| Command | Why it is theirs |
|---|---|
| `rationale pin <record-id>`, `rationale unpin <record-id>` | Decides which rules no agent may replace |
| `rationale resolve <conflict-id> keep-pinned`, `... adopt-new` | Decides a conflict with a pinned rule |
| `rationale review-record <record-id>` | Corrects, disputes, revokes, or supersedes a Record with an audited event |
| `rationale doctor --repair` | Applies repairs one finding at a time, with confirmation |
| `rationale review` | Legacy confirmation of pre-1.0 proposals |

## Environment variables

`RATIONALE_PROVIDER=none` disables the structural provider and
`RATIONALE_ACTIVITY=off` disables local activity and operation snapshots. Set
them only at the user's request: both change what Rationale can report.
