# Rationale

## Proceso de construcción con agentes 0.1

### Manual operativo para Claude Code, Codex y otros agentes de programación

**Versión:** 0.1  
**Fecha de corte:** 2026-07-24  
**Arquitectura obligatoria:** `Rationale_Arquitectura_Conceptual_v0.1.md`  
**Contrato de producto obligatorio:** `Rationale_v0.5.md`

---

# 0. Propósito

Este documento define cómo varios agentes de código deben construir Rationale sin perder:

- Decisiones.
- Evidencia.
- Contexto.
- Arquitectura.
- Resultados de experimentos.
- Limitaciones.
- Riesgos.
- Motivos.
- Continuidad entre sesiones.

El proyecto será construido principalmente con:

- Claude Code.
- OpenAI Codex.
- Otros agentes compatibles.
- Revisión humana de decisiones críticas.

La herramienta podrá ser desarrollada por agentes distintos en momentos distintos.

Por eso, ningún agente debe asumir que conoce conversaciones anteriores.

El repositorio debe contener todo lo necesario para continuar.

---

# 1. Regla fundamental

> Nada importante puede existir únicamente en la conversación de un agente.

Toda decisión importante deberá terminar en:

- Código.
- Test.
- Documento.
- ADR.
- Research note.
- Experiment result.
- Rationale Record.

Según corresponda.

No se documentará por documentar.

Se documentará aquello que otro agente necesitaría para no repetir errores ni destruir decisiones.

---

# 2. Jerarquía de fuentes

Antes de actuar, un agente debe aplicar:

```text
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

Una inferencia nunca debe sobrescribir silenciosamente una decisión aprobada.

---

# 3. Protocolo de inicio de sesión

Cada agente deberá:

## Paso 1 — Identificar repositorio

```bash
git rev-parse --show-toplevel
git status --short
git branch --show-current
git rev-parse HEAD
```

## Paso 2 — Leer documentos mínimos

En este orden:

```text
AGENTS.md
Rationale_v0.5.md
Rationale_Arquitectura_Conceptual_v0.1.md
Rationale_Proceso_Construccion_Agentes_v0.1.md
ADRs relevantes
Issue/plan actual
```

No es necesario leer 5,000 líneas en cada operación trivial.

`AGENTS.md` deberá contener una ruta de lectura por tarea.

## Paso 3 — Verificar entorno

```bash
rationale health          # cuando exista
codebase-memory-mcp --version
git --version
```

## Paso 4 — Consultar Codebase Memory

Para tareas no triviales:

- Ver arquitectura.
- Buscar targets.
- Ver dependencias.
- Ver impacto.
- Ver coverage.
- Confirmar revisión.

## Paso 5 — Revisar trabajo existente

- Git status.
- Diffs.
- TODOs.
- Tests fallidos.
- Research en progreso.
- ADRs pendientes.

## Paso 6 — Declarar alcance

Antes de editar, el agente debe expresar en su plan:

- Qué va a cambiar.
- Qué no va a cambiar.
- Qué documentos necesita.
- Qué tests ejecutará.
- Qué decisiones podrían verse afectadas.

---

# 4. Roles de agentes

Un agente puede ocupar varios roles de forma secuencial.

No debe mezclarlos sin indicarlo.

## 4.1 Research Agent

Responsable de:

- Leer upstream.
- Ejecutar experimentos.
- Citar archivos.
- Separar claim de observation.
- Registrar unknowns.
- No implementar producción prematuramente.

## 4.2 Architecture Agent

Responsable de:

- Definir fronteras.
- Evaluar tradeoffs.
- Crear ADRs.
- Mantener trazabilidad.
- Evitar acoplamientos.

No aprueba solo decisiones críticas.

## 4.3 Implementation Agent

Responsable de:

- Implementar alcance aprobado.
- Crear tests.
- Respetar contratos.
- Actualizar documentación cercana.
- No ampliar scope sin registrar.

## 4.4 Review Agent

Responsable de:

- Leer diff.
- Ejecutar tests.
- Buscar contradicciones.
- Revisar seguridad.
- Revisar docs.
- Revisar performance.
- No defender la implementación por haberla creado.

Preferiblemente será:

- Otro agente.
- Otra sesión.
- Otro modelo.
- Un humano.

## 4.5 Evaluation Agent

Responsable de:

- Ejecutar harness.
- Aplicar rubrica.
- No modificar outputs.
- Registrar métricas.
- Comparar condiciones.
- Separar análisis de juicio.

## 4.6 Documentation Agent

Responsable de:

- Consolidar resultados.
- Evitar duplicación.
- Mantener enlaces.
- Actualizar changelog.
- Verificar ejemplos.
- No inventar comportamiento.

---

# 5. Separación mínima de funciones

Para cambios críticos:

```text
Research/Plan
    ↓
