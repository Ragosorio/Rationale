# 06 — Daemon and watcher (CBM-009)

**Fuente de evidencia:** comportamiento observado del binario compilado en HEAD (`build/c/codebase-memory-mcp dev`) + lectura de `src/daemon/`, `src/watcher/`, `src/cli/hook_augment.c`.

## Observed

- `daemon <start|stop|status> [--open] [--port=N]` es el contrato completo del subcomando.
- `daemon start` crea un daemon **permanente** que sobrevive a periodos de inactividad y al fin de la sesión; requiere `daemon stop` explícito para retirarlo.
- `daemon status` (con daemon corriendo) devuelve:
  ```text
  daemon: active (permanent)
    pid: 61604
    build: dev (52ddfafc803f...)
    committed clients: 0
  ```
  **Tercer identificador de versión distinto** además de `--version` (`dev`) y el SHA de Git (`97ce23f9...`): un hash de build (`52ddfafc803f...`) que no coincide con ninguno de los otros dos. Ver Decision impact.
- `src/watcher/watcher.c` (54.449 bytes) es el módulo de file watching; no se probó en vivo en esta sesión (requeriría modificar archivos y observar reindexado incremental, fuera de alcance inmediato).
- **`src/cli/hook_augment.c` implementa exactamente el patrón "hook de augmentación no bloqueante" que `Rationale_v0.5.md §20.7` y `Rationale_Arquitectura_Conceptual_v0.1.md §6` anticipan como referencia de diseño**, con las siguientes propiedades verificadas por lectura directa del código:
  - Deadline duro en proceso: `HA_DEADLINE_MS 300` (300ms). Al dispararse el timer, el proceso llama `_exit(0)` inmediatamente.
  - Regla cardinal explícita en el comentario de cabecera: *"this NEVER blocks a tool call. Every error, timeout, missing project, or short/odd pattern path results in exit 0 with NO stdout output (a clean pass-through)"*.
  - Usa `search_graph` (SQLite puro, sin shell) en vez de `search_code` (que hace shell-out a `grep|xargs`) específicamente para mantenerse "cheap enough to run before every Grep/Glob".
  - **Observabilidad explícita del timeout:** un comentario documenta que un deadline disparado es indistinguible de "no matches" si no se registra, por lo que el handler escribe un breadcrumb a `~/.cache/codebase-memory-mcp/logs/hook-augment-timeouts.log` usando únicamente `write()`/`_exit()` async-signal-safe (fd y mensaje preparados de antemano, al armar el timer, no en el propio handler de señal).
  - El comentario referencia issues internos (`#362`, `#858`) como origen de estas decisiones — es decir, este patrón nació de bugs reales de bloqueo/opacidad, no de diseño especulativo.

## Claimed

El propio comentario de cabecera de `hook_augment.c` afirma que este diseño hace "estructuralmente imposible" que el hook deniegue una llamada de herramienta — una afirmación de diseño, no verificada aquí mediante un test de estrés (ej. forzando timeouts artificiales), pero consistente con el mecanismo descrito (deadline + exit(0) + sin salida parcial).

## Verified

- `daemon start`/`status`/`stop` se comportan exactamente como se documentan en el mensaje de la propia CLI, reproducido en dos sesiones distintas de esta investigación (ver también `04-cli-contracts.md`).
- El tercer identificador de versión (`build: dev (52ddfafc803f...)`) es reproducible en múltiples invocaciones de `daemon status`.

## Unknown

- Si `52ddfafc803f...` es un hash de contenido del binario, un build-id del compilador, o algún otro identificador — no documentado en la salida ni confirmado por lectura adicional de `version_cohort.c` (mencionado en tests como `test_version_cohort.c`, con 909 líneas — sugiere que la gestión de identidad de build entre versiones del daemon es una preocupación seria y no trivial dentro de CBM, probablemente para coordinar múltiples clientes/sesiones contra el mismo daemon).
- Comportamiento real del watcher ante cambios de archivo en vivo — no probado (requeriría modificar el repo bajo prueba y medir tiempo de reindexado incremental).
- Si `hook_augment` está expuesto o documentado como parte del contrato público estable, o es una integración específica para ciertos clientes (el propio comentario menciona "vendor hook payload", sugiriendo un formato específico por integrador).

## Risk

**Bajo, con una lección de diseño de alto valor.** No hay riesgo nuevo detectado; al contrario, este módulo es la mejor evidencia encontrada en toda la epic de que el patrón "non-blocking by default, bounded latency, observable timeout" que `Rationale_v0.5.md §20.7` propone adoptar (no como dependencia interna, sino como principio) ya fue validado en producción por un proveedor real, incluyendo el motivo concreto (issues #362, #858) que lo justificó.

## Decision impact

1. **Confirma directamente y con evidencia de primera mano** el principio ya aceptado en `Rationale_v0.5.md §20.7`: *"non-blocking by default, bounded latency, untrusted metadata as data, observable timeout/no-op, query-time correctness check"*. Rationale debe replicar el patrón (deadline duro + exit limpio + breadcrumb de timeout observable), no copiar el código.
2. El uso deliberado de `search_graph` (SQLite puro) en vez de `search_code` (shell-out) por motivos de latencia es un dato concreto a favor de que el **Fast path baseline** de Rationale evite absolutamente cualquier operación que dispare un subproceso o shell, incluso indirectamente vía el proveedor.
3. El tercer identificador de versión (build hash ≠ `--version` ≠ Git SHA) refuerza, por tercera vez en esta epic (ver `00` y `04`), que **el adaptador de Rationale no debe intentar inferir compatibilidad de capacidades a partir de ningún string de versión de Codebase Memory** — debe usar negociación de capacidades explícita (`Rationale_v0.5.md §21.2`), nunca parseo de versión.
4. Relevante para ADR-0009 (Baseline integration surfaces): si Rationale llega a ofrecer un hook propio de augmentación, `hook_augment.c` es la referencia de diseño más cercana disponible y debería revisarse en detalle antes de implementar (Fase D/E, fuera de este bootstrap).

## Reproducir

```bash
cd ~/Desktop/codebase-memory-mcp
./build/c/codebase-memory-mcp daemon start
./build/c/codebase-memory-mcp daemon status
./build/c/codebase-memory-mcp daemon stop
head -60 src/cli/hook_augment.c
wc -l src/daemon/version_cohort.c tests/test_version_cohort.c
```
