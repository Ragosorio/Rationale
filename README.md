# Rationale

Rationale es un compilador local de contexto causal y una capa de procedencia, autoridad y vigencia para agentes de programación. Conserva por qué se hicieron cambios importantes, qué decisiones y restricciones gobiernan el comportamiento del sistema, quién podía aprobarlas y qué evidencia las respalda — y compila únicamente el contexto confiable, relevante y accionable que una tarea concreta necesita.

> Git remembers what changed. Rationale remembers why it still matters.

## Estado real del proyecto

**Pre-implementación.** No existe todavía núcleo, lenguaje elegido, ni producto distribuible. Este repositorio contiene:

- El contrato conceptual del producto y su arquitectura.
- El proceso operativo para construirlo con múltiples agentes (Claude Code, Codex, otros).
- El bootstrap del repositorio (Fase A) y el análisis reproducible de Codebase Memory (Fase B), su primer proveedor estructural.

No hay lenguaje de núcleo decidido todavía: esa decisión (ADR-0001) requiere evidencia de un spike comparativo, no una preferencia. Ver `Rationale_Arquitectura_Conceptual_v0.1.md §8` y `docs/research/language/`.

## Documentos fundacionales (leer en este orden)

1. [`Rationale_v0.5.md`](Rationale_v0.5.md) — contrato de producto: qué es, qué problema resuelve, modelo de entidades, modelo de confianza, roadmap.
2. [`Rationale_Arquitectura_Conceptual_v0.1.md`](Rationale_Arquitectura_Conceptual_v0.1.md) — contrato técnico: fronteras, componentes, qué está decidido y qué requiere investigación.
3. [`Rationale_Proceso_Construccion_Agentes_v0.1.md`](Rationale_Proceso_Construccion_Agentes_v0.1.md) — manual operativo para agentes que construyen el proyecto.

## Para agentes

Empezar por [`AGENTS.md`](AGENTS.md), no por estos tres documentos completos. `AGENTS.md` indica qué leer según el tipo de tarea.

## Licencia

MIT. Ver [`LICENSE`](LICENSE).