Implementation
    ↓
Independent Review
    ↓
Evaluation
    ↓
Human or authorized approval
```

El mismo agente puede implementar y hacer una primera self-review.

Eso no sustituye una revisión independiente.

---

# 6. Flujo de un trabajo

## 6.1 Intake

Crear o actualizar:

```text
docs/work-items/<id>.md
```

Debe contener:

- Problema.
- Objetivo.
- Non-goals.
- Base revision.
- Evidencia.
- Riesgos.
- Plan.
- Tests.
- Docs.
- Criterio de éxito.

## 6.2 Preflight

Antes de código:

- Consultar Codebase Memory.
- Consultar Rationale cuando exista.
- Identificar restricciones.
- Identificar ADRs.
- Identificar módulos.
- Registrar unknowns.

## 6.3 Investigación

Cuando exista incertidumbre:

- Crear spike.
- No contaminar producción.
- Medir.
- Comparar.
- Guardar resultados.

## 6.4 Implementación

- Cambios pequeños.
- Commits coherentes.
- Tests junto al cambio.
- Sin refactors no relacionados.
- Sin dependencias innecesarias.
- Sin secrets.

## 6.5 Self-review

El implementador revisará:

```bash
git diff --check
git diff
```

Y ejecutará:

- Formatter.
- Lint.
- Unit.
- Contract.
- Integration relevante.
- Security relevante.
- Benchmark si aplica.

## 6.6 Independent review

Otro reviewer verificará:

- Correctness.
- Scope.
- Architecture.
- Security.
- Error handling.
- Concurrency.
- Docs.
- Tests.
- Metrics.
- Backward compatibility.

## 6.7 Documentation gate

Antes de completar:

- Actualizar ADR si hubo decisión.
- Actualizar architecture si cambió frontera.
- Actualizar research si se descubrió algo.
- Actualizar runbook si cambió operación.
- Actualizar examples si cambió contrato.
- Actualizar Rationale Record cuando exista.

## 6.8 Finalize

- Ejecutar suite.
- Guardar resultados.
- Registrar revisión.
- Capturar evidence.
- Cerrar work item.
- Crear resumen de cambio.

---

# 7. Uso obligatorio de Codebase Memory

## 7.1 Durante bootstrap

Codebase Memory se utilizará para:

- Indexar su propio clon.
- Indexar Rationale.
- Comparar arquitectura declarada y observada.
- Encontrar módulos.
- Analizar impacto.
- Reducir lecturas manuales.

## 7.2 Antes de cambios

El agente deberá consultar estructura cuando:

- Toca varios módulos.
- Cambia contratos.
- Cambia almacenamiento.
- Cambia MCP.
- Cambia revisión.
- Cambia providers.
- Cambia seguridad.
- Cambia packaging.
- Cambia un Subject crítico.

## 7.3 Después de cambios

- Reindexar o esperar actualización verificada.
- Comprobar provider revision.
- Volver a consultar target.
- Confirmar impacto.
- Registrar discrepancias.

## 7.4 No confiar ciegamente

Los resultados deberán incluir:

- Coverage.
- Revision.
- Provider version.
- Warnings.

El código fuente sigue siendo evidencia primaria para el comportamiento exacto.

---

# 8. Análisis inicial de Codebase Memory

El primer epic deberá producir:

```text
EPIC-CBM-ANALYSIS
```

Subtareas:

```text
CBM-001 Clone and lock revision
CBM-002 Build on MacBook Air M4
CBM-003 Run tests
CBM-004 Index itself
CBM-005 Map modules
CBM-006 Inspect MCP
CBM-007 Inspect CLI
CBM-008 Inspect revision and coverage
CBM-009 Inspect daemon and watcher
CBM-010 Inspect workspace support
CBM-011 Measure CLI vs MCP
CBM-012 Recommend adapter boundary
```

Cada tarea tendrá evidence.

---

# 9. Selección de lenguaje

Ningún agente comenzará el núcleo definitivo antes de `ADR-0001`.

## 9.1 Spike común

Cada candidato implementará la misma función:

```text
Input:
target + intent + revision

