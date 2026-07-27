---
lang: en
slug: evidence
title: Evidence and release status
description: Read the project’s current evidence with coverage, uncertainty, and open gates attached.
section: Project
order: 12
---

## Current evidence

The governance-chain dogfood test reproduces the critical path: an uncommitted
change is captured, a Subject is materialized, a human approves the proposal,
and a new MCP session retrieves the same governing Record. The negative control
does not invent a Record for an unrelated target.

The repository’s Rust gates include debug and release tests, Clippy with
warnings denied, formatting, schema validation, and RustSec auditing. The
landing and this manual are built separately with Astro.

## What this proves

The chain is exercised against the real binary and the provider is treated as
optional coverage. The test proves retrieval and authority boundaries; it does
not prove semantic contradiction detection by itself.

## What remains open

The project is still `pre-1.0 / alpha`. Broader clean-machine installation,
platform matrices, pilot review, and unusual repository layouts remain visible
gates rather than hidden claims.
