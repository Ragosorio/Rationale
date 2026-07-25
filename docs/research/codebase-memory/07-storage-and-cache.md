# 07 — Storage and cache

**Fuente de evidencia:** inspección directa del filesystem en la máquina de referencia. No se leyó contenido interno de ninguna base SQLite (`Rationale_Arquitectura_Conceptual_v0.1.md §7.2` prohíbe depender de eso como contrato) — solo metadatos de archivo (rutas, tamaños, permisos).

## Observed

- Almacenamiento derivado por defecto: **`~/.cache/codebase-memory-mcp/`**, con un archivo SQLite por proyecto indexado: `<Project-Id-Sanitizado>.db` (+ `.db-wal`, `.db-shm` en modo WAL activo).
- El `Project-Id-Sanitizado` es la ruta absoluta del proyecto con separadores reemplazados (ej. `/Users/roor.osorio/Desktop/Monorepo` → `Users-roor.osorio-Desktop-Monorepo`) — coincide exactamente con el `name` devuelto por `list_projects`.
- Tamaños observados: 3MB–86MB por proyecto, proporcional aproximadamente al tamaño/complejidad del código indexado (el propio CBM, el más grande, pesa 83MB; Monorepo 27MB).
- Permisos del directorio de cache: `drwx------` (700, solo el usuario propietario) — comportamiento correcto por defecto.
- Config global en `~/.cache/codebase-memory-mcp/config.json`: solo `{"ui_enabled": true, "ui_port": 9749}` — sin secretos ni tokens visibles.
- `_config.db` adicional junto a los `.db` por proyecto — probablemente configuración/metadata a nivel de instalación, no inspeccionado internamente.
- `logs/cbm-daemon.log` existe dentro del mismo directorio de cache (permisos `-rw-------`, 600) — consistente con el timeout log de `hook_augment.c` documentado en `06-daemon-and-watcher.md` (`~/.cache/codebase-memory-mcp/logs/hook-augment-timeouts.log`).
- **No se encontró ningún directorio `.codebase-memory/` dentro de los repositorios indexados** (ni en `~/Desktop/Monorepo` ni en `~/Desktop/codebase-memory-mcp`) — es decir, por defecto CBM **no escribe nada dentro del propio repositorio del usuario**; toda su persistencia vive fuera, en el cache de usuario. Esto solo cambiaría si se invoca `index_repository(persistence=true)` (visto en `03-mcp-contracts.md`), que sí escribiría `.codebase-memory/graph.db.zst` dentro del repo para compartir en equipo — no se activó esa opción en ninguna indexación de esta epic.

## Claimed

Ninguna reclamación pública inspeccionada en esta epic sobre el formato exacto interno de las bases `.db` — consistente con `Rationale_Arquitectura_Conceptual_v0.1.md §7.2`, que ya asume que no debe tratarse como contrato estable.

## Verified

- La convención de nombre de archivo (`<sanitized-path>.db`) es consistente entre los 7 proyectos observados vía `list_projects` y los archivos reales en `~/.cache/codebase-memory-mcp/`.
- Los permisos restrictivos (700/600) son reales, no solo documentados.

## Unknown

- Formato interno exacto de las tablas SQLite — deliberadamente no inspeccionado (fuera de los límites de integración aceptables, `Rationale_Arquitectura_Conceptual_v0.1.md §7.2`: "Rationale no deberá leer directamente tablas internas de Codebase Memory").
- Contenido y formato exacto de `_config.db`.
- Comportamiento exacto de `persistence: true` en `index_repository` — no ejecutado (evitar modificar repos de evidencia con un artefacto compartido no solicitado).
- Política de invalidación/expiración de estos `.db` — si hay un límite de tamaño, LRU, o crecen indefinidamente con cada reindexado.

## Risk

**Bajo.** El comportamiento observado (cache fuera del repo, permisos restrictivos, sin secretos visibles en config) es consistente con buenas prácticas y no contradice ningún principio de Rationale. El único punto de atención genuino es la opción `persistence: true`, que si un desarrollador la activa sin saberlo, empezaría a versionar un artefacto binario de índice dentro del repo — algo que Rationale debe evitar activamente para su propia capa derivada.

## Decision impact

- Confirma que el patrón "capa derivada fuera del repo, regenerable, con permisos restrictivos" (`Rationale_v0.5.md §26.2`, Subject `storage.canonical-vs-derived`) es un patrón ya validado por el propio proveedor — Rationale puede adoptar una convención de nombre de cache análoga (`~/.cache/rationale/projects/<project-id>/`, ya previsto en `Rationale_Arquitectura_Conceptual_v0.1.md §10.2`) con confianza de que es un patrón probado en la misma clase de herramienta.
- Refuerza que Rationale **nunca** debe ofrecer, ni siquiera como opción, escribir su índice derivado dentro del repositorio versionado (a diferencia de la opción `persistence: true` de CBM) — mantiene la separación canónica/derivada estricta que `Rationale_v0.5.md §4.19` exige.
- ADR-0005 (Cache root and project identity): la convención de sanitización de path-a-nombre-de-archivo de CBM es un precedente razonable a evaluar, con la precaución de que rutas muy largas o con caracteres especiales podrían colisionar o truncarse — no probado aquí.

## Reproducir

```bash
ls -la ~/.cache/codebase-memory-mcp/
cat ~/.cache/codebase-memory-mcp/config.json
du -sh ~/.cache/codebase-memory-mcp/*.db
find ~/Desktop/Monorepo ~/Desktop/codebase-memory-mcp -maxdepth 1 -iname ".codebase-memory*"
```
