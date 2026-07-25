# Índice de ADRs

Ningún ADR puede justificarse solamente con "elegimos X porque es rápido" (`Rationale_Arquitectura_Conceptual_v0.1.md §22`). Debe contener evidencia reproducible.

Los 12 ADRs iniciales obligatorios (`Rationale_Arquitectura_Conceptual_v0.1.md §22`), todos en estado `pending` hasta que exista evidencia:

| ADR | Título | Estado | Depende de |
|---|---|---|---|
| [ADR-0001](ADR-0001-core-language.md) | Core language and toolchain | **proposed** — Rust | Fase C — spike Rust vs Go ejecutado, `docs/research/language/` |
| [ADR-0002](ADR-0002-cbm-transport.md) | Codebase Memory transport (MCP vs CLI) | **proposed** — sesión MCP persistente | Fase B.1 — medición formal de latencia |
| ADR-0003 | Canonical serialization (YAML/JSON) | pending | Fase D/E |
| ADR-0004 | Derived database | pending | Fase D/E |
| ADR-0005 | Cache root and project identity | pending | Fase D/E |
| [ADR-0006](ADR-0006-revision-fingerprint.md) | Revision fingerprint | **proposed** — derivar de Git, nunca del proveedor | Fase B — CBM-008, hallazgo crítico de `detect_changes` |
| ADR-0007 | MCP SDK and protocol version | pending | Fase D/E |
| ADR-0008 | Concurrency and locking | pending | Fase D/E |
| ADR-0009 | Baseline integration surfaces | pending | Fase D/E |
| ADR-0010 | Packaging strategy | pending | Fase J |
| ADR-0011 | Licensing and dependency policy | accepted (parcial) | Licencia MIT decidida (ver nota abajo); política de dependencias pendiente |
| ADR-0012 | Telemetry and privacy | pending | Fase D/E |

Los tres ADRs en estado `proposed` requieren revisión cruzada de otro agente y aprobación humana antes de pasar a `accepted` (`AGENTS.md §Roles y revisión cruzada`, Subject `evaluation.no-self-certification`).

## Nota sobre licencia (ADR-0011, parcial)

Se decidió **MIT** para el repositorio (ver `LICENSE`), priorizando simplicidad y adopción temprana. MIT no concede patentes explícitamente.

Si Rationale avanza hacia protocolo abierto con implementaciones de terceros (`Rationale_v0.5.md §34` — condición para llamarlo protocolo: al menos dos consumidores o proveedores independientes, conformance tests, versionado), revisar si conviene añadir una concesión de patentes o evaluar relicenciar bajo Apache-2.0. Hacerlo ahora es trivial; hacerlo después de tener contribuidores externos es mucho más costoso (requiere consentimiento de todos los copyright holders).

Este ADR se considera **abierto** hasta que exista una política completa de dependencias (ver `docs/dependencies/inventory.yaml`).
