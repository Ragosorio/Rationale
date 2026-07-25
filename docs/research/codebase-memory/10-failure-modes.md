# 10 — Failure modes

Consolida los modos de fallo observados a lo largo de toda la epic, más dos pruebas adicionales deliberadas de casos límite. Referencia cruzada a los documentos donde cada uno se originó.

## Observed

| # | Modo de fallo | Fuente | Comportamiento observado |
|---|---|---|---|
| 1 | Cambios reales no detectados | `05-revision-and-coverage.md` | `detect_changes` devolvió `{"changed_files": [], "changed_count": 0}` con tres formatos de `since` distintos, pese a 200 archivos realmente modificados verificables por Git. **Fallo silencioso** — sin error, sin warning, solo un resultado vacío indistinguible de "no hubo cambios". |
| 2 | Target de test build inconsistente | `01-build-and-test.md` | `make -f Makefile.cbm test-foundation` falla en el link con decenas de símbolos `_suite_*` no resueltos. **Fallo explícito y ruidoso** (el linker aborta con mensaje de error) — a diferencia del caso 1, este sí es imposible de ignorar. |
| 3 | Tres identificadores de versión inconsistentes entre sí | `00`, `01`, `06` | `--version` → `0.8.1` (release) / `dev` (build local); `git describe` → `v0.9.0-338-g97ce23f9`; `daemon status` → `build: dev (52ddfafc803f...)`. Ninguno de los tres puede usarse para inferir compatibilidad de capacidades del otro. |
| 4 | Cobertura cross-package ausente sin señal de alerta | `08-workspaces-and-monorepos.md` | Cero relaciones `IMPORTS` cruzan paquetes en un monorepo real de 8 paquetes npm. La consulta no devuelve error ni advertencia — simplemente no hay edges, y nada en la respuesta indica "esto podría estar incompleto". |
| 5 | Query Cypher inválida | Prueba directa en esta sesión | `query_graph` con sintaxis inválida (`"THIS IS NOT VALID CYPHER {{{"`) devuelve `{"error": "expected token type 0, got 85 at pos 0"}"` — **error explícito, correcto en el sentido de que no fabrica un resultado**, pero expone detalle de implementación interna (números de token del parser) en vez de un mensaje orientado al usuario. |
| 6 | Símbolo inexistente en `get_code_snippet` | Prueba directa en esta sesión | Con un `qualified_name` inventado, devuelve `{"error": "symbol not found. Use search_graph(name_pattern=\"...\") first to discover the exact qualified_name..."}"` — **el mejor patrón de error observado en toda la epic**: explícito, no fabrica contenido, y dice exactamente qué hacer a continuación. |
| 7 | Proyecto no encontrado | Observado repetidamente al pasar mal el parámetro `project` | Devuelve `{"error": "project not found or not indexed", "hint": "...", "available_projects": [...]}` — igualmente un buen patrón: explícito, con lista de alternativas válidas. |

## Claimed

Ninguna documentación de CBM promete explícitamente un catálogo unificado de modos de fallo o una política declarada de "fail loud vs fail silent" — se infiere únicamente de la observación caso por caso.

## Verified

Los 7 modos de fallo listados fueron todos reproducidos directamente en esta sesión de investigación (no son inferencias de segunda mano).

## Unknown

- Si existe una política interna consistente que explique por qué algunos fallos son ruidosos (2, 5, 6, 7) y otros son completamente silenciosos (1, 4) — parece más una consecuencia de qué capa de la aplicación detecta el problema (parser/validación de input vs. una consulta de grafo que simplemente no encuentra edges) que una política deliberada, pero no se confirmó con el equipo de CBM ni con más lectura de código.
- Si versiones más nuevas (posteriores a HEAD `97ce23f9`) atienden específicamente los casos 1 y 4.

## Risk

**El hallazgo transversal más importante de toda la epic:** los modos de fallo **ruidosos** (2, 5, 6, 7) son manejables — un adaptador puede capturarlos y traducirlos. Los modos **silenciosos** (1, 4) son estructuralmente peligrosos porque **no hay ninguna señal en la respuesta que distinga "no hay cambios/relaciones" de "no pude detectarlos"**. Esto es precisamente la advertencia central de `Rationale_v0.5.md §4.9` y `§19.2`: *"No se encontró una relación"* no es lo mismo que *"se comprobó que la relación no existe"*, y aquí se demostró con dos casos reales y reproducibles que el proveedor no distingue ambos internamente en su respuesta.

## Decision impact

1. **El adaptador `CodeIntelligenceProvider` de Rationale (`Rationale_v0.5.md §21`) debe tratar toda ausencia de relación o de cambio como `unknown`/`no encontrado dentro de la cobertura disponible`, nunca como confirmación negativa** — esto ya estaba en el contrato conceptual (`§19.2`) y ahora tiene dos casos empíricos concretos que lo sustentan (casos 1 y 4).
2. **Nunca propagar mensajes de error crudos del proveedor (caso 5) directamente al agente** sin normalizarlos — el adaptador debe traducir errores internos a los estados explícitos que `Rationale_Arquitectura_Conceptual_v0.1.md §11.1` exige (`supported/unsupported/degraded/unknown`), no reenviar strings como `"expected token type 0, got 85 at pos 0"`.
3. **Los patrones 6 y 7 (error explícito + sugerencia accionable + alternativas) son el estándar a igualar**, no solo evitar — el propio Subject `policy.no-inferred-blocks` de Rationale se beneficia de un proveedor que falla así de claro cuando puede.
4. Esto cierra la evidencia necesaria para escribir `12-integration-recommendation.md` con una sección explícita de manejo de fallos del adaptador.

## Reproducir

```text
query_graph(project="...", query="THIS IS NOT VALID CYPHER {{{")
get_code_snippet(project="...", qualified_name="this_function_does_not_exist_anywhere_xyz")
index_status(project="proyecto-que-no-existe")
```
