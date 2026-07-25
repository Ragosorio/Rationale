# 12 — Integration recommendation (CBM-012)

Síntesis de `00` a `11`. Este documento no introduce evidencia nueva — consolida los hallazgos de la epic en una recomendación de frontera de adaptador, siguiendo el contrato `CodeIntelligenceProvider` de `Rationale_v0.5.md §21`.

## Resumen de hallazgos por severidad

### Alto impacto arquitectónico

1. **Revisión y cobertura no son confiables desde el proveedor sin verificación independiente** (`05`). `detect_changes` devolvió resultados vacíos ante 200 archivos realmente modificados. **Decisión:** el Revision Coordinator de Rationale debe derivar su propia verdad de revisión desde Git directamente, tratando cualquier señal de revisión/cambio del proveedor como un dato adicional de baja confianza, nunca como fuente autoritativa. Esto no es una novedad conceptual (ya estaba en `v0.5 §4.16`) — esta epic lo convierte de principio preventivo en necesidad demostrada empíricamente. *(Nota: B1.3 sí resolvió que el campo de cobertura `parse_partial`/`skipped`/`not_indexed` funciona correctamente vía MCP en versiones posteriores a 0.8.1 — el problema de `detect_changes` es independiente y sigue sin explicación.)*
2. **La resolución cross-package no puede asumirse — ni siquiera cuando el proveedor la soporta** (`08`, refinado en B1.2). Cero relaciones `IMPORTS` cruzan paquetes en un monorepo real de 8 paquetes npm, **pese a que `pass_pkgmap.c` está diseñado exactamente para resolver ese patrón** (`@repo/pkg`), fue introducido 3 meses antes de la indexación, y los manifiestos/imports reales del Monorepo cumplen las condiciones documentadas para que funcione. **Decisión:** el "camino de relevancia" cross-workspace que `v0.5 §19.3, §32.0` promete no puede construirse únicamente sobre edges del proveedor en la v1 — los bindings manuales/contractuales declarados por el equipo deben ser la vía primaria para relaciones cross-package, no un fallback de última instancia.
3. **Latencia de CLI y de arranque de MCP incompatible con el fast path baseline** (`04`, `11`, medido formalmente en B1.1). CLI: 2.2s–6.8s por invocación. MCP: el handshake `initialize` cuesta ~6.8s (igual que la CLI fría — es el mismo costo de arranque, no un problema de transporte), pero **una vez completado, cada llamada subsecuente en la misma sesión cuesta 15-30ms**, dentro del presupuesto de `v0.5 §20.5.2`. **Decisión:** el fast path baseline nunca debe lanzar un proceso/sesión nuevo por operación; debe depender de bindings ya resueltos localmente. Para el modo intent-aware, una **sesión MCP persistente de larga duración** (un solo `initialize` por vida del proceso de Rationale) es viable y preferible a subprocesos CLI repetidos — a favor de ADR-0002.

### Impacto medio

4. **Tres identificadores de versión inconsistentes entre sí** (`00`, `01`, `06`): `--version` (release vs `dev`), `git describe`, y el hash de `daemon status`. **Decisión:** negociación de capacidades explícita (`capabilities()`), nunca inferencia de compatibilidad a partir de parseo de versión.
5. **El "ADR" de CBM es un documento único de arquitectura, no un log de decisiones** (`03`). **Decisión:** puede consumirse como evidencia (`stated`/`inferred`), nunca como equivalente de un Record aprobado con procedencia y autoridad.
6. **El clustering estructural no revela límites de módulo/paquete reales** (`02`, `08` — el campo `packages` repite el nombre del proyecto en ambos casos, C plano y monorepo npm). **Decisión:** no usar `get_architecture` como fuente de identidad de workspace.

### Impacto bajo / informativo

7. Fuente completa de CBM es compilable en la máquina de referencia (`01`) — 2m49s, binario de 296MB.
8. Patrón de hook no bloqueante ya validado en producción por CBM (`hook_augment.c`, `06`) — referencia de diseño valiosa para un futuro hook propio de Rationale.
9. Almacenamiento derivado fuera del repo, con permisos restrictivos, sin secretos visibles (`07`) — patrón a replicar.
10. Errores explícitos con sugerencias accionables en varios casos (símbolo no encontrado, proyecto no encontrado) son el estándar a igualar; errores silenciosos (revisión, cross-package) son el riesgo real (`10`).

## Superficie recomendada a consumir

De las 14 herramientas observadas (`03`, confirmadas también vía `--help` en `04`), el adaptador inicial de Rationale (Fase D/E) debería consumir, en este orden de prioridad:

