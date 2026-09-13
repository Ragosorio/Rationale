# 06 — Daemon and watcher (CBM-009)

**Source of evidence:** observed behavior of the binary built at HEAD (`build/c/codebase-memory-mcp dev`) + reading `src/daemon/`, `src/watcher/`, and `src/cli/hook_augment.c`.

## Observed

- `daemon <start|stop|status> [--open] [--port=N]` is the subcommand's full contract.
- `daemon start` creates a **permanent** daemon that survives idle periods and the end of the session; it requires an explicit `daemon stop` to retire it.
- `daemon status` (with the daemon running) returns:
  ```text
  daemon: active (permanent)
    pid: 61604
    build: dev (52ddfafc803f...)
    committed clients: 0
  ```
  **A third, distinct version identifier** besides `--version` (`dev`) and the Git SHA (`97ce23f9...`): a build hash (`52ddfafc803f...`) that matches neither of the other two. See Decision impact.
- `src/watcher/watcher.c` (54,449 bytes) is the file-watching module; it was not tested live in this session (it would require modifying files and observing incremental re-indexing, out of immediate scope).
- **`src/cli/hook_augment.c` implements exactly the "non-blocking augmentation hook" pattern that `Rationale_v0.5.md §20.7` and `Rationale_Arquitectura_Conceptual_v0.1.md §6` anticipate as a design reference**, with the following properties verified by reading the code directly:
  - A hard in-process deadline: `HA_DEADLINE_MS 300` (300 ms). When the timer fires, the process calls `_exit(0)` immediately.
  - A cardinal rule stated in the header comment: *"this NEVER blocks a tool call. Every error, timeout, missing project, or short/odd pattern path results in exit 0 with NO stdout output (a clean pass-through)"*.
  - It uses `search_graph` (pure SQLite, no shell) instead of `search_code` (which shells out to `grep|xargs`) specifically to stay "cheap enough to run before every Grep/Glob".
  - **Explicit timeout observability:** a comment documents that a fired deadline is indistinguishable from "no matches" if it is not logged, so the handler writes a breadcrumb to `~/.cache/codebase-memory-mcp/logs/hook-augment-timeouts.log` using only async-signal-safe `write()`/`_exit()` (the fd and message prepared in advance, when arming the timer, not in the signal handler itself).
  - The comment references internal issues (`#362`, `#858`) as the origin of these decisions — that is, this pattern was born from real blocking/opacity bugs, not from speculative design.

## Claimed

The header comment of `hook_augment.c` itself states that this design makes it "structurally impossible" for the hook to deny a tool call — a design claim, not verified here through a stress test (for example, forcing artificial timeouts), but consistent with the described mechanism (deadline + exit(0) + no partial output).

## Verified

- `daemon start`/`status`/`stop` behave exactly as the CLI's own message documents them, reproduced in two different sessions of this research (see also `04-cli-contracts.md`).
- The third version identifier (`build: dev (52ddfafc803f...)`) is reproducible across multiple `daemon status` invocations.

## Unknown

- Whether `52ddfafc803f...` is a content hash of the binary, a compiler build ID, or some other identifier — not documented in the output or confirmed by further reading of `version_cohort.c` (mentioned in tests as `test_version_cohort.c`, with 909 lines — suggesting that managing build identity across daemon versions is a serious, non-trivial concern inside CBM, probably to coordinate multiple clients/sessions against the same daemon).
- The watcher's real behavior on live file changes — not tested (it would require modifying the repository under test and measuring incremental re-indexing time).
- Whether `hook_augment` is exposed or documented as part of the stable public contract, or is an integration specific to certain clients (the comment itself mentions a "vendor hook payload", suggesting a per-integrator format).

## Risk

**Low, with a high-value design lesson.** No new risk was detected; on the contrary, this module is the best evidence found in the entire epic that the "non-blocking by default, bounded latency, observable timeout" pattern that `Rationale_v0.5.md §20.7` proposes adopting (not as an internal dependency, but as a principle) has already been validated in production by a real provider, including the concrete reason (issues #362, #858) that justified it.

## Decision impact

1. **It directly confirms, with first-hand evidence,** the principle already accepted in `Rationale_v0.5.md §20.7`: *"non-blocking by default, bounded latency, untrusted metadata as data, observable timeout/no-op, query-time correctness check"*. Rationale must replicate the pattern (hard deadline + clean exit + observable timeout breadcrumb), not copy the code.
2. The deliberate use of `search_graph` (pure SQLite) instead of `search_code` (shell-out) for latency reasons is concrete evidence that Rationale's **baseline fast path** should entirely avoid any operation that triggers a subprocess or shell, even indirectly through the provider.
3. The third version identifier (build hash ≠ `--version` ≠ Git SHA) reinforces, for the third time in this epic (see `00` and `04`), that **Rationale's adapter must not try to infer capability compatibility from any Codebase Memory version string** — it must use explicit capability negotiation (`Rationale_v0.5.md §21.2`), never version parsing.
4. Relevant for ADR-0009 (Baseline integration surfaces): if Rationale ever offers its own augmentation hook, `hook_augment.c` is the closest available design reference and should be reviewed in detail before implementing (Phase D/E, outside this bootstrap).

## Reproduce

```bash
cd ~/Desktop/codebase-memory-mcp
./build/c/codebase-memory-mcp daemon start
./build/c/codebase-memory-mcp daemon status
./build/c/codebase-memory-mcp daemon stop
head -60 src/cli/hook_augment.c
wc -l src/daemon/version_cohort.c tests/test_version_cohort.c
```