Operations:
read one Record
open SQLite
call/mock provider
check revision
rank one constraint
emit JSON

Measurements:
startup
latency
memory
binary size
test speed
cross-platform viability
```

## 9.2 Misma carga

No se permitirá que un candidato use un demo trivial y otro implemente todo.

## 9.3 Skills y documentación

Después de elegir lenguaje:

- Buscar documentación oficial.
- Instalar o crear skills.
- Registrar versiones.
- Crear style guide.
- Crear testing guide.
- Crear security guide.
- Configurar formatter/linter.
- Crear agent instructions.

## 9.4 Política de skills

Crear una skill cuando:

- El flujo se repite.
- Tiene pasos verificables.
- Reduce errores.
- Puede mantenerse.

No crear una skill para:

- Una tarea única.
- Reemplazar documentación oficial.
- Ocultar comandos inseguros.
- Dar permisos globales.

---

# 10. Documentación obligatoria

## 10.1 AGENTS.md

Debe ser breve.

Contendrá:

- Qué leer.
- Cómo ejecutar.
- Qué no hacer.
- Quality gates.
- Link a docs.

No duplicará toda la arquitectura.

## 10.2 ADR

Formato:

```text
Context
Decision
Status
Evidence
Alternatives
Consequences
Risks
Validation
Revisit trigger
```

## 10.3 Research note

Formato:

```text
Question
Environment
Source revision
Method
Observation
Result
Limitations
Impact
Artifacts
```

## 10.4 Experiment

Formato:

```text
Hypothesis
Protocol
Dataset
Conditions
Metrics
Raw results
Analysis
Threats to validity
Decision
```

## 10.5 Runbook

Para:

- Build.
- Test.
- Install.
- Update.
- Uninstall.
- Cache reset.
- Provider failure.
- Migration.
- Release.
- Diagnostics.

## 10.6 Code comments

Comentarios deben explicar:

- Por qué.
- Invariant.
- Security reason.
- Non-obvious tradeoff.

No deben repetir syntax.

---

# 11. Registro de trabajo por agente

Cada ejecución importante deberá generar un registro local.

```json
{
  "run_id": "...",
  "agent": "codex",
  "model": "...",
  "role": "implementation",
  "task_id": "...",
  "base_revision": "...",
  "end_revision": "...",
  "started_at": "...",
  "ended_at": "...",
  "tools": [],
  "files_read": [],
  "files_changed": [],
  "tests": [],
  "metrics": {},
  "status": "..."
}
```

Ubicación:

```text
.rationale-local/runs/
```

Datos sensibles no se versionan.

Un resumen puede guardarse en el work item.

---

# 12. Autorrevisión asistida por agentes

La autorrevisión tendrá cinco pases.

## Pass 1 — Correctness

- ¿Cumple objetivo?
- ¿Maneja errores?
- ¿Tiene tests?
- ¿Rompe invariants?

## Pass 2 — Architecture

- ¿Respeta fronteras?
- ¿Agrega dependencia circular?
- ¿Acopla internals de Codebase Memory?
- ¿Duplica responsabilidad?

## Pass 3 — Security

- Paths.
- Secrets.
- Injection.
- Permissions.
- Untrusted data.
- Temp files.
- Subprocess.

## Pass 4 — Performance

- Hot path.
- Allocations.
- Queries.
- Process spawn.
- Locks.
- Timeouts.
- Cache.

## Pass 5 — Documentation

- ADR.
- Examples.
- Runbook.
- Comments.
- Changelog.
- Rationale Record.

El agente debe reportar hallazgos concretos.

No deberá escribir solamente:

```text
Looks good.
```

---

# 13. Revisión cruzada Claude Code / Codex

Cuando sea posible:

```text
Agent A implements.
Agent B reviews.
Agent A addresses.
Agent B verifies.
```

La identidad de A y B puede alternarse.

Para decisiones críticas:

- Un agente propone.
- El otro intenta falsificar.
- El humano aprueba o rechaza.

La revisión debe buscar:

- Contraejemplos.
- Race conditions.
- Stale revision.
- False confidence.
- Hidden cost.
- Cross-platform issue.
- Missing test.
- Documentation gap.

---

# 14. Métricas de construcción

Además de medir Rationale sobre tareas, se medirá su construcción.

```text
Lead time per work item
Rework cycles
Tests added
Regression count
Review findings
Documentation completeness
Architecture violations
Dependency growth
Build time
Test time
Binary size
Memory
Agent tool calls
Prompt context written manually
```

El objetivo no es maximizar commits.

Es aumentar evidencia por cambio.

---

# 15. Evaluación empírica

## 15.1 Ground truth

Cada caso histórico tendrá:

- Must know.
- Useful.
- Irrelevant.
- Dangerous falsehoods.
- Expected tests.
- Expected invariant.

El ground truth deberá prepararse antes de ejecutar condiciones.

## 15.2 Condiciones

```text
A. Código + Git
B. AGENTS/ADR/docs
C. Codebase Memory
D. Codebase Memory + Rationale
E. Prompt experto
```

## 15.3 Misma tarea

Se controlará:

- Modelo.
- Temperature cuando sea configurable.
- Base revision.
- Herramientas.
- Time budget.
- Prompt.
- Test harness.

## 15.4 Evaluación ciega

El evaluator no deberá saber qué condición produjo el resultado cuando sea posible.

## 15.5 Self-evaluation limitation

Los agentes pueden:

- Recopilar datos.
- Ejecutar tests.
- Aplicar rubricas.
- Generar análisis.

No pueden ser la única prueba.

Las conclusiones deberán apoyarse en:

- Tests.
- Ground truth.
- Comparaciones pareadas.
- Bootstrap confidence intervals.
- Review separado.
- Datos brutos.

---

# 16. Definition of Done

Un work item no está completo si falta alguno aplicable:

- Código.
- Tests.
- Formatter.
- Lint.
- Security check.
- Documentation.
- ADR.
- Research artifact.
- Metrics.
- Review.
- Reindex verification.
- Rationale finalize.
- Clean git status.

---

# 17. Quality gates por fase

## Bootstrap

- Docs existen.
- Links funcionan.
- Environment captured.
- Codebase Memory installed.
- Repo indexes.

## Research

- Source pinned.
- Commands reproducibles.
- Claims cited.
- Unknowns visibles.
- Results committed.

## Vertical slice

- End-to-end test.
- Structured errors.
- No paid service.
- Revision shown.
- Provider coverage shown.
- Packet bounded.

## Dogfood

- Rationale installed in itself.
- Records reviewed.
- Baseline measured.
- No false block.
- Recovery documented.

## Pilot

- Dataset locked.
- Conditions run.
- Raw data preserved.
- Analysis reproducible.
- Sensitive data protected.

## Packaging

- Clean machine install.
- Update.
- Uninstall.
- Rollback.
- Checksums.
- Platform matrix.
- No orphan config.

---

# 18. Git workflow

## Branches

```text
research/<topic>
spike/<topic>
feature/<topic>
fix/<topic>
docs/<topic>
release/<version>
```

## Commits

Cada commit debe:

- Ser coherente.
- Pasar tests relevantes.
- Evitar mezclar refactor.
- Tener mensaje causal.

Ejemplo:

```text
feat(revision): reject exact context when provider is behind
```

## Pull requests

Debe incluir:

- Why.
- What.
- Non-goals.
- Tests.
- Metrics.
- Docs.
- Risks.
- Rollback.
- Rationale impact.

---

# 19. Dependencias

Antes de añadir una dependencia:

- ¿Es necesaria?
- ¿Puede resolverse con estándar?
- ¿Es activa?
- ¿Licencia compatible?
- ¿Cross-platform?
- ¿Binary impact?
- ¿Supply-chain risk?
- ¿Se puede fijar?
- ¿Requiere red?
- ¿Tiene alternativa?

Se registrará en:

```text
docs/dependencies/<name>.md
```

---

# 20. Cost control

El desarrollo debe preferir:

- Ejecución local.
- Tests locales.
- Fixtures pequeñas.
- Benchmarks controlados.
- Cache.
- Reutilización de contexto.
- Modelos ya disponibles.
- AI ya contratada.

No se debe introducir:

- Base administrada.
- Vector database remota.
- Telemetría SaaS.
- Servicio de colas.
- Infraestructura permanente.

sin una decisión posterior.

---

# 21. Dogfooding

Rationale deberá usarse sobre su propio repo cuando exista:

```text
init
prepare_change
explain_target
finalize_change
health
```

Primeros Subjects sugeridos:

```text
architecture.provider-boundary
architecture.revision-consistency
policy.no-inferred-blocks
policy.local-first
storage.canonical-vs-derived
retrieval.context-budget
evaluation.no-self-certification
```

Sus Records deben ser revisados.

No se autoaprobarán.

---

# 22. Piloto en el monorepo del trabajo

El piloto real debe comenzar solo después de:

- Vertical estable.
- Security review.
- Local-only verification.
- Sensitivity support.
- No automatic write fuera de `.rationale/`.
- Backup.
- Uninstall.

El primer modo será:

```text
read-only
```

Después:

```text
assisted capture
```

No se activará bloqueo automático al inicio.

---

# 23. Packaging

El packaging se realizará después de demostrar valor.

Orden:

```text
1. Development build for macOS arm64
2. Reproducible release build
3. macOS package
4. Linux amd64
5. Linux arm64
6. Windows amd64
7. Install/update/uninstall
8. Sign/checksum
9. Release automation
```

El instalador debe ser auditable.

No ejecutará cambios ocultos.

---

# 24. Landing page

La landing page será una fase de distribución.

No forma parte del núcleo.

Antes de crearla deben existir:

- Release descargable.
- Quick start probado.
- Screenshots reales.
- Benchmarks honestos.
- Security statement.
- License.
- Changelog.
- Troubleshooting.
- Platform support real.

No se usarán métricas no verificadas.

---

# 25. Qué debe hacer un agente si encuentra una contradicción

1. Detener el supuesto afectado.
2. Registrar evidencia.
3. Crear issue o work item.
4. Reproducir.
5. Identificar documentos afectados.
6. Proponer ADR.
7. No ocultar la contradicción.
8. No modificar concepto y código en silencio.

---

# 26. Qué debe hacer un agente si no sabe

Debe escribir:

```text
Unknown:
No pude verificar X.

