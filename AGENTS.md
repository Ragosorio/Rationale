# AGENTS.md

Instrucciones operativas para cualquier agente (Claude Code, Codex u otro) que trabaje en este repositorio. Este documento es breve a propósito — no repite la arquitectura ni el proceso completo, solo indica dónde está cada cosa y qué hacer primero.

## Qué es esto y en qué fase está

Rationale completó Fase A (bootstrap) y Fase B (análisis de Codebase Memory) — ver `docs/work-items/EPIC-CBM-ANALYSIS.md`. Está en **Fase C/D**: el spike de lenguaje ya se ejecutó (`docs/research/language/`) y propuso **Rust** en `docs/adr/ADR-0001-core-language.md` (estado `proposed`, pendiente de revisión cruzada y aprobación humana antes de `accepted`). Aún no existe núcleo de producto — solo el spike en `spikes/language/`, que es código de investigación desechable, no el punto de partida del núcleo real (Fase D construye la vertical slice desde cero, informada por el spike, no encima de él).

## Ruta de lectura por tipo de tarea

No leas los tres documentos fundacionales completos para una tarea trivial. Lee según lo que vayas a hacer:

| Tarea | Leer |
|---|---|
| Cambio de documentación menor, fix de typo, ajuste de plantilla | Este archivo únicamente |
| Entender qué es Rationale, qué problema resuelve, modelo de datos | `Rationale_v0.5.md` |
| Decidir o tocar fronteras técnicas, componentes, integración con proveedores | `Rationale_Arquitectura_Conceptual_v0.1.md` |
| Coordinar trabajo entre agentes, roles, revisión cruzada, work items | `Rationale_Proceso_Construccion_Agentes_v0.1.md` |
| Investigar Codebase Memory | `docs/research/codebase-memory/` + `Rationale_Arquitectura_Conceptual_v0.1.md §6-7` |
| Reabrir o cuestionar la elección de lenguaje | `docs/adr/ADR-0001-core-language.md` + `docs/research/language/` — **el ADR está `proposed`, no `accepted`; no lo autoapruebes** |
| Escribir código Rust del núcleo | `docs/rust/style-guide.md`, `testing-guide.md`, `security-guide.md` |
| Crear o revisar un Record/Subject de Rationale | `Rationale_v0.5.md §5, §9, §10` + `.rationale/` |
| Diseñar el experimento de validación | `Rationale_v0.5.md §30.1` |

Para cualquier tarea no trivial, siempre lee el ADR relevante en `docs/adr/` antes de actuar.

## Protocolo de inicio de sesión

```bash
git rev-parse --show-toplevel
git status --short
git branch --show-current
git rev-parse HEAD
```

Luego: lee este archivo, identifica la ruta de lectura de tu tarea, revisa `docs/work-items/` por trabajo en curso, consulta Codebase Memory si tu tarea toca más de un módulo (ver abajo), y declara en tu plan qué vas a cambiar, qué no vas a cambiar, qué tests correrás y qué decisiones podrían verse afectadas.

## Codebase Memory

Codebase Memory ya está indexando este repo y otros (`Monorepo`, el propio clon de `codebase-memory-mcp`) mediante las herramientas `mcp__codebase-memory-mcp__*`. Úsalo antes de tocar varios módulos, contratos, almacenamiento, MCP, revisión, providers, seguridad, packaging o un Subject crítico (`Proceso §7.2`). Nunca trates su salida como verdad absoluta: toda respuesta debe declarar cobertura, revisión y warnings (`Proceso §7.4`).

## Rust (lenguaje propuesto del núcleo, ADR-0001)

```bash
export PATH="$HOME/.cargo/bin:$PATH"   # necesario en shells no interactivos — ver docs/environment/
cargo fmt --check                       # antes de cualquier commit con código Rust
cargo clippy --all-targets -- -D warnings
cargo test
```

Guías completas: [`docs/rust/style-guide.md`](docs/rust/style-guide.md), [`docs/rust/testing-guide.md`](docs/rust/testing-guide.md), [`docs/rust/security-guide.md`](docs/rust/security-guide.md). El código en `spikes/language/rust/` es investigación desechable (spike), no el punto de partida del núcleo real.

