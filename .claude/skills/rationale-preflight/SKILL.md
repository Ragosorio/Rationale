---
description: "Prepara contexto y conflictos de gobernanza antes de cambiar código."
argument-hint: "[target] [intent]"
arguments: ["target","intent"]
disable-model-invocation: false
---

Haz el preflight de Rationale para `$target` con esta intención real:

`$intent`

1. Si Codebase Memory está disponible, úsalo primero para localizar el símbolo, sus callers y los archivos relevantes. Declara su cobertura y warnings; no lo trates como autoridad sobre el porqué.
2. Llama `prepare_change(target: "$target", intent: "$intent")`.
3. Antes de tocar código, resume constraints, autoridad, evidencia, linkage, cobertura del proveedor e intent conflicts.
4. Si hay un Record gobernante o un conflicto, pronúnciate explícitamente sobre si la intención lo respeta, lo contradice o sigue indeterminada. No procedas en silencio ni conviertas solapamiento léxico en contradicción semántica probada.
5. Si falta autoridad para decidir, detente y pide la decisión humana concreta.
