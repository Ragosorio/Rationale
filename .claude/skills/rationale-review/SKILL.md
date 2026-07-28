---
description: "Muestra propuestas pendientes y entrega la aprobación al humano."
argument-hint: ""
arguments: []
disable-model-invocation: true
---

Prepara la revisión humana de Rationale.

1. Lista los archivos YAML pendientes bajo `.rationale/proposals/` sin alterar su estado.
2. Resume qué propone cada uno y cualquier diagnóstico de YAML corrupto.
3. Indica al humano que ejecute `rationale review` en un terminal interactivo para aprobar, rechazar o saltar.
4. No ejecutes la revisión en nombre del humano, no elijas una respuesta interactiva y no afirmes aprobación antes de que exista evidencia canónica.
