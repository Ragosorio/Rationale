# Install

Rationale ships as a verifiable binary from GitHub Releases for macOS (ARM64 and
x86_64), Linux (x86_64 and ARM64), and Windows x86_64. Building from source is
for development; users do not need Rust.

## Install from GitHub

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-installer.sh | sh
```

On Windows PowerShell:

```powershell
$installer = Join-Path $env:TEMP "rationale-installer.ps1"
Invoke-WebRequest https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-installer.ps1 -OutFile $installer
& $installer
```

Supported variables:

- `RATIONALE_CHANNEL`: `stable` (the default since 1.0) resolves GitHub's
  `GET /releases/latest`, which excludes pre-releases; `preview` resolves the
  most recent release, pre-release or not. ADR-0010 distinguished both channels
  while the project was pre-1.0; with a stable version published, a later `-rc`
  must not reach someone who did not ask for it.
- `RATIONALE_VERSION=v1.0.0` pins a specific version.
- `RATIONALE_INSTALL_DIR=$HOME/.local/bin` changes the destination.
- `RATIONALE_SKIP_AGENT_CONFIG=1` skips registering agents.

The script downloads the platform artifact, checks SHA-256, installs `rationale`
and the `rationale-update` helper, runs `rationale install-agent --global-only`
to register the MCP server with the detected agents, and leaves every
`.rationale/` untouched.

## Structural provider (optional but recommended)

Rationale works without a code-intelligence provider, with degraded coverage
(`provider_status: unavailable`; it never blocks). For full coverage, install
[`codebase-memory-mcp`](https://github.com/DeusData/codebase-memory-mcp) and
check that it is on `PATH`:

```bash
curl -fsSL https://raw.githubusercontent.com/DeusData/codebase-memory-mcp/main/install.sh | bash
which codebase-memory-mcp
```

The integration research is in [`docs/research/codebase-memory/`](../research/codebase-memory/).

## Initialize a project

From the root of the project you want Rationale to govern (it can be Rationale's
own repository, which has been dogfooded this way since Phase D):

```bash
rationale init
```

It creates `.rationale/{subjects,records,approvals,schemas,migrations}/`.
Records appear in `records/` when an agent captures durable knowledge with
`finalize_change`.

**`init` also configures the coding agents it detects** (see the next section),
unless you disable it with `rationale init --skip-agent-config` or
`RATIONALE_SKIP_AGENT_CONFIG=1`.

## Register agents in a project

The installer already registered the MCP server and `rationale init` wrote the
project instructions. Run this by hand in a project where you added an agent
after `init`, after updating Rationale, or to review exactly what it would
write before touching anything:

```bash
rationale install-agent                     # detects claude-code/codex/cursor-agent; writes or updates instructions, skills, and MCP registration
rationale install-agent --dry-run           # prints what it would do without writing anything
rationale install-agent --project-root <p>  # targets a project other than the current directory
rationale install-agent --refresh-skills    # also regenerates skill files whose provenance is unknown
```

It detects each agent by its binary on `PATH` (`claude`, `codex`,
`cursor-agent`) or by configuration already present. It then:

- writes a delimited, idempotent block into `CLAUDE.md`, `AGENTS.md`, or
  `.cursor/rules/rationale.mdc`;
- installs the Claude Code shortcuts into `.claude/skills/rationale-*/` and,
  starting with the release after v1.0.0, the `rationale` skill into
  `.claude/skills/rationale/` and `.agents/skills/rationale/`;
- registers the MCP server globally for Claude Code, Codex, and Cursor with the
  absolute path of the installed binary, as `serve --client <agent>`.

An older registration of the same binary (plain `serve`) is migrated;
per-project MCP entries from old versions are removed only when they keep
Rationale's known shape, and other servers are preserved. Starting with the
release after v1.0.0, instruction blocks written with the earlier Spanish marker
are recognized and replaced. A skill file you edited is kept, and a skill
directory that is a symbolic link created by another tool is left alone.

Revert the project with `rationale uninstall-agent`, and the user registration
with `rationale uninstall-agent --global-only` — see [`uninstall.md`](uninstall.md).

**Restart the agent session** so it loads the new configuration.

## Build from source

```bash
cd /path/to/Rationale
npm --prefix ui ci && npm --prefix ui run build   # embeds the Control Room
cargo build --release
```

It produces `target/release/rationale`. See [`build-and-test.md`](build-and-test.md).

## Update and uninstall

After a fresh install, update the binary with:

```bash
rationale update
```

For a user who still has a version older than the one that ships the helper,
run once:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-update.sh | sh
```

The helper honors `RATIONALE_CHANNEL` and `RATIONALE_VERSION` when they are set
for rollback or preview. Rolling back means reinstalling an earlier version with
that variable set. After updating, run `rationale install-agent` in each project
so its protocol and skills match the new binary. Uninstalling removes only the
binary; it never deletes a project's `.rationale/` automatically. See
[`uninstall.md`](uninstall.md).

## Verify the installation

```bash
rationale health
```

It must print JSON with `project_id`, `git_revision`, and `provider_status`.
After migrating from a version older than 1.0, also run
`rationale migrate --dry-run` in case pending proposals remain. See
[`diagnostics.md`](diagnostics.md) if something fails.
