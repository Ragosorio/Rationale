# ADR-0009: Integration surfaces for the local MVP

**Status:** proposed
**Date:** 2026-07-26
**Deciders:** dueño humano del proyecto + revisión cruzada

## Context

Rationale tiene dos fronteras distintas: el agente necesita consultar y
preparar contexto, mientras que las decisiones canónicas requieren un humano.
Una sola interfaz que mezclara ambas cosas permitiría que una llamada MCP
mutara autoridad sin una confirmación visible.

## Decision

- MCP expone `health`, `prepare_change`, `explain_target` y
  `finalize_change`.
- MCP nunca aprueba, revoca, supersede ni cambia autoridad.
- La CLI interactiva expone `review` para propuestas y `review-record` para el
  lifecycle de Records.
- Las mutaciones escriben únicamente bajo `.rationale/` y verifican que el
  YAML no cambió mientras el humano decidía.

## Consequences

- La automatización puede preparar contexto sin convertirse en autoridad.
- Los agentes necesitan una sesión MCP y los humanos necesitan el binario CLI.
- El lifecycle requiere un terminal interactivo durante la alfa.
- Un futuro modo no interactivo necesitará otro ADR y una autorización
  explícita; no se infiere desde esta decisión.

## Evidence

- `src/mcp/server.rs`
- `src/main.rs::cmd_review` y `cmd_review_record`
- `src/review.rs::mutate_record`
- `tests/mcp_server.rs` y tests de lifecycle en `src/review.rs`
