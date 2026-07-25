# 02 — Module map (CBM-005)

**Fuente de evidencia:** mezcla explícita de dos fuentes distinguidas — (a) grafo indexado del **binario 0.8.1** vía MCP (`get_architecture`), y (b) lectura directa del **código fuente en HEAD `97ce23f9`**. Cada hallazgo indica su fuente.

## Observed

### Vía código fuente (HEAD)

`src/` tiene 17 módulos de primer nivel más `internal/cbm/` (extracción y gramáticas). Tamaño aproximado por líneas de `.c`/`.cpp`:

| Módulo | Líneas | Rol aparente |
|---|---:|---|
| `cli/` | 33.015 | El módulo más grande del proyecto. CLI, instalación, activación transaccional, estado de lanzador Windows |
| `pipeline/` | 23.973 | Núcleo de extracción: `pass_definitions`, `pass_calls`, `pass_usages`, `pass_semantic`, `pass_tests`, `pass_githistory`, `pass_gitdiff`, `pass_configures`, `pass_route_nodes`, `pass_cross_repo`, `pass_pkgmap`, `pass_k8s`, `pass_complexity`, entre otros |
| `daemon/` | 18.439 | Ciclo de vida del daemon, coordinación entre sesiones, IPC, "version cohort" |
| `mcp/` | 12.318 | Servidor MCP, supervisor de indexado, salida compacta |
| `foundation/` | 9.936 | Primitivas: arena, hash table, string interning, logging, plataforma, locks |
| `store/` | 7.799 | Persistencia (SQLite + writer propio) |
| `ui/` | 4.165 | Servidor HTTP embebido, visualización 3D del grafo |
| `cypher/` | 4.803 | Motor de consultas Cypher (usado por `query_graph`) |
| `discover/` | 3.218 | Descubrimiento de lenguaje, `.gitignore`, configuración de usuario |
| `semantic/` | 2.243 | Perfil de AST, análisis semántico |
| `graph_buffer/` | 1.843 | Buffer del grafo en memoria |
| `launcher/` | 1.684 | Lanzador (relevante para Windows) |
| `watcher/` | 1.440 | File watching |
| `simhash/` | 538 | MinHash / similitud |
| `git/` | 421 | Integración con Git — sorprendentemente pequeño dado que `pipeline/` ya contiene `pass_gitdiff.c` y `pass_githistory.c`; la lógica de Git parece repartida entre este módulo delgado y esos passes, no concentrada aquí |
| `traces/` | 142 | Trazas |

`internal/cbm/` contiene la capa de extracción multi-lenguaje: ~180 archivos `grammar_<lenguaje>.c` (uno por lenguaje soportado vía tree-sitter) más `extract_*.c` (definitions, calls, imports, usages, semantic, type_refs, env_accesses, k8s, channels) y el runtime de tree-sitter.

### Vía grafo indexado (binario 0.8.1, vía `get_architecture(aspects=["clusters"])`)

- Detección de comunidades (Leiden) sobre el grafo de llamadas produjo **13 clusters**, con cohesión entre 0.58 y 1.0.
- Los clusters de mayor tamaño (36-38 miembros) tienen como `top_nodes` funciones como `run`, `main`, `require`, `check`, `SmokeFailure`, `probe_future_generation_rendezvous` — consistente con clusters centrados en **infraestructura de testing/smoke**, no en el dominio del producto.
- **Hallazgo relevante:** el campo `packages` de cada cluster devuelve siempre el mismo valor (`osorio-Desktop-codebase-memory-mcp`, el nombre del proyecto), para los 13 clusters. No hay identificación de sub-paquetes o módulos internos vía este campo — es decir, **para un proyecto C sin manifiestos de paquete (no npm/cargo/go.mod), `get_architecture` no distingue módulos internos como paquetes separados**, solo agrupa por comunidad de llamadas.

## Claimed

Ninguna documentación en la superficie analizada promete que `packages` distinga módulos internos de un proyecto de un solo lenguaje sin manifiestos — esto es una observación, no una promesa incumplida.

## Verified

- El tamaño relativo de módulos (CLI y pipeline como los más grandes) es consistente con el propio historial de commits observado en `00-source-lock.md` (dominado por hardening de Windows, daemon, y test infra).
- Los 13 clusters y su cohesión son reproducibles con la misma llamada.

## Unknown

- Qué tan preciso es el clustering de Leiden para separar módulos de **producto** (pipeline, store, mcp) de módulos de **test infrastructure** — los `top_nodes` de los clusters más grandes sugieren que gran parte de la "arquitectura observada" vía clustering describe el andamiaje de pruebas, no el dominio funcional.
- Si `get_architecture(aspects=["all"])` (no probado, solo `overview` y `clusters`) expone una vista jerárquica más útil por módulo/carpeta en vez de por cluster de llamadas.

## Risk

**Bajo-medio.** No invalida el uso de Codebase Memory, pero confirma que **la vista arquitectónica automática no sustituye la lectura de la estructura real de carpetas** para entender los límites de módulo de un proyecto — relevante para el propio adaptador de Rationale, que deberá basar su comprensión de "módulo" en convenciones del proveedor (manifiestos, carpetas) más que asumir que el clustering estructural siempre refleja límites de dominio.

## Decision impact

- Confirma el Subject `architecture.provider-boundary`: Rationale debe tratar la salida de clustering/arquitectura de Codebase Memory como una señal más, no como la fuente de verdad sobre módulos o paquetes — especialmente en proyectos sin manifiestos de paquete claros.
- Relevante para CBM-010 (workspaces/monorepos): si el campo `packages` no distingue módulos en un repo C plano, hay que verificar específicamente en el fixture de Monorepo (`~/Desktop/Monorepo`, que sí tiene manifiestos npm/similar) si `packages` se puebla correctamente allí — ver `08-workspaces-and-monorepos.md`.

## Reproducir

```text
get_architecture(project="Users-roor.osorio-Desktop-codebase-memory-mcp", aspects=["clusters"])
```

```bash
cd ~/Desktop/codebase-memory-mcp
ls -F src/
for d in src/*/; do echo "$d: $(find "$d" -name '*.c' -o -name '*.cpp' | xargs wc -l 2>/dev/null | tail -1)"; done
```
