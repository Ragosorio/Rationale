---
lang: en
slug: quickstart
title: Five-minute quickstart
description: Install Rationale, connect your agent, make the first governed change, and watch it live in the Control Room.
section: Start
order: 1
---

## What you need

Rationale is a single local binary for macOS (Apple Silicon and Intel), Linux
(x86_64 and ARM64), and Windows x86_64. You need Git and a shell. Codebase
Memory is recommended for structural context but optional: without it,
Rationale reports degraded coverage and keeps working.

## Install

Install the structural companion first:

```bash
curl -fsSL https://raw.githubusercontent.com/DeusData/codebase-memory-mcp/main/install.sh | bash
```

Then install Rationale:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-installer.sh | sh
rationale --version
```

On Windows, in PowerShell:

```powershell
$installer = Join-Path $env:TEMP "rationale-installer.ps1"
Invoke-WebRequest https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-installer.ps1 -OutFile $installer
& $installer
rationale.exe --version
```

The installers verify SHA-256 checksums and use the `stable` channel. Set
`RATIONALE_CHANNEL=preview` only if you want pre-releases.

## Initialize a project

From the repository you want to protect:

```bash
rationale init
rationale health
```

`init` creates the versioned canon in `.rationale/` and configures the coding
agents it detects. `health` reports the Git revision, the working tree, and
whether the structural provider is available.

To connect agents later, or after an update:

```bash
rationale install-agent --dry-run
rationale install-agent
```

It registers the MCP server once per user (`rationale serve --client <agent>`)
and writes the invocation protocol into `CLAUDE.md`, `AGENTS.md`, or the Cursor
rule. Restart the agent afterwards.

## Add the rationale skill

The protocol tells your agent *that* it must prepare and capture. The
[`rationale` skill](/docs/skill) teaches it *how*, one playbook per operation,
loaded only when a task needs it. `init` and `install-agent` already installed
it in `.claude/skills/rationale/` and `.agents/skills/rationale/`. For another
agent that reads Agent Skills:

```bash
npx skills add Ragosorio/Rationale
```

## Make the first governed change

Ask your agent for a real change. With the protocol installed it will:

1. locate the code with Codebase Memory;
2. call `prepare_change(target, intent)` and read the rules, decisions, and
   explained relationships that govern the target;
3. make the change and run the tests;
4. call `finalize_change` with only the knowledge that stays true — Rationale
   writes it to `.rationale/records/` in the same call.

You don't have to mention Rationale. To drive it explicitly in Claude Code,
use `/rationale` (the skill) or the shortcuts
`/rationale-preflight <target> <intent>` and `/rationale-capture`. In Codex,
use `$rationale`, or ask in plain language: “Prepare this change with Rationale
for `<target>` with intent `<intent>`.”

The agent answers in the language you write in. The instructions it reads are
in English, and tool names, Record ids, and commands stay as they are.

## Watch it live

```bash
rationale ui
```

The Control Room opens on `127.0.0.1:9748`: the working subgraph the agent
received, the memory that explains it, and every session's activity as it
happens. It is read-only and never leaves your machine.

## Keep the rules that matter

When a rule must not be replaced by any agent, pin it:

```bash
rationale pin <record-id>
```

If an agent later tries to supersede it, nothing is written: you get a conflict
to decide with `rationale conflicts` and `rationale resolve`.
