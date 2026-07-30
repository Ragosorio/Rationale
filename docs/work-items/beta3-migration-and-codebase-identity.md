# beta.3 — migración de agentes e identidad de Codebase Memory

**Estado:** validación end-to-end completa, incluida recarga real de Cursor
(2026-07-29).

## Hallazgos

1. Cursor cargó la configuración pero no resolvió el comando lógico
   `rationale` desde la aplicación gráfica.
2. Codex comprobaba solo el nombre registrado, no comando y argumentos.
3. Rationale reconstruía el nombre derivado de Codebase Memory. El proveedor
   colapsa separadores consecutivos; Rationale no.
4. `resolve_target` reindexaba en cada consulta y elegía el primer símbolo por
   nombre sin acotar por archivo.
5. El cliente MCP no correlacionaba respuestas JSON-RPC por `id`.

## Decisiones propuestas

- ADR-0016 mueve el servidor a configuración global con ruta absoluta y deja
  instrucciones/skills en el proyecto.
- La identidad del índice se obtiene de `list_projects.root_path` o de la
  respuesta pública de `index_repository`; nunca se reconstruye ni se lee
  almacenamiento privado.
- El handle público se guarda en
  `.rationale-local/codebase-memory-project.json`, ligado a la `root_path`.
  Un índice existente se consulta; durante la migración solo se indexa si un
  proceso nuevo del proveedor no reporta la ruta ni existe aún ese vínculo.
- La resolución de símbolos pasa también `file_pattern`.

## Reindexación, IDs y clones

- Rationale no persiste node IDs de Codebase Memory. Sus bindings canónicos
  son rutas/símbolos propios y sobreviven a una nueva generación del grafo.
- Reindexar puede reemplazar la base derivada y cambiar identificadores
  internos. No debe romper el canon, pero crea un estado transitorio; por eso
  se eliminó la reindexación automática por consulta.
- Dos clones del mismo remoto en rutas diferentes son dos proyectos derivados
  distintos. Rationale conserva un canon `.rationale/` por checkout.

## Unknown

La versión instalada imprime `0.8.1` en CLI mientras su handshake MCP anuncia
`0.10.0`. Esa discrepancia pertenece al proveedor y no se resuelve leyendo su
almacenamiento interno.

## Risk

Una ruta extremadamente profunda hizo fallar al proveedor al volcar un índice
con nombre derivado largo. Rationale debe degradar honestamente; no puede
corregir un límite interno inventando otro algoritmo de nombres.

## Next experiment

Publicar el tag autorizado y verificar los artefactos cross-platform de CI
antes de promover beta.3 como Release.

## Evidencia de validación local

- Formatter y clippy estricto: pasan.
- Suite completa: 273 tests pasan (unitarios, CLI, MCP, concurrencia,
  schemas y cadena de dogfood).
- Clean-room beta.2 → beta.3: Claude Code, Codex y Cursor convergen; tres
  servidores ajenos permanecen; la segunda ejecución conserva hashes
  byte-idénticos; la reversión global elimina solo Rationale.
- Paquete macOS arm64: checksum verificado y contenido esperado.
- Instalación real: `/Users/roor.osorio/.local/bin/rationale` reporta
  `v0.1.0-beta.3`; `~/.claude.json`, `~/.cursor/mcp.json` y
  `codex mcp get rationale` apuntan a esa ruta.
- MCP stdio real: `initialize` reporta beta.3 y `tools/call health` devuelve
  `provider_status=successful`, `provider_coverage=complete`.
- `prepare src/agents.rs::install` resuelve
  `Users-roor.osorio-Desktop-Rationale.src.agents.install`, no una
  coincidencia documental.
- El clon profundo usado por Cursor reporta health completo después de
  guardar su handle público local; sus entradas `.mcp.json` heredadas fueron
  retiradas y el registro global permanece.
- Cursor recargado muestra `user-rationale` conectado (`ready`), expone
  `prepare_change`, `explain_target`, `finalize_change` y `health`, y la
  invocación real de `health` devuelve `provider_status=successful` y
  `provider_coverage=complete`.
