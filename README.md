# Rationale

Rationale is local causal memory for coding agents. Before an agent changes your
code, Rationale hands it the rules, decisions, and relationships that govern that
code. After the change, the knowledge that stays true is written back to your
repository automatically, and the rules you pin stay out of any agent's reach.

> Git remembers what changed. Rationale remembers why it still matters.

Website and documentation, in English and Spanish:
[rationale-pearl.vercel.app](https://rationale-pearl.vercel.app).

## Choose your path

- **I just want to use it:** start with the [quickstart](docs/quickstart.md).
- **I want to connect an agent:** read [Agents and MCP](docs/user-guide/agents-and-mcp.md).
- **I want my agents to follow a complete playbook:** see the [`rationale` skill](docs/user-guide/skills.md).
- **I want to see what my agents are doing:** open the [Control Room](docs/user-guide/control-room.md).
- **I want to contribute:** read [CONTRIBUTING.md](CONTRIBUTING.md).
- **I want to investigate a decision:** see [Concepts](docs/user-guide/concepts.md) and the [ADRs](docs/adr/).
- **I have a problem:** open an issue following [SUPPORT.md](SUPPORT.md).

## Status

`v1.1.0` is the current release. The full loop has been stable since `v1.0.0`
and is used on Rationale's own repository: context before the change,
autonomous capture after it, human authority over pinned rules, and the live
Control Room. `v1.1.0` adds the `rationale` Agent Skill, which `install-agent`
installs for Claude Code and Codex, and writes the text agents read in English
while they answer in your language. What changed is in
[`CHANGELOG.md`](CHANGELOG.md); how 1.0 was verified is in
[`docs/work-items/v1.0-release-verification.md`](docs/work-items/v1.0-release-verification.md).

## Quick install

### macOS and Linux

```bash
curl -fsSL https://raw.githubusercontent.com/DeusData/codebase-memory-mcp/main/install.sh | bash   # recommended
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-installer.sh | sh
rationale --version
```

### Windows PowerShell

```powershell
$installer = Join-Path $env:TEMP "rationale-installer.ps1"
Invoke-WebRequest https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-installer.ps1 -OutFile $installer
& $installer
rationale.exe --version
```

Then, from the root of the project you want to protect:

```bash
rationale init
rationale health
rationale ui
```

The installer verifies SHA-256 checksums, uses the `stable` channel, registers
the MCP server for the agents it detects, and never touches `.rationale/` when
updating or uninstalling. The full guide is in
[`docs/runbooks/install.md`](docs/runbooks/install.md).

## Get started

1. **Install** Codebase Memory (recommended) and Rationale with the commands above.
2. **Initialize** your repository with `rationale init`. It creates the canon in
   `.rationale/` and writes the invocation protocol for Claude Code, Codex, or
   Cursor.
3. **Restart your agent.** In Claude Code, run `/rationale-health`. In Codex,
   ask "Check Rationale health."
4. **Ask for a real change.** The agent prepares with `prepare_change`, makes the
   change, and captures what stays true with `finalize_change`.
5. **Pin the rules that must not move** with `rationale pin <record-id>`.
6. **Watch it happen** with `rationale ui`.

## The loop in five steps

1. **Locate.** The agent finds the code with Codebase Memory.
2. **Prepare.** It calls `prepare_change(target, intent)` and receives a bounded
   packet: the constraints and decisions that govern the target (with authority
   and provenance), explained relationships with their structural state, the
   code's neighborhood, and conflicts with its intent.
3. **Change.** It makes the smallest change consistent with that context.
4. **Capture.** It calls `finalize_change` with only durable knowledge.
   Rationale discards noise and duplicates and writes the rest as canonical
   Records in the same call. There is no approval queue.
5. **Pin.** You pin the rules no agent may replace with `rationale pin`. If an
   agent tries, nothing is written: you get a conflict to settle with
   `rationale resolve`.

The guided walkthrough is in [`docs/quickstart.md`](docs/quickstart.md), and the
day-to-day flow in [`docs/user-guide/daily-workflow.md`](docs/user-guide/daily-workflow.md).

## The `rationale` skill

Rationale ships an [Agent Skill](https://agentskills.io) in
[`skills/rationale/`](skills/rationale/). A short `SKILL.md` routes the agent to
one playbook per operation, loaded only when the task needs it:

| Operation | When the agent uses it |
|---|---|
| `preflight` | Before changing non-trivial code |
| `explain` | Before simplifying code that looks redundant or odd |
| `capture` | After a change, to write back what stays true |
| `conflicts` | When a candidate collides with a pinned rule |
| `health` | When tools are missing or results look degraded |
| `adopt` | When setting Rationale up and seeding the first Records |
| `maintain` | When bindings go stale or `rationale doctor` reports findings |

The skill also carries a guide to writing Records that the capture gate keeps, a
validator that mirrors the gate (`scripts/check_candidates.py`), report
templates, Codex metadata, and evals. Agent-facing text is written in English
and tells the agent to reply in the user's language.

```bash
rationale install-agent              # Claude Code and Codex (rationale init already runs it)
npx skills add Ragosorio/Rationale   # any other agent that reads Agent Skills
```

## How it works

```text
Codebase Memory (where/how) ─┐
Git (what/when) ─────────────┼─> context compiler ─> packet ─> agent
Canon .rationale (why) ──────┘

agent changes code ─> finalize_change ─> capture gate ─> canonical Records
                                             │
                          collision with a pinned rule ─> human decision

local activity and operations ─> rationale ui (read-only, 127.0.0.1)
```

Codebase Memory is an optional structural provider. Without it, Rationale keeps
working with degraded coverage and says so; it never reads the provider's
internal database or treats it as a source of authority.

Rationale is not another code indexer, does not replace Git, is not a SaaS, does
not store conversations, does not use remote embeddings, and does not decide on
its own what a statement means.

## Commands and MCP

| Need | CLI | MCP |
|---|---|---|
| Initialize | `rationale init` | — |
| Health | `rationale health` | `health` |
| Prepare context | `rationale prepare <target>` | `prepare_change` |
| Explain a target | — | `explain_target` |
| Capture knowledge | — | `finalize_change` |
| Watch work live | `rationale ui` | — |
| Pin / unpin a rule | `rationale pin` / `unpin` | — |
| Decide a conflict | `rationale conflicts` / `resolve` | `resolve_conflict` (with the person's literal answer) |
| Record lifecycle | `rationale review-record <id>` | — |
| Migrate pre-1.0 proposals | `rationale migrate` | — |
| Canon integrity | `rationale doctor` | — |
| Register / revert agents | `install-agent` / `uninstall-agent` | — |

Agents write memory; people keep authority. Pinning, unpinning, and replacing a
pinned rule require an interactive terminal and an actor declared in
`.rationale/config.yaml`.

## Foundational documents

1. [`Rationale_v0.5.md`](Rationale_v0.5.md) — product contract: problem,
   entities, trust, and roadmap.
2. [`Rationale_Arquitectura_Conceptual_v0.1.md`](Rationale_Arquitectura_Conceptual_v0.1.md)
   — conceptual architecture: technical boundaries and architecture decisions.
3. [`Rationale_Proceso_Construccion_Agentes_v0.1.md`](Rationale_Proceso_Construccion_Agentes_v0.1.md)
   — agent build process: workflow, cross-review, and quality gates.

The file names keep their original form so existing links keep working. Release
1.0 replaced the approval queue these documents describe with autonomous capture
under human authority over pinned rules; the details are in
[`docs/work-items/vnext-implementation-plan.md`](docs/work-items/vnext-implementation-plan.md).

## Data, privacy, and files

| Layer | Location | In Git |
|---|---|---|
| Records, Subjects, and configuration | `<project>/.rationale/` | Yes |
| Protocol blocks and skills | `CLAUDE.md`, `AGENTS.md`, `.cursor/rules/`, `.claude/skills/`, `.agents/skills/` | Yes |
| Activity, operations, and pending conflicts | `<project>/.rationale-local/` | No (excluded automatically) |
| SQLite search cache | `~/.cache/rationale/projects/<id>/` | No |
| Binary | `~/.local/bin/rationale` or the configured directory | No |

Rationale is local-first: it does not upload code, prompts, Records, or secrets.
Local activity stores identifiers and a one-line intent, never the content of a
Record (ADR-0017); `RATIONALE_ACTIVITY=off` disables it. Read
[`docs/user-guide/configuration.md`](docs/user-guide/configuration.md) before
using it in repositories with sensitive data.

## Documentation

The index by audience is [`docs/README.md`](docs/README.md). Documentation in
this repository is written in English; the website publishes the user
documentation in English and Spanish.

- [Quickstart](docs/quickstart.md) — first run.
- [User guide](docs/user-guide/) — concepts, daily workflow, the skill, Control Room, CLI, MCP, and configuration.
- [Runbooks](docs/runbooks/) — install, diagnostics, provider, cache, and release.
- [Factual architecture](docs/architecture/code-map.md) — real modules and flows.
- [ADRs](docs/adr/) — decisions and their approval status.
- [Security](docs/security/) — limits and baseline.
- [Codebase Memory research](docs/research/codebase-memory/) — integration and limits.

## Contributing and help

- Contributions: [`CONTRIBUTING.md`](CONTRIBUTING.md).
- Vulnerabilities: [`SECURITY.md`](SECURITY.md).
- Support and bugs: [`SUPPORT.md`](SUPPORT.md).
- Community conduct: [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md).
- Change history: [`CHANGELOG.md`](CHANGELOG.md).
- Third-party notices: [`THIRD_PARTY.md`](THIRD_PARTY.md).

## License

MIT. See [`LICENSE`](LICENSE).
