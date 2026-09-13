# Quickstart

Five minutes, with no prior knowledge of the project.

## What it does for you

Your code memory (Codebase Memory) knows **where** the code is and **how** it
connects. Rationale knows **why** it exists and **what** must stay true.
Together they keep an agent from tearing down a fence without knowing what it
protected — [Chesterton's fence](https://en.wikipedia.org/wiki/G._K._Chesterton#Chesterton's_fence):
do not remove something until you know why it is there.

It is local-first: no server, no account, no paid API. Codebase Memory is
optional; without it Rationale keeps working with degraded coverage.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/DeusData/codebase-memory-mcp/main/install.sh | bash   # recommended
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-installer.sh | sh
```

This places the binary in `~/.local/bin` (or `$RATIONALE_INSTALL_DIR`),
verifies SHA-256, and registers the MCP server for the agents it detects
(Claude Code, Codex, Cursor) with the binary's absolute path. It does not touch
any project yet. On Windows, use the PowerShell installer from the
[`README`](../README.md#windows-powershell).

Later updates:

```bash
rationale update
```

## Initialize a project

Inside the project you want to protect:

```bash
rationale init
rationale health
```

`init` creates `.rationale/` (the project's canon) and writes the invocation
protocol for the agents it detects. To do that later instead, run
`rationale init --skip-agent-config` and then `rationale install-agent` when
you are ready. Restart your agent afterwards so it loads the tools.

## What was installed and where it lives

| What | Where | Versioned in Git |
|---|---|---|
| The `rationale` binary | `~/.local/bin/rationale` | No — it is a tool |
| **The project's canon** | `<your-project>/.rationale/` — Records, Subjects, configuration | **Yes** — reviewed in pull requests and shared with the team |
| Instructions for your agent | A delimited block in `CLAUDE.md`, `AGENTS.md`, or `.cursor/rules/rationale.mdc` | Yes |
| The `rationale` skill and shortcuts | `.claude/skills/` (Claude Code) and `.agents/skills/rationale/` (Codex) | Yes |
| MCP server registration | `~/.claude.json`, `~/.cursor/mcp.json`, or `codex mcp` — per user | No — never in the project |
| Activity and operations | `<project>/.rationale-local/` | No — excluded automatically, never leaves the machine |

The complete `rationale` skill is installed by `install-agent` starting with the
release after v1.0.0. With v1.0.0, install it from GitHub with
`npx skills add Ragosorio/Rationale`. See [the skill guide](user-guide/skills.md).

## The real flow

You ask your agent something like:

> "Make payment retries stop after three attempts."

Without you mentioning it, the agent, guided by the installed protocol:

1. Uses Codebase Memory to find `charge`, its callers, and where it lives.
2. Calls `prepare_change(target, intent)` and receives the rules and decisions
   that govern that code, why its relationships exist, and which conflicts your
   intent has.
3. Makes the change and runs the tests.
4. Calls `finalize_change` with the knowledge that stays true — for example,
   "payments that already reached the bank are never retried." Rationale writes
   it to `.rationale/records/` in that same call.

The next conversation, with any agent, receives that rule before touching
`charge`.

You can also drive it explicitly. In Claude Code, `/rationale` loads the full
skill, and `/rationale-preflight <target> <intent>` and `/rationale-capture`
run single steps. In Codex, use `$rationale`, or ask in plain language:
"Prepare this change with Rationale for `<target>` with intent `<intent>`."

Agents answer in the language you write in. The instructions they read are in
English, and they keep tool names, Record ids, and commands as they are.

## Watch it live

```bash
rationale ui
```

The [Control Room](user-guide/control-room.md) opens on `127.0.0.1:9748`: the
subgraph the agent received, the memory that explains it, and every session's
activity as it happens.

## Pin what must not move

```bash
rationale pin <record-id>
```

An agent can use a pinned rule but cannot replace it. If it tries, nothing is
written and you get a conflict:

```bash
rationale conflicts
rationale resolve <conflict-id> keep-pinned
```

Pinning requires your Git actor to be declared under `authority:` in
`.rationale/config.yaml`.

## Master prompt

`install-agent` writes the [master prompt](prompt-master.md) into each agent's
instructions. Paste it by hand only for a client Rationale does not configure.

## Check that it worked

```bash
rationale --version
rationale health
rationale doctor --check
```

`health` prints JSON with `project_id`, `git_revision`, and `provider_status`.
If something fails, see [`docs/runbooks/diagnostics.md`](runbooks/diagnostics.md).

## Remove it

```bash
rationale uninstall-agent                # reverts this project's instructions and skills
rationale uninstall-agent --global-only  # reverts the user's MCP registrations
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-uninstall.sh | sh
```

None of them touch `.rationale/` — it is your canon, and deleting it is your
decision. See [`docs/runbooks/uninstall.md`](runbooks/uninstall.md).

## Next step

If you are going to build on Rationale, continue with the
[documentation index](README.md), [CONTRIBUTING.md](../CONTRIBUTING.md), and
the foundational documents listed in the [`README`](../README.md#foundational-documents).
