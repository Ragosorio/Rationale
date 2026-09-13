---
lang: en
slug: versioning
title: What is versioned
description: Canonical knowledge in Git, local traces on your machine, and how releases and channels work from 1.0.
section: Verify
order: 9
---

## Commit these

`.rationale/` travels with the project: `config.yaml` (including declared
authority), `records/`, `subjects/`, `schemas/`, `migrations/`, and the archive
of migrated proposals. These files are the shareable explanation of why the code
behaves as it does, and they are reviewed in the same pull request as the code.

The agent instructions (`CLAUDE.md`, `AGENTS.md`, `.cursor/rules/`) and the
Claude Code skills are ordinary project files too. They never contain a personal
path.

## Keep these local

| Path | Contents |
| --- | --- |
| `.rationale-local/activity/` | One NDJSON file per agent session. |
| `.rationale-local/operations/` | Context snapshots per operation. |
| `.rationale-local/conflicts/` | Pending conflicts with pinned Records. |
| `.rationale-local/installed-agent-files.json` | What `install-agent` wrote, for exact reversal. |
| `~/.cache/rationale/projects/` | Derived SQLite search cache. |

Rationale adds `.rationale-local/` to `.git/info/exclude` before its first
write, so none of it shows up in `git status`.

## Releases and channels

Releases are tagged `vMAJOR.MINOR.PATCH`, and the binary's version comes from
that tag; pre-releases add a suffix such as `-rc.1`. Each Release ships
checksummed archives for five targets, the installers, and build provenance
attestations.

| Channel | Resolves to |
| --- | --- |
| `stable` (default) | The latest full Release. |
| `preview` | The newest Release, including pre-releases (`-rc`, `-alpha`). |

Choose with `RATIONALE_CHANNEL`, both when installing and when running
`rationale update` (which defaults to `stable` too). Updating or uninstalling
never touches `.rationale/`.

## Safe recovery

Deleting the cache, snapshots, or activity loses no decision. Deleting a
canonical Record removes history and belongs in a reviewed commit — or better,
revoke or supersede it with `rationale review-record` so the lifecycle keeps
the reason.
