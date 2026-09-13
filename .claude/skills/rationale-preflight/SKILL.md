---
description: "Prepara contexto y conflictos de gobernanza antes de cambiar código."
argument-hint: "[target] [intent]"
arguments: ["target","intent"]
disable-model-invocation: false
---

Haz el preflight de Rationale para `$target` con esta intención real:

`$intent`

1. Si Codebase Memory está disponible, úsalo primero para localizar el símbolo, sus callers y los archivos relevantes. Declara su cobertura y warnings; no lo trates como autoridad sobre el porqué.
2. Llama `prepare_change(target: "$target", intent: "$intent")` y guarda el `operation_id` que devuelve: `finalize_change` lo usa para cerrar la misma operación.
3. Antes de tocar código, resume lo que gobierna el target: `critical_constraints` y `decisions` con su autoridad (`pinned` o `normal`) y procedencia, las `relationships` explicadas con su estado estructural (`observed`, `indirect`, `orphaned`, `unknown`) y su porqué, `intent_conflicts`, riesgos, `known_unknowns`, cobertura del proveedor y `budget_overflow` si aparece.
4. Si hay un Record gobernante o un conflicto con la intención, pronúnciate explícitamente sobre si la intención lo respeta, lo contradice o sigue indeterminada. No procedas en silencio ni conviertas solapamiento léxico en contradicción semántica probada. Un Record `pinned` lo fijó el proyecto: no lo esquives.
5. Una relación `orphaned` o `unknown` no prueba que la explicación sea falsa ni que la relación haya desaparecido; repórtala como incertidumbre.
6. Si falta autoridad para decidir, detente y pide la decisión humana concreta.
