# 00 — Source lock

## Observed

- Clon local en `~/Desktop/codebase-memory-mcp`, remote `origin` = `https://github.com/DeusData/codebase-memory-mcp.git`, rama `main`.
- `HEAD` = `97ce23f9827177fff3858831156e9795c6832b18`, commit del 2026-07-23, `Merge pull request #1230 from DeusData/fix/dry-run-determinism`.
- `git describe --tags` = `v0.9.0-338-g97ce23f9` → el clon está **338 commits por delante** del tag `v0.9.0`.
- Working tree del clon: limpio (`git status --short` sin salida).
- Binario instalado en `~/.local/bin/codebase-memory-mcp`, reporta versión **0.8.1** vía `--version`.
- `install.sh` (presente en el clon) descarga desde `https://github.com/${REPO}/releases/latest/download` — es decir, instala la **última release publicada**, no el HEAD del repositorio.
- Tags disponibles ordenados por fecha de creación: `v0.9.0`, `v0.8.1`, `0.8.0`, `v0.8.0`, `0.7.0`.

## Claimed

`install.sh` se documenta a sí mismo como "One-line installer for codebase-memory-mcp", con soporte de variante UI y directorio custom. No declara explícitamente una garantía de que el binario instalado corresponda al HEAD de `main`.

## Verified

- La discrepancia de versión es reproducible: `codebase-memory-mcp --version` devuelve `0.8.1` mientras `git rev-parse HEAD` en el clon devuelve un commit 338 posiciones después del tag `v0.9.0`.
- Fuente completa disponible en el clon (`src/`, `internal/`, `pkg/`, `Makefile.cbm`, `tests/`, `flake.nix`) — ver inventario en `docs/research/codebase-memory/source-lock.yaml`.
- No existe build local compilado en el árbol (`build/`, `bin/`, `out/`, `dist/`, `.build/` ausentes) — el binario en uso proviene exclusivamente del release descargado, nunca de un build desde este código fuente.

## Unknown

- Si `v0.9.0` mismo es una release "estable" publicada o un tag interno de desarrollo — no se verificó si existe una release de GitHub etiquetada `v0.9.0` con binarios adjuntos, o si "latest" apunta a `v0.8.1` porque `v0.9.0` todavía no se considera lista para publicar.
- Qué cambió funcionalmente entre `0.8.1` y el commit actual de `main` (338 commits) — no se ha revisado el changelog ni el diff.
- Si el MCP server expuesto por el binario 0.8.1 tiene el mismo contrato de herramientas que el código en HEAD.

## Risk

**Alto y directamente accionable.** Cualquier observación hecha en esta epic usando las herramientas MCP activas en esta sesión (`mcp__codebase-memory-mcp__*`) refleja el comportamiento de **0.8.1**, no el código que se lee en el clon. Si un documento de investigación posterior (CBM-005 a CBM-011) mezcla "esto es lo que until vi en el código" con "esto es lo que until observé vía MCP" sin distinguir la versión, la conclusión queda contaminada. Este es exactamente el tipo de discrepancia que `Rationale_Arquitectura_Conceptual_v0.1.md §6.2` exige distinguir explícitamente entre "binario publicado" y "build desde código fuente".

## Decision impact

- Todo documento de investigación posterior en esta epic (`01` a `12`) debe declarar explícitamente si su evidencia proviene del **binario instalado (0.8.1)** o de **lectura del código fuente en HEAD** (`97ce23f9`), y nunca presentar ambos como una sola fuente.
- CBM-002 (build desde fuente) se vuelve más importante de lo previsto: es la única forma de observar el comportamiento real de HEAD en vez de depender de una release potencialmente desactualizada.
- Impacta ADR-0002 (transporte MCP vs CLI): la negociación de capacidades del adaptador de Rationale (`ProviderCapabilities`, `Rationale_v0.5.md §21.2`) debe asumir que la versión de Codebase Memory disponible en un entorno de usuario puede estar detrás del desarrollo activo del proveedor, reforzando la necesidad de negociación de capacidades en vez de asumir un contrato fijo.

## Reproducir

```bash
cd ~/Desktop/codebase-memory-mcp
git rev-parse HEAD
git describe --tags --always
git status --short
git remote get-url origin
codebase-memory-mcp --version
```
