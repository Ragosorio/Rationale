---
lang: en
slug: architecture
title: Factual architecture
description: How Git, Codebase Memory, the canonical store, derived retrieval, MCP, and human review connect.
section: Project
order: 10
---

## The boundary

```text
Git + Codebase Memory → target resolution → Rationale context packet
                                      ↓
                           agent change → capture
                                      ↓
                         pending proposal → human review
                                      ↓
                              canonical Record
```

Git supplies revision and diff facts. Codebase Memory supplies optional
structural location and relationships. Rationale owns canonical Subjects,
Records, Evidence, and review lifecycle.

## Canonical versus derived

YAML under `.rationale/` is authoritative. The SQLite/FTS cache accelerates
retrieval and can be rebuilt. The MCP server reads the canonical model through
the pipeline; it does not read a provider’s private SQLite database.

## Why the boundary matters

The agent can ask for context and capture observed facts, but the protocol does
not receive authority to approve a normative decision. This keeps uncertainty,
provider coverage, and human responsibility visible in every packet.