## Jerarquía de fuentes

En caso de conflicto, en este orden (`Proceso §2`):

```
1. Tests y comportamiento reproducible
2. Rationale_v0.5.md
3. ADRs aprobados
4. Arquitectura conceptual vigente
5. Rationale Records aprobados
6. Research notes verificadas
7. Plan del issue o task
8. Comentarios de código
9. Inferencias del agente
```

Una inferencia nunca sobrescribe silenciosamente una decisión aprobada.

## Qué NO hacer (`Rationale_Arquitectura_Conceptual_v0.1.md §27`)

- Empezar la landing page.
- Elegir lenguaje sin ADR.
- Copiar internals de Codebase Memory.
- Leer su SQLite privado como contrato.
- Introducir un SaaS.
- Agregar embeddings remotos obligatorios.
- Crear veinte servicios.
- Crear un daemon antes de medir necesidad.
- Bloquear cambios con inferencias.
- Aprobar automáticamente Records.
- Ocultar cobertura parcial.
- Declarar éxito usando únicamente opinión del mismo agente.
- Saltarse documentación.
- Cambiar arquitectura sin ADR.
- Empaquetar antes de validar el núcleo.
- Optimizar antes de instrumentar.

## Roles y revisión cruzada

Este proyecto se construye con **Claude Code y Codex en revisión cruzada** (`Proceso §13`): un agente implementa, el otro intenta falsificar la propuesta (buscar contraejemplos, race conditions, revisión obsoleta, falsa confianza, costo oculto, problemas cross-platform, tests o docs faltantes), y el humano aprueba o rechaza. Para cambios críticos, la separación mínima es Research/Plan → Implementation → Independent Review → Evaluation → aprobación humana (`Proceso §5`). El mismo agente puede implementar y hacer self-review; eso no sustituye la revisión independiente.

Roles posibles (`Proceso §4`): Research, Architecture, Implementation, Review, Evaluation, Documentation. Un agente puede ocupar varios secuencialmente, pero debe indicarlo.

## Convención de ramas y commits

```
research/<topic>
spike/<topic>
feature/<topic>
fix/<topic>
docs/<topic>
release/<version>
```

Commits coherentes, que pasen los tests relevantes, sin mezclar refactors no relacionados, con mensaje causal (ej. `feat(revision): reject exact context when provider is behind`).

## Quality gates y Definition of Done

Un work item no está completo si falta algo aplicable (`Proceso §16`):

```
[ ] Código
[ ] Tests
[ ] Formatter
[ ] Lint
[ ] Security check
[ ] Documentation
[ ] ADR (si hubo decisión arquitectónica)
[ ] Research artifact (si hubo investigación)
[ ] Metrics
[ ] Review independiente
[ ] Reindex verification (Codebase Memory)
[ ] Rationale finalize (cuando exista)
[ ] Clean git status
```

Nada importante puede vivir únicamente en la conversación de un agente (`Proceso §1`). Toda decisión termina en código, test, documento, ADR, research note, experiment result o Rationale Record.

Si un agente encuentra una contradicción: detener el supuesto afectado, registrar evidencia, crear work item, reproducir, identificar documentos afectados, proponer ADR — nunca ocultarla ni resolverla en silencio (`Proceso §25`).

Si un agente no sabe algo: escribir `Unknown` + `Evidence` + `Risk` + `Next experiment` — nunca rellenar el vacío con una explicación convincente (`Proceso §26`).

## Enlaces

- [`Rationale_v0.5.md`](Rationale_v0.5.md)
- [`Rationale_Arquitectura_Conceptual_v0.1.md`](Rationale_Arquitectura_Conceptual_v0.1.md)
- [`Rationale_Proceso_Construccion_Agentes_v0.1.md`](Rationale_Proceso_Construccion_Agentes_v0.1.md)
- [`docs/adr/index.md`](docs/adr/index.md)
- [`docs/work-items/EPIC-CBM-ANALYSIS.md`](docs/work-items/EPIC-CBM-ANALYSIS.md)
- [`docs/research/codebase-memory/`](docs/research/codebase-memory/)
- [`docs/research/language/spike-protocol.md`](docs/research/language/spike-protocol.md)