```text
Imprescindibles:
  list_projects        — identidad de proyecto
  index_status          — señal básica de salud (con las limitaciones de 05)
  get_code_snippet       — evidencia de código para Claims
  search_graph           — recuperación de candidatos de binding (determinista antes de semántica, v0.5 §19.1)
  trace_path             — evidencia de impacto/relaciones (risk_labels como señal, nunca como Decision)

Útiles con matices:
  get_architecture       — solo como señal exploratoria, nunca como fuente de identidad de workspace
  query_graph            — para casos de investigación puntual (arqueología, v0.5 §17), no en el fast path
  manage_adr             — solo lectura, como fuente de evidencia stated/inferred

No usar todavía / requieren más investigación:
  detect_changes         — no confiable según 05; Rationale debe implementar su propia detección vía Git
  index_repository(persistence=true) — evitar el patrón de compartir índice binario en el repo (07)
  ingest_traces           — no evaluado, fuera de alcance de esta epic
```

## Qué el adaptador nunca debe hacer

Reafirmado con evidencia concreta de esta epic, no solo por principio (`Rationale_Arquitectura_Conceptual_v0.1.md §7.2`):

- Leer directamente los archivos `.db` de `~/.cache/codebase-memory-mcp/` (`07`) — están fuera del contrato público y su formato puede cambiar sin aviso.
- Parsear o comparar strings de versión para inferir capacidades (`00`, `06` — tres identificadores inconsistentes lo demuestran).
- Tratar un resultado vacío de cualquier herramienta como confirmación negativa (`05`, `08`, `10`) — siempre `unknown`/`no encontrado dentro de la cobertura disponible`.
- Reenviar mensajes de error crudos del proveedor al agente sin normalizar (`10`, caso 5).

## Modos de fallo que el adaptador debe absorber

Ver tabla completa en `10-failure-modes.md`. Resumen de la política de traducción requerida:

```text
Proveedor devuelve vacío silencioso  → adaptador reporta coverage: unknown, nunca "no existe"
Proveedor devuelve error de parser   → adaptador normaliza a status: degraded, sin exponer el string crudo
Proveedor no encuentra proyecto/símbolo → adaptador propaga el hint accionable, es un buen patrón a preservar
Proveedor tarda more que el deadline  → fail open, degradar, nunca bloquear (ya en v0.5 §20.5.2)
```

## Research items resueltos (B1, previos a Fase C)

Los tres pendientes que quedaron abiertos al cerrar esta epic ya se resolvieron con evidencia directa:

1. **Latencia MCP formal (B1.1) — resuelto.** Cliente stdio propio contra el binario HEAD: `initialize` cuesta ~6.8s (una vez, igual que el costo de arranque de la CLI fría), pero cada `tools/call` subsecuente en la misma sesión cuesta 15-30ms. Ver `11-performance-observations.md`. **A favor de ADR-0002: sesión MCP persistente sobre subprocesos CLI repetidos.**
2. **Lectura de `pass_pkgmap.c` (B1.2) — resuelto, con severidad revisada al alza.** El módulo sí resuelve exactamente el patrón `@org/pkg`, existe desde 3 meses antes de la indexación del Monorepo, y las condiciones para que funcione (manifiestos, imports reales) están presentes — y aun así no produjo ninguna relación cross-package. Ver `08-workspaces-and-monorepos.md`. **El gap no es "capability ausente" sino "capability presente que falla silenciosamente" — refuerza que los bindings manuales sean la vía primaria, no el fallback, para relaciones cross-package en la v1.**
3. **Cobertura: ¿versión o transporte? (B1.3) — resuelto: es versión.** El mismo build HEAD, invocado vía MCP real (no CLI), devolvió los mismos campos de cobertura (`parse_partial`/`skipped`/`not_indexed`) vistos por CLI. El protocolo MCP no es el cuello de botella; el release 0.8.1 simplemente no los implementaba todavía. Ver `05-revision-and-coverage.md`.

Ningún research item queda pendiente antes de proceder a Fase C (spike de lenguaje).

## Conclusión

Codebase Memory es un proveedor estructural real, con una superficie de herramientas rica, bien paginada, y con patrones de ingeniería sólidos en varios frentes (hooks no bloqueantes, instalación auditable, cache con permisos correctos). Pero **no puede tratarse como oráculo de revisión, cobertura, ni identidad de workspace** — los tres hallazgos de alto impacto de esta epic son evidencia directa y reproducible de exactamente los riesgos que `Rationale_v0.5.md §4.9, §20.6` ya anticipaban de forma conceptual. La integración correcta es la que el contrato conceptual ya exigía: consumir mediante interfaz pública versionada, con negociación de capacidades, y con Rationale como la capa que decide qué de todo esto sigue siendo confiable para una revisión concreta.
