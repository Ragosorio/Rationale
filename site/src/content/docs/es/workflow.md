---
lang: es
slug: workflow
title: El ciclo proponer → revisar → aprobar
description: Un lifecycle completo de cambios con autoridad humana explícita e incertidumbre honesta.
section: Operar
order: 6
---

## 1. Localizar

Usa Codebase Memory para encontrar el símbolo, sus callers y archivos relevantes
cuando esté instalado. Trátalo como cobertura estructural, no como una
decisión.

## 2. Preparar

Llama a `prepare_change(target, intent)` antes de cambiar código no trivial.
Lee restricciones, evidencia, autoridad, linkage, cobertura del proveedor y
conflictos con la intent. Un Record gobernante obliga al agente a declarar si
el cambio se alinea o contradice la decisión; Rationale no finge que una
heurística local sea prueba semántica.

## 3. Cambiar y finalizar

Haz el cambio mínimo consistente con el contexto. Si el código parece extraño,
llama a `explain_target` antes de simplificarlo. Después llama a
`finalize_change`; captura archivos committed, staged, unstaged y untracked sin
tratar el árbol sucio como “no ocurrió nada”.

## 4. Revisar

Ejecuta:

```bash
rationale review
```

Inspecciona statement, evidencia, bindings, estado provisional y efecto. Una
propuesta puede aprobarse, corregirse, rechazarse o saltarse. Las afirmaciones
críticas requieren confirmar la palabra indicada.

## 5. Aprobar y compartir

La aprobación mueve el Record al área canónica `records/` con un evento humano.
Commitea `.rationale/` con el código para que la siguiente sesión vea la misma
decisión. Nunca describas una propuesta como un Record aprobado.
