---
lang: en
slug: concepts
title: Core concepts
description: The small canonical model that separates identity, evidence, authority, and derived state.
section: Start
order: 2
---

## Subject

A Subject is the stable identity of a behavior, boundary, or concept. It keeps
a decision from becoming accidentally tied to one filename. A Subject can have
aliases, bindings, and extra fields preserved through round-trip serialization.

## Record

A Record is a versioned assertion about a Subject. It carries a statement,
severity, evidence, bindings, revision information, and lifecycle history. A
proposal is a Record-shaped observation waiting for a human decision.

## Binding

A Binding connects a Record to a file, symbol, route, table, migration, test,
or commit. File bindings can govern every symbol contained in that file.
Structural bindings are stronger when the provider resolves them; Rationale
never invents a structural identifier from a target string.

## Evidence and assessment

Evidence says what supports a statement and where to inspect it. Assessment is
derived: it reports authority, applicability, revision consistency, provider
coverage, and linkage. Derived SQLite/FTS data can be rebuilt; canonical YAML
is the source of truth.

## Approval and authority

MCP can prepare context and capture observed facts. It cannot approve a Record.
The interactive CLI records the human approval compatible with the project’s
declared authority. A pending proposal is never presented as approved merely
because it has a binding or a confident-looking statement.

## Lifecycle

The normal path is:

```text
locate → prepare → change → finalize → review → approve
```

An approved Record can later be corrected, disputed, revoked, superseded, or
given additional evidence through `review-record`; each mutation leaves an
auditable lifecycle event.
