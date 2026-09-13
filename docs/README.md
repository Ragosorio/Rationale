# Rationale documentation

Choose a path by goal. The root README summarizes the project; this page
organizes the details without duplicating them. Documentation in this
repository is written in English; the
[website](https://rationale-pearl.vercel.app) publishes the user documentation in
English and Spanish.

## I want to use Rationale

1. [Quickstart](quickstart.md)
2. [Concepts](user-guide/concepts.md)
3. [Daily workflow](user-guide/daily-workflow.md)
4. [The `rationale` skill](user-guide/skills.md)
5. [Control Room](user-guide/control-room.md)
6. [CLI reference](user-guide/cli-reference.md)
7. [Agents and MCP](user-guide/agents-and-mcp.md)
8. [Configuration, files, and privacy](user-guide/configuration.md)
9. [Diagnostics](runbooks/diagnostics.md)
10. [Master prompt for agents](prompt-master.md)

## I want to contribute

1. [CONTRIBUTING.md](../CONTRIBUTING.md)
2. [Architecture map](architecture/code-map.md)
3. [Rust guides](rust/)
4. [Agent build process](../Rationale_Proceso_Construccion_Agentes_v0.1.md)
5. [ADRs](adr/)
6. [Work items and evidence](work-items/)
7. [Build and test](runbooks/build-and-test.md)
8. [Skill source and evals](../skills/rationale/)

## I want to operate an installation

- [Install and update](runbooks/install.md)
- [Diagnostics](runbooks/diagnostics.md)
- [Provider failure](runbooks/provider-failure.md)
- [Cache reset](runbooks/cache-reset.md)
- [Uninstall](runbooks/uninstall.md)
- [Release](runbooks/release.md)
- [Security baseline](security/baseline.md)

## I want to understand the design

- [Plan and history of the 1.0 model](work-items/vnext-implementation-plan.md)
- [1.0 release verification](work-items/v1.0-release-verification.md)
- [Product contract](../Rationale_v0.5.md)
- [Conceptual architecture](../Rationale_Arquitectura_Conceptual_v0.1.md)
- [Factual code map](architecture/code-map.md)
- [Codebase Memory research](research/codebase-memory/)
- [Core language research](research/language/)
- [ADR index](adr/index.md)

## Conventions

Documentation states its audience, links to the source of truth instead of
duplicating it, and uses commands verified against the current binary or
scripts. When a claim is uncertain, mark it `Unknown`, with evidence, risk, and
the next experiment.

ADRs, work items, research notes, and the foundational documents are historical
records: they keep what was true when they were written, and their status lines
say whether they still describe current behavior. Their file names keep their
original form so links stay stable.
