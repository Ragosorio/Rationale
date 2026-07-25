# 05 — Revision and coverage (CBM-008)

**Esta es la pregunta que gobierna toda la arquitectura de consistencia por revisión de Rationale** (`Rationale_v0.5.md §4.8, §12.5, §20.3`; Subject `architecture.revision-consistency`). Se analiza antes que el resto de la epic porque puede invalidar suposiciones de otros documentos.

**Fuente de evidencia:** todas las observaciones de este documento provienen del **binario instalado (0.8.1)** invocado vía las herramientas MCP activas en esta sesión (`mcp__codebase-memory-mcp__*`), no de lectura del código fuente en el clon (HEAD `97ce23f9`). Ver `00-source-lock.md` — ambas fuentes están 338 commits desalineadas y no deben mezclarse.

## Pregunta

> ¿Codebase Memory expone la revisión que indexó, distingue working tree de HEAD, y reporta cobertura parcial de forma verificable?

## Observed

1. **`index_status(project)`** devuelve únicamente:
   ```json
   {"project": "...", "nodes": 20747, "edges": 77956, "status": "ready"}
   ```
   No incluye revisión de Git, generación del índice, timestamp de indexado, ni ningún campo de cobertura.

2. **`get_architecture(project, aspects=["overview"])`** devuelve únicamente `project`, `total_nodes`, `total_edges`. Mismo vacío de revisión/cobertura.

3. **`get_graph_schema(project)`** expone la forma del grafo (node labels, edge types y sus propiedades), pero ningún nodo ni edge lleva una propiedad de revisión Git a nivel de proyecto. El nodo `File` sí tiene `last_modified` (ver punto 4) y `change_count`.

4. **`last_modified` en nodos `File` es un timestamp Unix (epoch), no un SHA de Git.** Ejemplo real:
   ```json
   {"f.name": "server.json", "f.last_modified": "1781251641", "f.change_count": "9"}
   ```
   `change_count` es un contador (posiblemente de indexaciones o de commits que tocaron el archivo) sin unidad documentada visible desde este contrato — ver Unknown.

5. **`detect_changes(project, since=X)` devolvió `{"changed_files": [], "changed_count": 0, "impacted_symbols": [], "depth": 2}` en los tres formatos de `since` probados:** `HEAD~5`, un SHA de commit explícito (`50d0cc8c`), y una fecha (`2026-01-01`).

6. **Verificación cruzada con Git real:** en el mismo clon, `git diff --stat HEAD~5..HEAD` muestra **200 archivos modificados, 90.964 inserciones, 3.533 eliminaciones** — cambios reales y sustanciales existen en ese rango. `detect_changes` los reportó como cero en los tres intentos.

## Claimed

El propio `get_graph_schema` incluye un `adr_hint` que sugiere `get_architecture(aspects=['all'])` antes de usar `manage_adr` — es decir, el producto se presenta a sí mismo como consciente de necesitar más contexto que el `overview` básico. No se encontró, en la superficie de herramientas disponible en esta sesión, ninguna documentación en vivo que prometa explícitamente "revisión indexada" o "cobertura parcial" como campos de respuesta.

## Verified

- El vacío de revisión/cobertura en `index_status` y `get_architecture` es reproducible (repetido en múltiples llamadas).
- El resultado vacío de `detect_changes` es reproducible con tres formatos de `since` distintos, y contradice directamente el estado real de Git en el mismo repositorio.

## Unknown

- **Actualización cruzada con `02-module-map.md`:** el pipeline de compilación (`src/pipeline/`) sí incluye `pass_gitdiff.c` y `pass_githistory.c` — es decir, el código fuente en HEAD contiene passes dedicados a diff y a historial de Git. Esto hace **más probable** que `detect_changes` esté implementado y funcionando internamente, y que el resultado vacío observado sea un problema específico de: (a) el binario release 0.8.1 (desactualizado respecto a HEAD), (b) el contrato del parámetro `since` tal como se expone vía MCP, o (c) una precondición no satisfecha (ej. requiere que el `project` se haya indexado con un modo que habilite estos passes). No se pudo confirmar cuál, sin acceso a probar el binario compilado localmente contra las mismas llamadas MCP en esta sesión.
- **Si el resultado vacío de `detect_changes` es un bug real del build 0.8.1, una limitación documentada, o un malentendido del contrato de la herramienta** (por ejemplo: puede que `detect_changes` compare el working tree actual contra la *última revisión indexada*, no contra el argumento `since` de forma libre; o puede requerir que el daemon/watcher esté corriendo activamente; o puede requerir reindexar primero). No se descarta ninguna de estas explicaciones sin más evidencia.
- Qué significa exactamente `change_count` en un nodo `File` (¿commits que tocaron el archivo? ¿veces que fue reindexado?) — no documentado en el schema expuesto.
- Si el build desde fuente en HEAD (`97ce23f9`, pendiente en CBM-002) expone campos de revisión/cobertura que el release 0.8.1 no expone — es decir, si esto ya fue corregido en los 338 commits posteriores.
- Si existe una herramienta MCP adicional no probada en esta sesión (la lista de herramientas disponibles pudo no ser exhaustiva) que sí reporte revisión/generación/cobertura de forma explícita.

