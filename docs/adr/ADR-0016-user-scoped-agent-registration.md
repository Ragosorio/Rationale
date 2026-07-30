# ADR-0016: User-scoped MCP registration and convergent migration

**Status:** proposed — pendiente de revisión cruzada independiente y aprobación humana.
**Date:** 2026-07-29
**Deciders:** Codex (investigación e implementación); decisión humana pendiente
**Supersedes / Superseded by:** propone reemplazar ADR-0015 si este ADR llega a `accepted`. Mientras ambos sigan `proposed`, ninguno gobierna al otro.

## Context

ADR-0015 eligió el comando lógico `rationale` dentro de `.mcp.json` y
`.cursor/mcp.json`. Su propio `Revisit trigger` exigía reabrir la decisión si
Cursor no podía resolver el comando desde una aplicación gráfica.

La validación real falló el 2026-07-29. Cursor cargó la regla
`.cursor/rules/rationale.mdc` y vio `.cursor/mcp.json`, pero reportó el servidor
`rationale` como desconectado. El proceso gráfico no resolvía
`~/.local/bin/rationale` con el `PATH` que tenía disponible; el CLI local sí
respondía.

Codebase Memory resuelve la misma frontera registrando su MCP una vez en la
configuración del usuario de cada cliente y usando la ruta absoluta del binario
instalado. Sus archivos del proyecto no mezclan una ruta personal con la
configuración compartida.

La migración de Codex tenía además un defecto independiente: Rationale
consideraba terminado el trabajo si `codex mcp list` contenía el nombre
`rationale`, aunque el comando todavía apuntara a un build viejo.

## Decision

1. El registro MCP de Claude Code, Codex y Cursor es **por usuario**. El
   instalador usa la ruta absoluta del binario instalado en `~/.claude.json`,
   la configuración oficial administrada por `codex mcp`, y
   `~/.cursor/mcp.json`.
2. Los archivos del proyecto contienen instrucciones y skills, no la ruta del
   servidor instalado. `install-agent` retira entradas heredadas de
   `.mcp.json` y `.cursor/mcp.json` solo cuando conservan una forma que
   Rationale reconoce como propia.
3. Toda instalación es convergente: se compara comando y argumentos, no solo
   la existencia del nombre. Una entrada obsoleta se migra al binario actual.
4. La desinstalación global retira únicamente entradas que todavía apuntan al
   binario que se está desinstalando.
5. El soporte verificable de beta.3 sigue siendo Claude Code, Codex y Cursor.
   La tabla de clientes de Codebase Memory sirve como precedente de diseño,
   no como evidencia de que Rationale ya soporte todos sus clientes.
6. Cuando una versión de Codebase Memory no persiste `root_path` entre
   procesos, Rationale guarda en `.rationale-local/` el nombre público que
   devuelve `index_repository`, junto con la raíz canónica. No guarda node IDs
   ni accede al almacenamiento del proveedor.

## Evidence

- Cursor mostró `rationale` configurado pero desconectado mientras el CLI
  local respondía.
- Simular el `PATH` típico de una aplicación GUI no encuentra `rationale`; la
  ruta absoluta instalada sí existe.
- La configuración global de Cursor de Codebase Memory usa una ruta absoluta.
- `codex mcp get rationale` permite detectar un comando obsoleto que
  `codex mcp list` no distingue.

## Consequences

- Instalar o actualizar repara los tres clientes soportados sin editar cada
  repositorio.
- Reiniciar el cliente sigue siendo necesario.
- Dos usuarios del mismo repositorio pueden tener rutas de instalación
  distintas sin producir diffs.
- Un checkout clonado pero sin Rationale instalado obtiene instrucciones, no
  un servidor inexistente.

## Risks

- Los formatos globales son contratos externos y una versión futura podría
  cambiarlos.
- Cada integrante debe ejecutar el instalador una vez.
- El alcance inicial no replica las decenas de clientes soportados por
  Codebase Memory. Añadirlos exige detección, merge no destructivo, reversión
  y validación real por cliente.

## Validation

Antes de publicar beta.3:

1. Instalar sobre un HOME aislado con configuraciones preexistentes.
2. Migrar una entrada Codex con comando obsoleto.
3. Migrar un proyecto beta.2 preservando instrucciones y otros servidores.
4. Reiniciar Cursor y ejecutar `health` mediante MCP real. **Pasó el
   2026-07-29:** `user-rationale` apareció `ready`, las cuatro herramientas
   estuvieron disponibles y `health` devolvió cobertura completa.
5. Ejecutar formatter, clippy, tests y clean-room de release.

## Revisit trigger

Reabrir si un cliente soportado deja de aceptar su configuración global, si
un usuario necesita dos binarios simultáneos, o si se añade otro cliente sin
una estrategia explícita de detección, merge, reversión y prueba real.
