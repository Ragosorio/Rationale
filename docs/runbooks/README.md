# Runbooks

Según `Rationale_Proceso_Construccion_Agentes_v0.1.md §10.5`: build, test, install, update, uninstall, cache reset, provider failure, migration, release, diagnostics.

El núcleo real existe desde Fase D (`src/`) y Fase E lo completó (modelo canónico, capa derivada, Context Compiler, servidor MCP) — los runbooks de abajo documentan comandos reales, verificados corriéndolos, no comportamiento inventado. El runbook de release se implementa en la workflow de GitHub y scripts de `scripts/`; la matriz cross-platform y el piloto H siguen siendo gates antes de la alfa pública.

- [`build-and-test.md`](build-and-test.md)
- [`install.md`](install.md)
- [`cache-reset.md`](cache-reset.md)
- [`provider-failure.md`](provider-failure.md)
- [`diagnostics.md`](diagnostics.md)
- [`uninstall.md`](uninstall.md)
