---
lang: en
slug: evidence
title: Evidence and release status
description: What 1.0 was verified against, how, and what remains open — coverage and uncertainty attached.
section: Project
order: 13
---

## Release status

`v1.0.0` is the first stable Release: the full loop — context before a change,
autonomous capture after it, human authority over pinned rules, and the live
Control Room — is implemented, tested, and used on Rationale's own repository.

## How it was verified

- **Automated gates.** Formatting, Clippy with warnings denied, 366 Rust tests
  (unit and integration, including a governance chain in which an agent
  captures a Record, a person pins it, and a later attempt to replace it becomes
  a conflict that is resolved), RustSec audit, schema validation, the
  Control Room's typecheck, tests, and build, and this site's static checks.
- **Clean checkout.** The same gates pass from a fresh checkout of the release
  commit, where the binary serves its fallback page without `ui/dist`.
- **Release artifact.** The packaged binary was extracted and exercised: CLI,
  `doctor --check`, the embedded Control Room (Host allow-list, read-only
  methods, path traversal rejected), and all five MCP tools against the real
  Codebase Memory.
- **Registration migration.** Installing over pre-1.0 registrations for Claude
  Code, Cursor, and Codex (with the real `codex` CLI, in isolated homes)
  migrated all three, converged on a second run, and uninstalled cleanly while
  keeping unrelated servers.
- **Dogfood.** A real change to Rationale itself went through one persistent
  agent session: prepare, governance stated, change, tests, capture of three
  Records with provider-confirmed bindings, retrieval of the new rule as
  governing, and the whole operation visible live in the Control Room.

The detailed records live in the repository under `docs/work-items/`.

## What remains open

- Windows is built and tested by CI; the local release verification ran on
  macOS.
- Lexical intent-conflict polarity is a known noisy heuristic (see
  [Known limits](/docs/limits)).
- Several ADRs behind 1.0 are implemented but still `proposed` pending
  independent review.
