# ADR-0006: Revision fingerprint

**Status:** proposed — pendiente de revisión cruzada independiente antes de `accepted`.
**Date:** 2026-07-25
**Deciders:** Claude Code (análisis e implementación); pendiente aprobación humana y/o revisión cruzada de otro agente
**Supersedes / Superseded by:** ninguno

## Context

`Rationale_v0.5.md §4.8` exige que cada paquete de contexto declare una revisión coherente entre Git, el proveedor estructural y las evaluaciones de Rationale, y que la herramienta degrade o rechace una respuesta en vez de servir contexto plausible pero incorrecto. `Rationale_Arquitectura_Conceptual_v0.1.md §11.4` define el Revision Coordinator como el módulo responsable de esta comparación, pero no fijaba de dónde debía obtenerse la revisión "de verdad".

`docs/research/codebase-memory/05-revision-and-coverage.md` (CBM-008) produjo evidencia empírica directa y reproducible: `detect_changes` de Codebase Memory devolvió `{"changed_files": [], "changed_count": 0}` en tres formatos distintos de `since` (`HEAD~5`, un SHA explícito, una fecha), pese a que `git diff --stat` sobre el mismo rango mostraba **200 archivos modificados, 90.964 inserciones, 3.533 eliminaciones**. Este es exactamente el escenario que `Rationale_v0.5.md §4.9` advertía en abstracto ("no se encontró una relación" ≠ "se comprobó que la relación no existe") — ahora demostrado con datos concretos.

## Decision

El **revision fingerprint** de Rationale se calcula **exclusivamente a partir de Git** (`git rev-parse HEAD`, estado del working tree, hash de contenido cuando aplique) del lado de Rationale. Cualquier señal de revisión, cobertura o cambio reportada por Codebase Memory (u otro proveedor estructural futuro) se trata como un **dato adicional de baja confianza**, nunca como la fuente autoritativa de si el código cambió.

## Evidence

- `05-revision-and-coverage.md`: `detect_changes` falló en los tres formatos de `since` probados, con 200 archivos de diferencia real no detectados.
- `08-workspaces-and-monorepos.md` (B1.2): un hallazgo independiente de la misma naturaleza — una capability de resolución de paquetes (`pass_pkgmap.c`) que existe, está activa, y aun así no produjo ninguna relación cross-package real en un monorepo genuino. Refuerza el patrón: **el proveedor puede fallar en silencio incluso cuando la capability está presente y activa**, no solo cuando está ausente.
- `12-integration-recommendation.md`: consolida ambos hallazgos como la razón de más peso para no delegar en el proveedor ninguna afirmación de "esto cambió" o "esto no cambió".

## Alternatives considered

- **Usar `detect_changes` de Codebase Memory como fuente primaria de cambios**: descartado — la evidencia de `05` demuestra que puede devolver silenciosamente cero cambios cuando existen cientos de archivos modificados reales, sin ningún error ni advertencia en la respuesta.
- **Usar la revisión indexada que reporta el proveedor (`index_status`) como snapshot de referencia**: descartado como única fuente — `index_status` ni siquiera expone una revisión de Git en la versión probada (0.8.1); cuando sí la expone (build HEAD, `04-cli-contracts.md`), sigue sin resolver el problema de `detect_changes`, que es independiente.
- **Confiar en el string de versión del binario para inferir si sus datos de revisión son confiables**: descartado — `00-source-lock.md` y `06-daemon-and-watcher.md` documentan **tres identificadores de versión inconsistentes entre sí** (`--version`, `git describe` del clon, hash de `daemon status`), ninguno utilizable para esa inferencia.

## Consequences

- El Revision Coordinator de Rationale (`Arquitectura_Conceptual_v0.1.md §11.4`) queda completamente desacoplado de la fiabilidad del proveedor estructural para su función más crítica (saber si el código cambió) — esto es una ventaja de robustez, no una limitación aceptada a regañadientes.
- Codebase Memory sigue siendo consultado para su función correcta: estructura, símbolos, relaciones, impacto — nunca para "¿qué cambió desde la última revisión?".
- El adaptador debe registrar la revisión/generación que el proveedor reporta (cuando la reporte) únicamente como metadato de diagnóstico (`provider_generation`, `Rationale_v0.5.md §21.1`), nunca como entrada de una decisión de invalidación.
- Cualquier `Assessment` de Rationale queda `stale` o `unknown` en cuanto el fingerprint de Git (calculado por Rationale) difiere del fingerprint registrado en el `Assessment`, independientemente de lo que el proveedor diga.

## Risks

- Calcular el fingerprint solo desde Git no captura cambios en archivos no versionados (ej. generados, ignorados) — aceptable, porque el modelo conceptual de Rationale ya limita su alcance a lo versionado en Git (`Rationale_v0.5.md §4.19`).
- El costo de calcular el estado del working tree (no solo `HEAD`) puede no ser trivial en repos muy grandes — pendiente de medir en Fase D con la vertical slice real, no bloqueante para esta decisión.

## Validation

Evidencia reproducible en `docs/research/codebase-memory/05-revision-and-coverage.md §Reproducir` y `08-workspaces-and-monorepos.md §Reproducir`. La vertical slice de Fase D debe incluir un test explícito: mover `HEAD` sin reindexar el proveedor y confirmar que Rationale degrada el `Assessment` a `stale`/`unknown` usando únicamente su propio cálculo de Git, sin depender de ninguna señal del proveedor (ver plan de verificación de Fase D).

**Este ADR está en estado `proposed`**, pendiente de revisión cruzada y aprobación humana.

## Revisit trigger

Reabrir si una versión futura de Codebase Memory demuestra, con la misma metodología empírica de `05-revision-and-coverage.md` repetida, que `detect_changes` deja de fallar en el mismo escenario — eso no invalidaría la arquitectura (Git seguiría siendo la fuente primaria por robustez), pero permitiría reconsiderar si vale la pena usar la señal del proveedor como aceleración opcional, nunca como reemplazo.
