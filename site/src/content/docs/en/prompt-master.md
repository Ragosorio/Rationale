---
lang: en
slug: prompt-master
title: Master prompt
description: The invocation protocol that install-agent writes for your agents — prepare, change, capture, and hand conflicts to a person.
section: Start
order: 3
---

## Installed for you

`rationale install-agent` writes this protocol into `CLAUDE.md`, `AGENTS.md`,
or the Cursor rule, inside a delimited block that `uninstall-agent` removes
exactly. You only need to paste it by hand for a client Rationale does not
configure.

It is kept in the repository at `docs/prompt-master.md`, compiled into the
binary, and injected into this page at build time — the installed instructions
and the site cannot drift apart.

The Spanish version is available at [prompt maestro en español](/es/docs/prompt-master).

## What it asks of the agent

Locate with Codebase Memory, call `prepare_change` before a non-trivial change,
state any governing Record or conflict explicitly, make the smallest change,
and close with `finalize_change` carrying only durable knowledge — one decision
per Record. If a candidate collides with a pinned rule, the agent stops and
asks the human, then calls `resolve_conflict` with their literal answer.

## Copy

The block below is the canonical source.
