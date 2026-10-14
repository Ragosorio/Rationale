---
lang: en
slug: next-release
title: In the next release
description: What the release after v1.1.0 changes in the context agents receive, the commands that keep the canon true, and the evidence behind it.
section: Verify
order: 12
---

These changes are on `main` and ship in **the release after v1.1.0**. With
v1.1.0, which is what the installer gives you today, the loop works as the rest
of this documentation describes; nothing here is needed to use it.

A pre-release, **v1.2.0-rc.1**, carries these changes for people who want
them now; the stable channel stays on v1.1.0:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Ragosorio/Rationale/releases/download/v1.2.0-rc.1/rationale-installer.sh | RATIONALE_VERSION=v1.2.0-rc.1 sh
```

Update it with `RATIONALE_CHANNEL=preview rationale update`; a plain
`rationale update` goes back to v1.1.0.

## The context an agent receives

- **Only current rules are authority.** Records that were superseded, revoked,
  or retired are listed under `history` with their status and replacement, and
  `explain_target(target, include_history=true)` returns their text. An agent
  is never handed a replaced decision next to its replacement.
- **The scope is the one you asked for.** A symbol query returns what is bound
  to that symbol plus file-wide rules; a file query everything bound in the
  file; a folder query the Records bound inside it, by priority. A symbol new on
  your branch is bound to itself, not to its whole file, even when the
  structural index does not know it yet.
- **Every answer says what it is.** `retrieval.status` is `found`,
  `none_found`, `index_incomplete`, or `target_missing`; each Record says `why`
  it is there. Structure the working tree no longer has is dropped as a stale
  index.
- **Invariants cross modules along real calls.** A critical, high, or pinned
  constraint of the code your target calls arrives as an inherited constraint,
  with the call that brought it.
- **The budget covers the whole response.** What did not fit is listed in
  `omitted`, with the call that returns it. Rules that govern are never cut.

## Keeping the canon true

```bash
rationale operations                                  # prepared changes nobody closed
rationale doctor --update-baseline && rationale doctor --check   # CI that fails only on new problems
rationale retire src/legacy --reason "Replaced by src/notifications"   # plan; --apply to write, --undo to revert
rationale rule add constraint "Amounts are integer cents." --why "…" --bind src/billing --pin
```

Agents close operations with `finalize_change`, or with `abandon=true` when the
change did not happen. In Claude Code, `install-agent` adds hooks that recover
context at the start of a session, remind before editing a governed file, and
ask once to close an operation the session left open; they never block an edit.

## Evidence

Rationale now ships an evaluation (`eval/` in the repository) that runs any
binary against an invented project reproducing reported failure modes. On it,
v1.1.0 passed 1 of 15 retrieval scenarios and this release passes 15; Records
that were superseded or revoked and served as current went from 7 to 0, and
Records of another symbol from 3 to 0. These are **synthetic** results.

On Rationale's own repository, with the real structural provider, the median
estimated response went from 3,068 to 2,370 tokens and the targets over the
default budget from 10 of 12 to 2 (both declared overflows of governing rules),
with no latency regression in an interleaved comparison. The full record,
including what was not verified, is in the repository's
`docs/work-items/context-integrity-1.2.md`.
