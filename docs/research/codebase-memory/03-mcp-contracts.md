# 03 — MCP contracts (CBM-006)

**Fuente de evidencia:** herramientas MCP activas en esta sesión, servidas por el **binario instalado (0.8.1)**. La lista exacta de herramientas disponibles en una sesión puede depender de configuración del cliente; esta es la superficie observada, no necesariamente exhaustiva (ver Unknown).

## Observed

Herramientas MCP observadas y su contrato exacto:

| Herramienta | Input requerido | Notas de contrato |
|---|---|---|
| `list_projects` | ninguno | Devuelve `{name, root_path, nodes, edges, size_bytes}` por proyecto indexado. Sin revisión ni cobertura. |
| `index_status` | `project` | `{project, nodes, edges, status}`. Sin revisión ni generación (ver `05-revision-and-coverage.md`). |
| `index_repository` | `repo_path`; opcional `mode` (`full`\|`moderate`\|`fast`\|`cross-repo-intelligence`), `persistence` (bool), `target_projects` | **Hallazgo relevante:** `persistence: true` escribe un artefacto comprimido en `.codebase-memory/graph.db.zst` "for team sharing" — CBM ya contempla compartir el índice derivado entre miembros de equipo, algo que el modelo de Rationale (`storage.canonical-vs-derived`) trata deliberadamente como no compartido por defecto. Ver Decision impact. |
| `get_architecture` | `project`; opcional `aspects[]` | Con `aspects=["clusters"]` devuelve comunidades Leiden con cohesión, `top_nodes`, `packages` (ver `02-module-map.md` — `packages` no distingue módulos en proyectos sin manifiesto). |
| `get_graph_schema` | `project` | Devuelve node labels/edge types y sus propiedades — el contrato de forma del grafo es introspectable, lo cual es valioso para el adaptador. |
| `search_graph` | `project`; combina `query` (BM25 full-text), `name_pattern` (regex), `semantic_query` (array de keywords, requiere modo moderate/full) | Tres modos independientes y combinables. Paginación explícita vía `limit`/`offset`/`has_more`/`total` — **contrato de paginación bien definido**, relevante para acotar presupuesto de tokens en el adaptador de Rationale. |
| `search_code` | `project`, `pattern` | Grep aumentado por grafo; agrupa matches en funciones contenedoras, rankea por importancia estructural. Trunca a `limit` (default 10) sin `offset` — para ver más hay que subir `limit` o acotar con `path_filter`/`file_pattern`. |
| `trace_path` | `function_name`, `project` | Modos `calls`/`data_flow`/`cross_service`; incluye `risk_labels` opcional (`CRITICAL`/`HIGH`/`MEDIUM`/`LOW` por distancia de hop) — señal de riesgo estructural preexistente que Rationale podría consumir como evidencia, no como decisión normativa. |
| `get_code_snippet` | `qualified_name`, `project` | Requiere `qualified_name` exacto obtenido de `search_graph` primero — no es una herramienta de búsqueda. |
| `query_graph` | `query` (Cypher), `project` | Techo duro de 100k filas; sin `offset`, requiere `LIMIT` explícito en el propio Cypher para queries amplias. |
| `detect_changes` | `project`; opcional `since`, `base_branch` (default `main`), `depth`, `scope` | Ver hallazgo crítico en `05-revision-and-coverage.md`: devolvió resultados vacíos en las tres pruebas realizadas pese a cambios reales verificables por Git. |
| `manage_adr` | `project`; `mode` (`get`\|`update`\|`sections`); opcional `content`, `sections[]` | **Hallazgo relevante:** el "ADR" de CBM es **un único documento markdown por proyecto**, con secciones fijas sugeridas (`PURPOSE, STACK, ARCHITECTURE, PATTERNS, TRADEOFFS, PHILOSOPHY`) — no una colección de decisiones individuales con fecha, alternativas, procedencia o supersesión. Confirmado en vivo: `manage_adr(mode="get")` sobre el proyecto CBM devolvió `{"status": "no_adr", "content": ""}`. |
| `ingest_traces` | `traces[]`, `project` | Acepta trazas de runtime para enriquecer el grafo — no probado en esta sesión (requiere datos de trazas reales). |
| `delete_project` | (no inspeccionado a fondo) | Administrativo; no ejecutado para no destruir los índices existentes usados como evidencia de esta epic. |

