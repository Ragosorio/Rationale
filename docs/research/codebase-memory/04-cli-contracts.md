# 04 — CLI contracts (CBM-007)

**Fuente de evidencia:** binario **compilado localmente desde HEAD** (`build/c/codebase-memory-mcp`, reporta versión `dev`) — a diferencia de los documentos `03` y `05`, que usan el binario release 0.8.1 vía MCP. Se declara explícitamente por documento cuál fuente se usó (`00-source-lock.md`).

## Observed

- `--help` confirma la misma lista de 14 herramientas vista vía MCP: `index_repository, search_graph, query_graph, trace_path, get_code_snippet, get_graph_schema, get_architecture, search_code, list_projects, delete_project, index_status, detect_changes, manage_adr, ingest_traces` — el contrato de herramientas es idéntico entre transporte CLI y MCP (mismo binario, dos frontends).
- Subcomandos de nivel superior: `cli`, `install`, `uninstall`, `update`, `config`, `--version`, `--help`, más `daemon start/stop` (descubierto en uso, no listado en `--help` principal).
- El binario declara soporte "automático/condicional" para **43 superficies de cliente** (Claude Code, Codex CLI, Gemini CLI, Cursor, Windsurf, VS Code, etc.) vía su instalador — el patrón que Rationale aspira a replicar con `rationale install-agent` (`Rationale_Arquitectura_Conceptual_v0.1.md §24`).
- **`cli <tool> <json_args>` está deprecado a favor de flags, `--args-file` o stdin** — la CLI advierte activamente: `warning: passing raw JSON to 'cli index_status' is deprecated and will be removed in a future release`.
- **Hallazgo mayor — cobertura real expuesta vía CLI que no apareció vía MCP (0.8.1) en `03-mcp-contracts.md`:** `cli --json index_status --project <p>` en este build (HEAD) devuelve:
  ```json
  {
    "project": "...", "nodes": 20747, "edges": 77956, "status": "ready",
    "root_path": "/Users/.../codebase-memory-mcp",
    "parse_partial": {"files": [], "count": 0, "truncated": false},
    "skipped": {"files": [], "count": 0, "truncated": false},
    "not_indexed": {"dirs": [], "dirs_count": 0, "files": [], "files_count": 0, "truncated": false}
  }
  ```
  Esto **sí** es información de cobertura parcial estructurada (`parse_partial`, `skipped`, `not_indexed`) — justo lo que `05-revision-and-coverage.md` reportó como ausente en la llamada MCP equivalente sobre el binario 0.8.1. Ver Decision impact: esto puede significar que la cobertura mejoró entre 0.8.1 y HEAD, o que el wrapper MCP de esta sesión no solicita/expone estos campos aunque existan.
- **Latencia real medida, CLI sin daemon corriendo (temporal por invocación):** dos llamadas consecutivas de `index_status` tardaron **6.811s** y **6.873s** de tiempo total (wall clock), con ~2.2s de CPU de usuario cada una — la diferencia es overhead de arranque de un daemon temporal por invocación (confirmado por el propio mensaje de la CLI, ver abajo).
- **Latencia con daemon persistente (`daemon start` previo):** la misma llamada bajó a **2.283s** y **2.275s** — mejora real, pero sigue muy por encima de cualquier presupuesto de baseline.
- La CLI misma lo advierte: `hint: this command started a temporary CBM daemon. 'codebase-memory-mcp daemon start' keeps one warm and removes this startup cost from every CLI command.`
- `daemon start` reporta explícitamente que es **permanente**: `daemon: started (permanent, pid ...). It survives idle periods and session ends; 'codebase-memory-mcp daemon stop' retires it.`

## Claimed

La CLI se autodocumenta activamente con warnings y hints accionables en stderr (deprecaciones, sugerencia de usar el daemon persistente) — un nivel de guía en tiempo de ejecución más alto que el de un CLI típico sin esas señales.

## Verified

- La lista de herramientas vía `--help` coincide exactamente con la observada vía MCP en `03-mcp-contracts.md`, confirmando que ambos transportes exponen el mismo contrato de operaciones (aunque potencialmente distintos campos de respuesta según versión — ver el hallazgo de cobertura arriba).
- La mejora de latencia con daemon persistente (6.8s → 2.2s) es reproducible y consistente en dos corridas cada una.

## Unknown

- **Si el campo de cobertura (`parse_partial`/`skipped`/`not_indexed`) es nuevo en HEAD respecto a 0.8.1, o si simplemente el tool wrapper MCP de esta sesión no lo solicita/propaga.** No se pudo aislar la causa sin ejecutar el binario HEAD como servidor MCP y comparar la misma llamada exacta — pendiente como próximo research item antes de cerrar `12-integration-recommendation.md`.
- A qué se debe el ~2.2s restante incluso con daemon caliente: ¿arranque del propio binario cliente CLI (296MB, carga de ~180 gramáticas aunque no se usen para `index_status`), IPC al daemon, o ambos? No perfilado en detalle — fuera del alcance de este research puntual.
- Si `--tool-profile=analysis|scout` (mencionado en `--help` como "expose a restricted inspection surface") cambia el contrato de herramientas disponibles — no probado.

## Risk

**Alto y directamente accionable para el diseño de Rationale.** Ninguna de las dos rutas CLI medidas (fría: ~6.8s; con daemon persistente: ~2.2s) se acerca al presupuesto de baseline de Rationale (`Rationale_v0.5.md §20.5.2`: P50 ≤ 50ms, P95 ≤ 150ms, hard deadline ≤ 250ms). Si el fast path baseline de Rationale invocara la CLI de Codebase Memory en cada lectura o búsqueda, violaría su propio presupuesto por uno a dos órdenes de magnitud.

## Decision impact

1. **Confirma empíricamente una decisión ya tomada en el contrato conceptual**: el "Fast path baseline" (`Rationale_v0.5.md §20.5.1`) **no puede** invocar Codebase Memory (ni por CLI ni, presumiblemente, por una sesión MCP fría) en cada lectura. Debe depender exclusivamente de almacenamiento local ya resuelto por Rationale. Esto deja de ser una precaución teórica y pasa a ser un requisito respaldado por medición real.
2. Para el **modo intent-aware** (presupuesto ~2s tolerado, `Rationale_Arquitectura_Conceptual_v0.1.md §13.2`), una sesión MCP persistente (proceso de larga duración, no un `cli` subprocess por llamada) es la única ruta viable observada — refuerza la hipótesis de ADR-0002 a favor de MCP sobre CLI subprocess, pendiente de medir el mismo `index_status` a través de una sesión MCP ya viva (no un roundtrip nuevo) para tener una comparación justa.
3. El hallazgo de cobertura parcial en HEAD (si se confirma real y no un artefacto de sesión) sería una **buena noticia** para Rationale: significaría que versiones más nuevas de Codebase Memory sí exponen justo los campos que `05-revision-and-coverage.md` marcó como ausentes. Se recomienda una research note de seguimiento antes de cerrar `12-integration-recommendation.md`.

## Reproducir

```bash
cd ~/Desktop/codebase-memory-mcp
./build/c/codebase-memory-mcp --help
./build/c/codebase-memory-mcp daemon start
time ./build/c/codebase-memory-mcp cli --json index_status --project Users-roor.osorio-Desktop-codebase-memory-mcp
time ./build/c/codebase-memory-mcp cli --json index_status --project Users-roor.osorio-Desktop-codebase-memory-mcp
./build/c/codebase-memory-mcp daemon stop
```
