# ADR-0013: Auditable Record lifecycle with project-declared authority

**Status:** proposed
**Date:** 2026-07-26
**Deciders:** dueño humano del proyecto + revisión cruzada

## Context

La captura F8 produce propuestas pendientes y la aprobación inicial ya es
humana. La alfa necesita operar también sobre Records aprobados: corregirlos,
disputarlos, revocarlos, supersederlos, cambiar la autoridad y añadir
evidencia, sin borrar la historia ni elevar el rol de un actor por accidente.

## Decision proposal

- Cada mutación se registra como evento bajo `Record.lifecycle.events`.
- `revoke` prevalece sobre aprobaciones históricas.
- `supersede` establece `applicability_policy.superseded_by` y marca el
  lifecycle como `superseded`.
- Cambiar autoridad añade una aprobación auditable, pero solo para un actor y
  rol presentes en `.rationale/config.yaml`.
- Un actor no declarado no puede ejecutar lifecycle mutations ni autoelevarse.
- `review_record` es CLI interactiva; MCP permanece read-only/prepare.
- La escritura compara el contenido leído antes de sobrescribir, y aborta si
  hubo drift.

## Evidence

- `src/review.rs::mutate_record`
- `src/storage.rs::is_revoked` y `superseded_by`
- `src/assessment.rs`
- tests de disputa, revocación, superseder, evidencia y autoelevación