## Risk

**Alto, y ya no es solo hipotético — se confirmó empíricamente.** `Rationale_v0.5.md §4.9` advierte exactamente este escenario: "una relación ausente puede significar que no existe... o que el índice está atrasado... o que hubo un error del proveedor" y exige diferenciar "no se encontró una relación" de "se comprobó que la relación no existe". Aquí se observó el caso más severo posible: una herramienta diseñada específicamente para detectar cambios (`detect_changes`) devolvió cero cambios cuando existían 200 archivos modificados verificables por Git. Si Rationale confiara en esta señal para decidir si degradar un `Assessment`, fallaría exactamente en el peor momento: cuando sí hubo cambios reales que deberían volver `stale` una decisión.

Esto valida directamente el principio de `Rationale_Arquitectura_Conceptual_v0.1.md §4.6` ("fallar con humildad") y refuerza por qué `Rationale_v0.5.md §4.16` exige que la frescura se compruebe activamente comparando revisiones de Git, **nunca delegando esa comprobación al proveedor estructural sin verificación independiente**.

## Actualización posterior (ver `04-cli-contracts.md`)

Al probar el mismo `index_status` vía **CLI sobre el binario compilado en HEAD** (no el release 0.8.1 usado en este documento), la respuesta **sí incluyó** campos de cobertura estructurados: `parse_partial`, `skipped`, `not_indexed` (con conteos y flag `truncated`). Esto no estaba presente en la llamada MCP sobre 0.8.1 documentada arriba. No se pudo aislar si la diferencia es de versión (mejora real entre 0.8.1 y HEAD) o de transporte (el wrapper MCP de esta sesión no solicita/propaga esos campos aunque el binario los produzca) — ver `Unknown` en `04-cli-contracts.md`. El hallazgo de `detect_changes` vacío permanece sin explicar y no se ve afectado por esto, pero **el vacío de `coverage` reportado abajo debe leerse con esta matización**: puede no ser un vacío del proveedor sino un vacío del contrato MCP tal como está expuesto en esta sesión.

## Decision impact

1. **El Revision Coordinator de Rationale no puede depender de `detect_changes` (ni de ningún campo de revisión del proveedor) como fuente de verdad sobre si el código cambió.** La revisión canónica debe derivarse directamente de Git (`git rev-parse HEAD`, `git status`, hashes de working tree) del lado de Rationale, y usarse para invalidar/degradar `Assessments` independientemente de lo que el proveedor estructural reporte o deje de reportar. Esto confirma la decisión ya tomada en `Rationale_v0.5.md §4.16` y `§15.7`, y la convierte de principio preventivo en necesidad demostrada.
2. **`Coverage` como campo (`Rationale_v0.5.md §5.3`, `§21.1`) no puede poblarse desde ninguna llamada de esta epic.** Si Codebase Memory no expone cobertura parcial de forma explícita, el adaptador de Rationale (`CodebaseMemoryProvider`) deberá inferirla indirectamente (ej. comparando `nodes`/`edges` contra un recuento esperado, o registrando `unknown` como valor por defecto) en lugar de asumir que existe un campo nativo que leer.
3. **CBM-002 (build desde fuente) se vuelve prioritario para re-verificar este hallazgo contra HEAD**, ya que el binario probado está 338 commits detrás; es posible que `detect_changes` ya esté corregido o cambiado en el código actual del repositorio.
4. Impacta directamente ADR-0006 (Revision fingerprint): el fingerprint de revisión de Rationale debe construirse enteramente del lado de Rationale (Git + working tree hash), tratando cualquier revisión reportada por el proveedor como un dato adicional de baja confianza, nunca como la fuente autoritativa.

## Reproducir

```text
# Requiere el proyecto ya indexado en Codebase Memory (MCP tools):
index_status(project="Users-roor.osorio-Desktop-codebase-memory-mcp")
get_architecture(project="Users-roor.osorio-Desktop-codebase-memory-mcp", aspects=["overview"])
get_graph_schema(project="Users-roor.osorio-Desktop-codebase-memory-mcp")
query_graph(project="...", query="MATCH (f:File) RETURN f.name, f.file_path, f.last_modified, f.change_count LIMIT 5")
detect_changes(project="...", since="HEAD~5")
detect_changes(project="...", since="<commit-sha>")
detect_changes(project="...", since="2026-01-01")
```

```bash
# Verificación cruzada independiente en el clon real:
cd ~/Desktop/codebase-memory-mcp
git diff --stat HEAD~5..HEAD
```
