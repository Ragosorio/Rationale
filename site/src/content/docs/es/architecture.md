---
lang: es
slug: architecture
title: Arquitectura factual
description: Cómo se conectan Git, Codebase Memory, el store canónico, la recuperación derivada, MCP y la revisión humana.
section: Proyecto
order: 10
---

## La frontera

```text
Git + Codebase Memory → resolución del target → packet de contexto Rationale
                                      ↓
                           cambio del agente → captura
                                      ↓
                         propuesta pendiente → revisión humana
                                      ↓
                              Record canónico
```

Git aporta revisión y hechos del diff. Codebase Memory aporta ubicación y
relaciones estructurales opcionales. Rationale posee Subjects, Records,
Evidence y el lifecycle de revisión canónicos.

## Canónico frente a derivado

El YAML bajo `.rationale/` tiene autoridad. La cache SQLite/FTS acelera la
recuperación y se puede reconstruir. El servidor MCP lee el modelo canónico a
través del pipeline; no lee la SQLite privada del proveedor.

## Por qué importa la frontera

El agente puede pedir contexto y capturar hechos observados, pero el protocolo
no recibe autoridad para aprobar una decisión normativa. Así la incertidumbre,
la cobertura del proveedor y la responsabilidad humana permanecen visibles.
