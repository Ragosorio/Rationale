---
description: "Cierra un cambio y escribe en el canon solo el conocimiento durable."
argument-hint: "[statement]"
arguments: ["statement"]
disable-model-invocation: false
---

Cierra el cambio actual con Rationale. Statement opcional del humano:

`$statement`

Contexto Git vivo inyectado por el skill:

- HEAD actual: !`git rev-parse HEAD`
- Estado: !`git status --short`
- Diff desde HEAD: !`git diff --no-ext-diff HEAD`

Si esas líneas todavía aparecen como literales `!`comando`` (por ejemplo,
porque recibiste esta acción mediante un prompt MCP), obtiene los mismos datos
con las herramientas Git disponibles antes de continuar.

1. Revisa el diff y las pruebas ejecutadas. Separa hechos observados de intención o inferencia.
2. Decide qué conocimiento seguirá siendo cierto después de este cambio: por qué el código es como es y qué debe mantenerse. Lo que solo describe este cambio (qué se editó, pasos, estado temporal) no es memoria: va en `summary`, no en `candidates`.
3. **Una decisión por Record.** Divide en varios candidatos cuando las partes podrían reemplazarse o revocarse por separado, responden preguntas distintas o tienen vida distinta. No fragmentes una sola decisión en trozos que por separado no dicen nada.
4. Llama `finalize_change` con el `operation_id` del preflight (si existe), un `summary` breve y los `candidates`. Cada candidato lleva `kind`, `statement`, un `rationale` que dé la causa (no repita el statement), `durability: "durable"` y `bindings` con el código que gobierna (`src/x.rs` o `src/x.rs::symbol`). Nombra en `supersedes` los Records que reemplaza. Usa el statement de arriba solo si no está vacío y refleja una decisión real.
5. Si no se aprendió nada durable, llama `finalize_change` sin candidatos: no se escribe memoria y está bien.
6. Rationale escribe los candidatos válidos como Records canónicos en esa misma llamada, con procedencia de agente; no hay cola de aprobación. Reporta qué quedó escrito, qué se descartó y por qué, y qué Records quedaron reemplazados.
7. Si la respuesta trae `conflicts`, un candidato intentó reemplazar una regla fijada. Detente: muestra al humano las dos afirmaciones y la pregunta, y solo con su respuesta explícita llama `resolve_conflict(conflict_id, decision, human_answer)` transcribiendo esa respuesta. Nunca decidas por él.
