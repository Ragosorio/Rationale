# AGENTS.md

Instrucciones operativas para cualquier agente (Claude Code, Codex u otro) que trabaje en este repositorio. Este documento es breve a propósito — no repite la arquitectura ni el proceso completo, solo indica dónde está cada cosa y qué hacer primero.

## Qué es esto y en qué fase está

Rationale completó Fase A (bootstrap), Fase B (análisis de Codebase Memory — ver `docs/work-items/EPIC-CBM-ANALYSIS.md`), Fase C (spike de lenguaje, propuso **Rust** en `docs/adr/ADR-0001-core-language.md`), Fase D (vertical slice: `init`/`health`/`prepare` reales contra Codebase Memory vía MCP persistente), Fase E (store canónico completo — `Subject`/`Evidence`/`Assessment`, capa derivada SQLite+FTS, Context Compiler con niveles de prioridad y budget, servidor MCP con `prepare_change`/`explain_target`/`health`) y **Fase F** (captura: escritura canónica atómica con fidelidad de round-trip, captura mecánica del diff, señales de alto valor y niveles 0-3, Subject Resolver completo, `finalize_change` como cuarta herramienta MCP, y `rationale review` en la CLI como única vía de aprobación humana). El núcleo real vive en `src/`; `spikes/language/` sigue siendo código de investigación desechable, no su punto de partida. Ver [`docs/architecture/code-map.md`](docs/architecture/code-map.md) para el mapa real de módulos y flujos.

Los ADRs registrados incluyen doce propuestas y una decisión parcial abierta
(ADR-0011); ninguno se autoaprueba (`evaluation.no-self-certification`). Antes
de asumir que uno describe el comportamiento actual, revisa su estado en
`docs/adr/index.md`.

F8 cerró los hallazgos P1/P2 de la auditoría adversarial: autoridad declarada
por proyecto, `novelty_reason` estructurada, claim atómico de propuestas,
diagnóstico de YAML corrupto, drift documental y CI Linux/macOS. Fase G está en
dogfood formal; las decisiones F8 ya tienen propuestas pendientes capturadas en
`.rationale/proposals/`, pero ninguna aprobación automática. Ver
`docs/work-items/fase-g-dogfood.md` para la evidencia y los límites conocidos.

Siguiente: revisión humana de esas propuestas y de los nueve Subjects
fundacionales; `review_record` ya tiene lifecycle completo (corregir,
disputar, revocar, superseder, cambiar autoridad y añadir evidencia) por CLI
interactiva. El siguiente gate es el dogfood instalable y el piloto H — ver
`docs/work-items/` por el plan vigente.

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

### Cumplir las condiciones técnicas no es autorización

Un plan aprobado que dice «cuando pase la validación, haz el merge» describe un
orden, **no concede permiso anticipado**. Cumplir la condición técnica devuelve
la decisión al humano; no la ejecuta por él. Antes de fusionar a `main`,
publicar, o modificar cualquier repositorio real —del proyecto o de un
piloto— hace falta una autorización explícita **posterior** a la evidencia,
para esa acción concreta. Una autorización no se extiende al siguiente paso ni
al siguiente repositorio.

Hallazgo real que motivó esta regla (2026-07-28, dogfood pre-beta): tras cerrar
la validación #1 de ADR-0015, el agente hizo `merge --ff-only` a `main` por su
cuenta, interpretando el plan acordado como autorización permanente. El estado
resultante era correcto y no se revirtió, pero la decisión no era del agente.
El riesgo no es el merge en sí: es que el mismo razonamiento aplicado un paso
después habría tocado repositorios con remoto.

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

<!-- rationale:begin (no editar a mano — `rationale uninstall-agent` lo revierte) -->
## Rationale — protocolo de invocación

Este proyecto usa Rationale (servidor MCP `rationale`, binario en `target/release/rationale`)
para preservar el *por qué* del código. Sigue este protocolo:

You are working in a project that uses Rationale for decision context.

Use this protocol at the start of every conversation that may change code:

1. If Codebase Memory is installed, use it first to locate the target symbol,
   its callers, and the relevant files. It tells you where the code is and
   how it connects; it does not decide why the code must remain as it is.
2. Before changing non-trivial code, call Rationale's
   `prepare_change(target, intent)` with the target you found and your actual
   intended change. Read the returned constraints, authority, evidence,
   linkage, provider coverage, and intent conflicts.
3. If the packet reports a governing constraint or a conflict with your
   intent, say so explicitly. Compare the proposed change with the Record;
   do not silently proceed and do not call an undetermined conflict a proven
   semantic contradiction. Ask for clarification when the decision is not
   yours to make.
4. If code looks unnecessarily complex, redundant, or "weird", call
   `explain_target(target)` before simplifying it. The code may be a
   Chesterton fence whose reason lives in the canon.
5. Make the smallest change consistent with the approved context. Keep
   tests, evidence, and the declared project authority in view.
6. After a non-trivial change, run the relevant tests and call
   `finalize_change(...)` so observed facts and the diff become a pending
   proposal when the capture policy requires it.
7. A proposal is not an approved Record. Never claim that a decision is
   approved until a human has completed `rationale review`.

When Codebase Memory is unavailable, continue with the coverage reported by
Rationale and state that limitation. Never invent a symbol resolution,
authority, approval, evidence, or provider result.

## Pre-made actions

`rationale install-agent` installs six project-scoped Claude Code skills. Type
`/rationale` to filter them in autocomplete:

- `/rationale-preflight <target> <intent>` — locate the target, call
  `prepare_change`, and state constraints or conflicts before editing.
- `/rationale-explain <target>` — call `explain_target` before simplifying a
  possible Chesterton fence.
- `/rationale-capture [statement]` — inject live Git context and guide
  `finalize_change` after the change.
- `/rationale-review` — list pending proposals and hand the interactive
  `rationale review` decision to the human. Agents cannot invoke this skill
  automatically.
- `/rationale-health` — combine MCP `health` with `rationale doctor --check`.
- `/rationale-protocol` — load this full protocol when project instructions
  were not loaded by the client.

The MCP server exposes the same source actions as prompts named `preflight`,
`explain`, `capture`, `review`, `health`, and `protocol`. Prompt discovery and
command decoration belong to each MCP client; do not assume a slash-command
name without verifying that client. In Codex, the portable path is to ask in
plain language, for example: “Prepare this change with Rationale for `<target>`
with intent `<intent>`.”
<!-- rationale:end -->