## Claimed

Las descripciones de herramienta (visibles como metadata MCP) documentan explícitamente reglas de uso: "usar en vez de grep", límites de paginación, requisitos de modo de índice (`semantic_query` requiere modo moderate/full), y el hint de `manage_adr` sobre cómo estructurar el contenido. El nivel de documentación embebida en el contrato de herramienta es notablemente más alto que en un CLI típico.

## Verified

- El comportamiento de `manage_adr(mode="get")` devolviendo `no_adr` es reproducible y consistente con que nunca se ha escrito un ADR de CBM para este proyecto.
- Los contratos de paginación (`search_graph`, `search_code`, `query_graph`) están documentados de forma consistente entre sí (variantes del mismo patrón `limit`/`total`/truncation).

## Unknown

- Si la lista de herramientas observada en esta sesión es exhaustiva o si el cliente MCP la filtró/configuró — no se comparó contra una lista canónica publicada por CBM.
- Qué diferencia exacta de contrato existe entre el binario release 0.8.1 y el build local en HEAD (`dev`) — no se comparó lado a lado en esta sesión (requeriría configurar una segunda conexión MCP contra el binario local, fuera de alcance inmediato).
- Cómo se comporta `index_repository(mode="cross-repo-intelligence")` en la práctica — no ejecutado (requiere múltiples proyectos target y podría modificar el estado de proyectos ya indexados usados como evidencia).

## Risk

**Medio.** El hallazgo de `persistence: true` en `index_repository` (compartir el índice derivado comprimido entre el equipo) es una tentación arquitectónica directa: **Rationale no debe adoptar este patrón para su propia capa derivada** (`Rationale_v0.5.md §26.3`, Subject `storage.canonical-vs-derived`) — el índice de CBM puede compartirse por conveniencia de rendimiento del proveedor, pero los `Assessments` de Rationale siguen siendo responsabilidad de reconstrucción local por máquina, no de sincronización de artefacto binario.

## Decision impact

- Confirma que `manage_adr` de CBM **no sustituye ni compite con** el modelo de Records de Rationale (`Rationale_v0.5.md §7.4`, §20.2): es un documento de arquitectura vivo de una sola pieza, sin procedencia por afirmación, sin autoridad por dominio, sin evidencia estructurada, sin supersesión versionada. Rationale puede eventualmente **leer** el ADR de CBM como una fuente de evidencia (`stated`/`inferred`), nunca como equivalente a un Record aprobado.
- El contrato de paginación de `search_graph`/`search_code`/`query_graph` es reutilizable como patrón de diseño para el propio `context_budget` del Context Compiler de Rationale (`Rationale_v0.5.md §18`).
- `risk_labels` en `trace_path` es una señal estructural aprovechable como **evidencia** de riesgo (`Rationale_v0.5.md §10.1`, hechos observados), nunca como una `Decision` o `Constraint` aprobada por sí sola.
- Relevante para ADR-0002 (transporte): esta superficie MCP es rica y ya paginada/acotada — refuerza la opción de consumir vía MCP en vez de CLI subprocess, pendiente de medir latencia real en CBM-011.

## Reproducir

```text
list_projects()
index_status(project="Users-roor.osorio-Desktop-codebase-memory-mcp")
get_graph_schema(project="Users-roor.osorio-Desktop-codebase-memory-mcp")
get_architecture(project="Users-roor.osorio-Desktop-codebase-memory-mcp", aspects=["clusters"])
manage_adr(project="Users-roor.osorio-Desktop-codebase-memory-mcp", mode="get")
```
