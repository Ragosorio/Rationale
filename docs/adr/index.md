# Índice de ADRs

Ningún ADR puede justificarse solamente con "elegimos X porque es rápido" (`Rationale_Arquitectura_Conceptual_v0.1.md §22`). Debe contener evidencia reproducible.

Los 12 ADRs iniciales obligatorios (`Rationale_Arquitectura_Conceptual_v0.1.md §22`), todos en estado `pending` hasta que exista evidencia:

| ADR | Título | Estado | Depende de |
|---|---|---|---|
| [ADR-0001](ADR-0001-core-language.md) | Core language and toolchain | **proposed** — Rust | Fase C — spike Rust vs Go ejecutado, `docs/research/language/`. Revisión adversarial: sostiene con matices |
| [ADR-0002](ADR-0002-cbm-transport.md) | Codebase Memory transport (MCP vs CLI) | **proposed** — sesión MCP persistente | Fase B.1 — medición formal de latencia. Revisión adversarial: sostiene con matices significativos — ver corrección incorporada al ADR |
| [ADR-0003](ADR-0003-canonical-serialization.md) | Canonical serialization (YAML/JSON) | **proposed** — YAML + `yaml_serde` reemplaza `serde_yaml` | Fase E1 — probado con roundtrip real contra un Subject del repo |
| [ADR-0004](ADR-0004-derived-database.md) | Derived database | **proposed** — SQLite vía `rusqlite` | Fase E1 — ya validado en el spike de lenguaje |
| [ADR-0005](ADR-0005-cache-root-and-project-identity.md) | Cache root and project identity | **proposed** — `~/.cache/rationale/projects/<id>/` | Fase E1 — precedente medido en Codebase Memory |
| [ADR-0006](ADR-0006-revision-fingerprint.md) | Revision fingerprint | **proposed** — derivar de Git, nunca del proveedor | Fase B — CBM-008, hallazgo crítico de `detect_changes`. Revisión adversarial: sostiene |
| [ADR-0007](ADR-0007-mcp-sdk-and-protocol-version.md) | MCP SDK and protocol version | **proposed** — framing manual, `rmcp` diferido | Fase E1 — `rmcp` compilado y evaluado, requiere runtime async |
| [ADR-0008](ADR-0008-concurrency-and-locking.md) | Concurrency and locking | **proposed** — rename atómico basta, locking diferido | Fase F1 — bug real de nombre temporal encontrado y corregido, test de 8 hilos escribiendo concurrentemente verificado 15/15 |
| ADR-0009 | Baseline integration surfaces | pending | Fase F+ |
| ADR-0010 | Packaging strategy | pending | Fase J |
| ADR-0011 | Licensing and dependency policy | accepted (parcial) | Licencia MIT decidida (ver nota abajo); política de dependencias pendiente |
| [ADR-0012](ADR-0012-telemetry-and-privacy.md) | Telemetry and privacy | **proposed** — local-only, formaliza `src/evaluation.rs` | Fase E1 |

Todos los ADRs en estado `proposed` requieren revisión cruzada de otro agente y aprobación humana antes de pasar a `accepted` (`AGENTS.md §Roles y revisión cruzada`, Subject `evaluation.no-self-certification`). ADR-0001, 0002 y 0006 ya pasaron por una revisión adversarial de una sesión independiente (`docs/work-items/adversarial-review-adr-0001-0002-0006.md`) — ninguno fue aprobado ni rechazado, la decisión final queda pendiente del dueño humano del proyecto.

## Nota sobre licencia (ADR-0011, parcial)

Se decidió **MIT** para el repositorio (ver `LICENSE`), priorizando simplicidad y adopción temprana. MIT no concede patentes explícitamente.

Si Rationale avanza hacia protocolo abierto con implementaciones de terceros (`Rationale_v0.5.md §34` — condición para llamarlo protocolo: al menos dos consumidores o proveedores independientes, conformance tests, versionado), revisar si conviene añadir una concesión de patentes o evaluar relicenciar bajo Apache-2.0. Hacerlo ahora es trivial; hacerlo después de tener contribuidores externos es mucho más costoso (requiere consentimiento de todos los copyright holders).

Este ADR se considera **abierto** hasta que exista una política completa de dependencias (ver `docs/dependencies/inventory.yaml`).
