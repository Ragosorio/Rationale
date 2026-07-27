---
lang: es
slug: concepts
title: Conceptos esenciales
description: El modelo canónico pequeño que separa identidad, evidencia, autoridad y estado derivado.
section: Empezar
order: 2
---

## Subject

Un Subject es la identidad estable de un comportamiento, frontera o concepto.
Evita que una decisión quede atada accidentalmente a un solo archivo. Puede
tener aliases, bindings y campos extra que se preservan al serializar de ida y
vuelta.

## Record

Un Record es una afirmación versionada sobre un Subject. Contiene statement,
severidad, evidencia, bindings, información de revisión y lifecycle. Una
propuesta tiene forma de Record, pero espera una decisión humana.

## Binding

Un Binding conecta un Record con un archivo, símbolo, ruta, tabla, migración,
test o commit. Un binding de archivo puede gobernar todos los símbolos que
contiene. Los bindings estructurales son más fuertes cuando el proveedor los
resuelve; Rationale nunca inventa un identificador estructural desde el texto.

## Evidence y assessment

Evidence dice qué respalda una afirmación y dónde inspeccionarlo. Assessment es
derivado: reporta autoridad, aplicabilidad, consistencia de revisión, cobertura
del proveedor y linkage. SQLite/FTS se puede reconstruir; el YAML canónico es
la fuente de verdad.

## Aprobación y autoridad

MCP puede preparar contexto y capturar hechos observados. No puede aprobar un
Record. La CLI interactiva registra la aprobación humana compatible con la
autoridad declarada del proyecto. Una propuesta pendiente nunca se presenta
como aprobada solo porque tiene un binding o un statement convincente.

## Lifecycle

El camino normal es:

```text
localizar → preparar → cambiar → finalizar → revisar → aprobar
```

Un Record aprobado puede corregirse, disputarse, revocarse, supersederse o
recibir evidencia adicional mediante `review-record`; cada mutación deja un
evento de lifecycle auditable.