Evidence:
...

Risk:
...

Next experiment:
...
```

No debe completar el vacío con una explicación convincente.

---

# 27. Entregables iniciales

## Semana/iteración conceptual 1

- Repository bootstrap.
- AGENTS.md.
- Environment script.
- Codebase Memory clone.
- Source lock.
- Build note.
- Test note.

## Iteración 2

- Module map.
- MCP analysis.
- CLI analysis.
- Revision analysis.
- Monorepo analysis.

## Iteración 3

- Language spikes.
- Benchmarks.
- ADR-0001.
- Toolchain.

## Iteración 4

- Vertical slice.
- Contract tests.
- Instrumentation.
- Golden packet.

No son fechas prometidas.

Son el orden de dependencia.

---

# 28. Checklist de inicio para cualquier agente

```text
[ ] Leí AGENTS.md
[ ] Identifiqué base revision
[ ] Revisé git status
[ ] Leí el work item
[ ] Consulté Codebase Memory
[ ] Revisé coverage
[ ] Revisé ADRs
[ ] Declaré non-goals
[ ] Definí tests
[ ] Definí docs
```

# 29. Checklist de cierre

```text
[ ] Diff revisado
[ ] Tests pasan
[ ] No secrets
[ ] Docs actualizadas
[ ] ADR actualizado
[ ] Métricas guardadas
[ ] Review independiente
[ ] Codebase Memory actualizado
[ ] Revision consistency verificada
[ ] Rationale finalize ejecutado
[ ] Work item cerrado
```

---

# 30. Regla final

> Los agentes no están construyendo únicamente código.  
> Están construyendo el sistema y la memoria necesaria para que el siguiente agente pueda construirlo mejor.

El proyecto habrá aplicado correctamente este proceso cuando una nueva sesión de Claude Code, Codex u otro agente pueda:

1. Clonar el repositorio.
2. Leer instrucciones.
3. Indexarlo.
4. Comprender la arquitectura.
5. Encontrar decisiones.
6. Ejecutar tests.
7. Continuar una tarea.
8. Justificar sus cambios.
9. Dejar el proyecto más comprensible que antes.
