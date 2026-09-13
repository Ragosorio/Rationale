---
description: "Muestra conflictos con reglas fijadas y entrega la decisión al humano."
argument-hint: ""
arguments: []
disable-model-invocation: true
allowed-tools: Bash(rationale conflicts:*)
---

Prepara la decisión humana sobre conflictos de Rationale.

Conflictos pendientes inyectados por el skill:

!`rationale conflicts`

Si la línea anterior todavía aparece como un literal `!`comando`` (por
ejemplo, mediante un prompt MCP), obtén la misma lista antes de responder.

1. Un conflicto aparece cuando un agente intentó reemplazar (`supersedes`) un Record fijado (`pinned`). Esa afirmación no se escribió en el canon: espera esta decisión.
2. Para cada conflicto, muestra la regla fijada, la afirmación nueva y la pregunta, sin inclinar la respuesta.
3. La decisión es del humano: `keep_pinned` conserva la regla fijada; `adopt_new` la reemplaza y exige autoridad declarada en `.rationale/config.yaml`.
4. Indica que puede decidir en un terminal con `rationale resolve <conflict-id> <keep-pinned|adopt-new>`. Si prefiere decidir en esta conversación, llama `resolve_conflict` solo después de su respuesta explícita y transcríbela en `human_answer`.
5. No elijas una opción en su nombre, no fijes ni desfijes Records y no afirmes una resolución antes de que la herramienta la confirme.
