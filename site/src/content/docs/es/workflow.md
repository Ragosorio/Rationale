---
lang: es
slug: workflow
title: El ciclo preparar → cambiar → capturar
description: Un cambio completo con Rationale — contexto antes de editar, memoria durable después y una persona solo donde está en juego la autoridad.
section: Operar
order: 5
---

## 1. Localizar

El agente encuentra el target real con Codebase Memory: el símbolo, sus callers
y los archivos alrededor. La estructura responde *dónde* y *cómo se conecta*;
nunca decide *por qué* el código debe quedarse así.

## 2. Preparar

```text
prepare_change(target: "src/billing/retry.rs::backoff", intent: "limitar los reintentos a tres")
```

Rationale compila un packet de contexto dentro de un techo de tokens y devuelve
un `operation_id`:

- **constraints y decisiones** que gobiernan el target, con su autoridad
  (`pinned` o `normal`) y procedencia;
- **relaciones explicadas** con su estado estructural
  (`observed`, `indirect`, `orphaned`, `unknown`) y el Record que dice por qué;
- una **vecindad estructural** acotada (callers, callees, dependencias, tests)
  y el código del target;
- **conflictos con la intención**, riesgos, desconocidos y cobertura del
  proveedor.

Si algo gobierna el target, el agente debe decir si su intención lo respeta, lo
contradice o sigue indeterminada. Un conflicto léxico es una señal, no una
contradicción probada. El conocimiento gobernante nunca se recorta para caber
en el presupuesto; cuando no cabe, el packet lo dice con `budget_overflow`.

## 3. Cambiar

El agente hace el cambio mínimo coherente con ese contexto y corre los tests
relevantes. Si el código parece extrañamente complejo, primero va
`explain_target`: puede ser una valla cuyo motivo vive en el canon.

## 4. Capturar

```text
finalize_change(operation_id, summary, candidates: [...])
```

Cada candidato es conocimiento que sigue siendo cierto después del cambio:
`kind`, `statement`, un `rationale` que da la causa, `durability: durable` y
los `bindings` que gobierna. El gate de Rationale descarta ruido, notas
transitorias, rationales que repiten el statement, duplicados y candidatos sin
binding significativo — siempre con un motivo — y escribe el resto como Records
canónicos **en esa misma llamada**. No hay cola de aprobación, y sin candidatos
no hay memoria.

## 5. Decide solo lo que te toca

Las personas conservan la autoridad que importa:

- `rationale pin <record-id>` fija una regla que ningún agente puede reemplazar.
- Si un candidato intenta reemplazar un Record fijado, **no se escribe**.
  `finalize_change` devuelve un conflicto con la pregunta y el agente te
  consulta.
- Respondes con `rationale resolve <conflict-id> keep-pinned|adopt-new`, o el
  agente transmite tu respuesta literal mediante `resolve_conflict`.
  `adopt-new` exige un actor declarado, y el reemplazo hereda `pinned`.

## 6. Observar y revisar

`rationale ui` muestra cada operación en vivo: qué recibió el agente, qué
capturó, qué se descartó y qué explicaciones están en riesgo. Los Records son
YAML en Git, así que el cambio y sus razones viajan en el mismo pull request.
