# 08 — Workspaces and monorepos (CBM-010)

**Fuente de evidencia:** proyecto `Users-roor.osorio-Desktop-Monorepo` (12.367 nodos / 23.331 edges), un **monorepo real de trabajo** con Turborepo y npm workspaces genuinos — no un fixture sintético. Estructura confirmada: `apps/{native,web,docs}` y `packages/{ui,crm-services,shared-types,api-client,icons,dev-mobile-bff,typescript-config}`, cada uno con su propio `package.json`, más `turbo.json` en la raíz.

## Observed

1. **`get_graph_schema` no tiene ningún label `Package` ni `Workspace`.** Existe un label `Project` con `count: 1` — el monorepo entero se modela como un único nodo de proyecto, sin nodos intermedios para cada workspace/paquete npm.
2. **El campo `packages` de cada cluster en `get_architecture(aspects=["clusters"])` repite siempre el mismo valor** (`osorio-Desktop-Monorepo`, el nombre del proyecto completo) en los 12 clusters detectados — igual que se observó para el propio repo de Codebase Memory en `02-module-map.md`. Este patrón se confirma ahora en un monorepo real con manifiestos npm genuinos, no solo en un proyecto C plano.
3. **Cero relaciones `IMPORTS` cruzan la frontera de un paquete hacia otro**, verificado con tres queries Cypher independientes:
   - `apps/web` → cualquier archivo fuera de `apps/web`: **0 resultados**.
   - Cualquier `IMPORTS` con destino dentro de `packages/*`: **10 resultados muestreados, todos intra-paquete** (ej. `packages/shared-types/src/auth.ts` → `packages/shared-types/src/permissions.ts`; `packages/api-client/src/client.ts` → `packages/api-client/src/types.ts`).
   - Ningún archivo de `apps/web` aparece como origen de un `IMPORTS` hacia `packages/ui`, `packages/crm-services`, `packages/api-client` ni ningún otro paquete compartido, pese a que estas apps casi con certeza consumen esos paquetes en tiempo de ejecución (patrón típico de monorepo Turborepo con specifiers como `@repo/ui`).
4. Sí existen 126 nodos `Route` y 90 edges `HTTP_CALLS` en este proyecto — la resolución de rutas HTTP parece capturarse independientemente de la resolución de imports entre paquetes.

## Claimed

Ninguna documentación observada en esta epic promete que `IMPORTS` resuelva specifiers de paquete de workspace (ej. `@repo/ui`) hacia el paquete físico real — es una observación empírica, no un incumplimiento de promesa documentada.

## Verified

- Los tres queries Cypher son reproducibles y consistentes entre sí: la ausencia de cruce de paquete no es un artefacto de una sola consulta, sino un patrón confirmado desde ambos extremos (buscando desde `apps/web` hacia afuera, y buscando hacia `packages/*` desde cualquier origen).
- La estructura de manifiestos (`package.json` en cada app/paquete + `turbo.json`) confirma que este es efectivamente un monorepo con límites de paquete reales y bien definidos, no una carpeta plana.

## Unknown

- **La causa raíz exacta**: si CBM no sigue la resolución de módulos de Node (workspace symlinks en `node_modules/@repo/*` apuntando a `packages/*`), si requiere un paso de configuración adicional no activado en esta indexación, o si simplemente no está implementado para specifiers de paquete (solo para imports relativos `./`/`../`). No se leyó el código de `pass_pkgmap.c` (visto en el log de build de `01-build-and-test.md`) a fondo — ese archivo es el candidato más probable para contener esta lógica y merece una research note de seguimiento antes de diseñar el adaptador definitivo.
- Si `index_repository(mode="cross-repo-intelligence")` (visto en `03-mcp-contracts.md`) resuelve esto de otra forma — no probado, ese modo está documentado para **cruzar Routes/Channels entre proyectos indexados por separado**, no necesariamente para paquetes dentro del mismo repositorio.
- Si una re-indexación con `mode="full"` explícito (vs. el modo usado originalmente para este proyecto, desconocido) cambia el resultado.

## Risk

**Alto para una promesa central del producto.** El caso de éxito cross-workspace que `Rationale_v0.5.md §32.0` usa como ejemplo canónico (`RoleBadge → @boost/auth-contracts → authorization subject → approved constraint`) depende exactamente de que el proveedor estructural resuelva esta clase de relación. La evidencia de este monorepo real sugiere que **no debe asumirse que Codebase Memory resuelve imports de workspace por defecto** — validando directamente la cautela ya expresada en `Rationale_v0.5.md §20.7` y `§14.1` (Limitación del proveedor): *"La arquitectura no asumirá que Codebase Memory resuelve perfectamente cada workspace... Toda relación entre paquetes deberá incluir coverage y provider revision"*.

## Decision impact

1. **Rationale no puede depender únicamente de edges `IMPORTS`/`CALLS` del proveedor estructural para construir el "camino de relevancia" cross-workspace** que `Rationale_v0.5.md §19.3` y `§32.0` prometen. Se necesita al menos una de estas mitigaciones, a decidir en Fase E/D con más investigación:
   - Bindings manuales/contractuales declarados explícitamente en `.rationale/bindings/` como fallback (ya contemplado en `Rationale_v0.5.md §20.7`: *"Debe aceptar bindings manuales o contractuales como fallback en el piloto"*), en vez de depender de resolución automática.
   - Investigar si `pass_pkgmap.c` resuelve esto de otra forma consultable (ej. otro edge type no revisado, o requiere una bandera de indexación distinta) antes de descartar la resolución automática por completo.
2. **`provider_gap` (concepto ya anticipado en `Rationale_v0.5.md §20.7`) deja de ser una precaución teórica: aquí hay un `provider_gap` real, reproducible, documentado con evidencia** — cero relaciones cross-package en un monorepo Turborepo genuino con 8 paquetes npm reales.
3. Confirma nuevamente (tercera vez en la epic, junto a `02` y `05`) que el campo `packages` de `get_architecture` no debe usarse como fuente de identidad de workspace — ni en proyectos C sin manifiesto ni en un monorepo npm real con manifiestos correctos.
4. Antes de cerrar `12-integration-recommendation.md`, agregar como research item pendiente: leer `internal src/pipeline/pass_pkgmap.c` para entender qué sí resuelve, y probar `index_repository(mode="full")` explícito sobre una re-indexación fresca del Monorepo para descartar que el gap sea de una indexación antigua con un modo distinto.

## Reproducir

```text
get_graph_schema(project="Users-roor.osorio-Desktop-Monorepo")
get_architecture(project="Users-roor.osorio-Desktop-Monorepo", aspects=["clusters"])
query_graph(project="Users-roor.osorio-Desktop-Monorepo",
  query="MATCH (a)-[r:IMPORTS]->(b) WHERE a.file_path STARTS WITH 'apps/web' AND NOT b.file_path STARTS WITH 'apps/web' RETURN a.file_path, b.file_path, r.local_name LIMIT 10")
query_graph(project="Users-roor.osorio-Desktop-Monorepo",
  query="MATCH (a)-[r:IMPORTS]->(b) WHERE b.file_path STARTS WITH 'packages/' RETURN a.file_path, b.file_path LIMIT 10")
```
