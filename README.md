# Rationale

Rationale es un compilador local de contexto causal y una capa de procedencia, autoridad y vigencia para agentes de programación. Conserva por qué se hicieron cambios importantes, qué decisiones y restricciones gobiernan el comportamiento del sistema, quién podía aprobarlas y qué evidencia las respalda — y compila únicamente el contexto confiable, relevante y accionable que una tarea concreta necesita.

> Git remembers what changed. Rationale remembers why it still matters.

## Estado real del proyecto

**Pre-1.0, núcleo funcional con ciclo de captura y lifecycle de revisión.** El lenguaje del núcleo está decidido (Rust, `docs/adr/ADR-0001-core-language.md`) y ya existe un binario real en `src/` que implementa el modelo canónico completo (`Subject`, `Evidence`, `Assessment`, `Record`), una capa derivada local (SQLite + FTS, invalidada por revisión de Git), un Context Compiler con niveles de prioridad y presupuesto explícito, y una superficie MCP con cuatro herramientas: `prepare_change`, `explain_target`, `health` y `finalize_change`. `finalize_change` captura mecánicamente un cambio (diff, señales de alto valor, Subject candidato) y escribe una propuesta **pendiente** — nunca aprobada automáticamente; `rationale review` y `rationale review-record` en la CLI son las únicas vías de mutación humana, con confirmación explícita y eventos auditables.

**Ningún ADR completo está `accepted` todavía** — los 12 ADRs propuestos y la decisión parcial de ADR-0011 siguen pendientes de revisión cruzada y aprobación humana (`docs/adr/index.md`, Subject `evaluation.no-self-certification`). El empaquetado de la alfa se genera para GitHub Releases; la Release dogfood vigente es `v0.0.0-dogfood.7` y precede al tag instalable `v0.1.0-alpha.1`.

## Instalación

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-installer.sh | sh
rationale init   # dentro de tu proyecto: crea .rationale/ y avisa solo a tu agente de código (Claude Code, Codex, Cursor)
```

Guía completa de cinco minutos, con el flujo real de uso: [`docs/quickstart.md`](docs/quickstart.md).

El instalador coloca el binario en `~/.local/bin` (o `RATIONALE_INSTALL_DIR`), verifica SHA-256 y conserva `.rationale/` al actualizar o desinstalar. `rationale init` detecta y configura automáticamente el agente presente en el proyecto — desactivable con `--skip-agent-config` o `RATIONALE_SKIP_AGENT_CONFIG=1`; re-ejecutable a mano con `rationale install-agent` (y reversible con `rationale uninstall-agent`).

Requiere, opcionalmente, [`codebase-memory-mcp`](docs/research/codebase-memory/) en el `PATH` como proveedor estructural — sin él, Rationale sigue funcionando pero con cobertura degradada (`provider_status: unavailable`), nunca bloquea.

Para compilar desde fuente en vez de usar el binario, ver [`docs/runbooks/install.md`](docs/runbooks/install.md).

## Documentos fundacionales (leer en este orden)

1. [`Rationale_v0.5.md`](Rationale_v0.5.md) — contrato de producto: qué es, qué problema resuelve, modelo de entidades, modelo de confianza, roadmap.
2. [`Rationale_Arquitectura_Conceptual_v0.1.md`](Rationale_Arquitectura_Conceptual_v0.1.md) — contrato técnico: fronteras, componentes, qué está decidido y qué requiere investigación.
3. [`Rationale_Proceso_Construccion_Agentes_v0.1.md`](Rationale_Proceso_Construccion_Agentes_v0.1.md) — manual operativo para agentes que construyen el proyecto.

## Para agentes

Empezar por [`AGENTS.md`](AGENTS.md), no por estos tres documentos completos. `AGENTS.md` indica qué leer según el tipo de tarea, en qué fase está el proyecto ahora mismo, y qué NO hacer.

## Más documentación

- [`docs/quickstart.md`](docs/quickstart.md) — guía de cinco minutos: qué hace, qué se instala, el flujo real de uso, cómo se quita.
- [`docs/architecture/code-map.md`](docs/architecture/code-map.md) — mapa de los módulos reales de `src/` y cómo fluyen `prepare`/`serve` de punta a punta.
- [`docs/runbooks/`](docs/runbooks/) — build, test, instalación, reseteo de cache, fallo de proveedor, diagnóstico, desinstalación.
- [`docs/adr/`](docs/adr/) — decisiones arquitectónicas con su evidencia, todas en estado `proposed`.

## Licencia

MIT. Ver [`LICENSE`](LICENSE).
