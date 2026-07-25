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

## Actualización (B1.2) — la funcionalidad existe, el fallo es real y más grave de lo que parecía

Se leyó `~/Desktop/codebase-memory-mcp/src/pipeline/pass_pkgmap.c` (1.849 líneas) directamente. Hallazgos que **cierran** la duda anterior sobre la causa raíz y la reabren en un sentido más serio:

1. **El módulo existe exactamente para este caso.** Su comentario de cabecera: *"Scans discovered files for manifest files (package.json, go.mod, Cargo.toml, ...) and builds a hash table mapping bare package specifiers to resolved module QNs. This enables IMPORTS edges for non-relative imports like '@myorg/pkg'"*. Y explícitamente: *"This is what lets bare workspace imports (e.g. '@org/pkg' declared in an ignored package.json) resolve..."*.
2. **El mecanismo de resolución es correcto en el papel:** lee el campo `"name"` de cada `package.json` (`parse_package_json`), construye un mapa specifier→módulo, y usa prefix-matching (`resolve_slash_prefix`, `resolve_dot_prefix`, `resolve_backslash_prefix`) para resolver imports como `@repo/shared-types/foo` contra la entrada `@repo/shared-types`.
3. **Se verificó que las condiciones para que esto funcione están presentes en el Monorepo real:**
   - `packages/shared-types/package.json` declara `"name": "@repo/shared-types"`.
   - `packages/ui/package.json` declara `"name": "@repo/ui"`; `packages/crm-services/package.json` declara `"name": "@repo/crm-services"`.
   - `apps/web/app/types/interactions.ts` y `apps/web/app/types/kanban.ts` importan literalmente `from "@repo/shared-types"`; `apps/web/app/components/Topbar.tsx` importa `from "@repo/icons/web"` — imports bare reales, exactamente el patrón que `pass_pkgmap.c` dice resolver.
4. **La feature es anterior a la indexación, no posterior:** `pass_pkgmap.c` se introdujo el 2026-04-15 (`feat: generic package/module resolution for IMPORTS edges across 10 languages`); el `.db` del Monorepo se generó el 2026-07-22 — **más de 3 meses después**. No es un caso de "la indexación es de antes de que existiera la función".

**Conclusión revisada:** esto ya no es un gap de diseño no implementado — es una **falla reproducible de una funcionalidad que existe, está activa, y debería haber resuelto exactamente estos imports reales, y no lo hizo.** Es un modo de fallo más serio que "capability ausente": es "capability presente, silenciosamente no efectiva".

## Unknown

- **La causa exacta de por qué falló en esta indexación concreta** pese a que manifiestos y specifiers cumplen el patrón esperado. Hipótesis no descartadas: (a) el modo de indexación usado para el Monorepo (`fast`/`moderate`/`full`, no registrado en su momento) pudo haber omitido el paso `pass_pkgmap`; (b) alias de TypeScript (`tsconfig.json` "paths", ej. el prefijo `@/` visto también en el mismo código fuente para imports internos de `apps/web`) podrían interferir con el prefix-matching si el extractor no distingue ambos mecanismos; (c) un bug genuino del proveedor en este caso particular de proyecto. No se pudo aislar sin re-indexar el Monorepo con `mode="full"` explícito y comparar — **research item concreto para antes de Fase E**.
- Si `index_repository(mode="cross-repo-intelligence")` (visto en `03-mcp-contracts.md`) resuelve esto de otra forma — no probado, ese modo está documentado para **cruzar Routes/Channels entre proyectos indexados por separado**, no necesariamente para paquetes dentro del mismo repositorio.

## Risk

**Alto, y más severo tras B1.2 de lo que parecía inicialmente.** El caso de éxito cross-workspace que `Rationale_v0.5.md §32.0` usa como ejemplo canónico (`RoleBadge → @boost/auth-contracts → authorization subject → approved constraint`) depende exactamente de que el proveedor estructural resuelva esta clase de relación. La lectura de `pass_pkgmap.c` mostró que **no es un gap de diseño** — es una capability que existe, es anterior a esta indexación por meses, y aun así no resolvió imports reales que cumplen exactamente el patrón que dice soportar. Esto valida la cautela de `Rationale_v0.5.md §20.7` y `§14.1` en una forma más fuerte que "el proveedor puede no soportar esto": **el proveedor puede soportarlo y fallar silenciosamente de todos modos**, sin ningún error o advertencia visible en la respuesta.

## Decision impact

1. **Rationale no puede depender únicamente de edges `IMPORTS`/`CALLS` del proveedor estructural para construir el "camino de relevancia" cross-workspace** que `Rationale_v0.5.md §19.3` y `§32.0` prometen — ni siquiera cuando el proveedor declara soportar el mecanismo necesario. Mitigación obligatoria, no opcional: bindings manuales/contractuales declarados explícitamente en `.rationale/bindings/` como fallback (`Rationale_v0.5.md §20.7`: *"Debe aceptar bindings manuales o contractuales como fallback en el piloto"*), tratados como la vía primaria para relaciones cross-package en la v1, no como respaldo de última instancia.
2. **`provider_gap` deja de ser una precaución teórica y se confirma en su forma más peligrosa**: no basta con que Rationale detecte "el proveedor no soporta X" (eso sería manejable declarando `unsupported`) — debe asumir que **incluso una capability declarada como soportada puede no producir resultados**, y por tanto nunca tratar `0 resultados` como equivalente a `0 relaciones reales`.
3. Confirma nuevamente (tercera vez en la epic, junto a `02` y `05`) que el campo `packages` de `get_architecture` no debe usarse como fuente de identidad de workspace — ni en proyectos C sin manifiesto ni en un monorepo npm real con manifiestos correctos.
4. Research item concreto para antes de Fase E (no bloquea D): re-indexar el Monorepo con `mode="full"` explícito y repetir las mismas queries Cypher — determinaría si el modo de indexación original fue la causa, lo cual cambiaría la severidad de "bug del proveedor" a "requiere modo de indexación específico documentado por Rationale".

## Reproducir

```text
get_graph_schema(project="Users-roor.osorio-Desktop-Monorepo")
get_architecture(project="Users-roor.osorio-Desktop-Monorepo", aspects=["clusters"])
query_graph(project="Users-roor.osorio-Desktop-Monorepo",
  query="MATCH (a)-[r:IMPORTS]->(b) WHERE a.file_path STARTS WITH 'apps/web' AND NOT b.file_path STARTS WITH 'apps/web' RETURN a.file_path, b.file_path, r.local_name LIMIT 10")
query_graph(project="Users-roor.osorio-Desktop-Monorepo",
  query="MATCH (a)-[r:IMPORTS]->(b) WHERE b.file_path STARTS WITH 'packages/' RETURN a.file_path, b.file_path LIMIT 10")
```
