# 04 — CLI contracts (CBM-007)

**Source of evidence:** the binary **built locally from HEAD** (`build/c/codebase-memory-mcp`, reporting version `dev`) — unlike documents `03` and `05`, which use the 0.8.1 release binary over MCP. Each document states explicitly which source it used (`00-source-lock.md`).

## Observed

- `--help` confirms the same list of 14 tools seen over MCP: `index_repository, search_graph, query_graph, trace_path, get_code_snippet, get_graph_schema, get_architecture, search_code, list_projects, delete_project, index_status, detect_changes, manage_adr, ingest_traces` — the tool contract is identical between the CLI and MCP transports (the same binary, two front ends).
- Top-level subcommands: `cli`, `install`, `uninstall`, `update`, `config`, `--version`, `--help`, plus `daemon start/stop` (discovered in use, not listed in the main `--help`).
- The binary declares "automatic/conditional" support for **43 client surfaces** (Claude Code, Codex CLI, Gemini CLI, Cursor, Windsurf, VS Code, etc.) through its installer — the pattern Rationale aims to replicate with `rationale install-agent` (`Rationale_Arquitectura_Conceptual_v0.1.md §24`).
- **`cli <tool> <json_args>` is deprecated in favor of flags, `--args-file`, or stdin** — the CLI actively warns: `warning: passing raw JSON to 'cli index_status' is deprecated and will be removed in a future release`.
- **Major finding — real coverage exposed through the CLI that did not appear through MCP (0.8.1) in `03-mcp-contracts.md`:** `cli --json index_status --project <p>` on this build (HEAD) returns:
  ```json
  {
    "project": "...", "nodes": 20747, "edges": 77956, "status": "ready",
    "root_path": "/Users/.../codebase-memory-mcp",
    "parse_partial": {"files": [], "count": 0, "truncated": false},
    "skipped": {"files": [], "count": 0, "truncated": false},
    "not_indexed": {"dirs": [], "dirs_count": 0, "files": [], "files_count": 0, "truncated": false}
  }
  ```
  This **is** structured partial-coverage information (`parse_partial`, `skipped`, `not_indexed`) — exactly what `05-revision-and-coverage.md` reported as absent in the equivalent MCP call on the 0.8.1 binary. See Decision impact: it may mean coverage improved between 0.8.1 and HEAD, or that this session's MCP wrapper does not request/expose these fields even though they exist.
- **Real measured latency, CLI without a running daemon (a temporary one per invocation):** two consecutive `index_status` calls took **6.811 s** and **6.873 s** of total (wall-clock) time, with ~2.2 s of user CPU each — the difference is the startup overhead of a temporary daemon per invocation (confirmed by the CLI's own message, see below).
- **Latency with a persistent daemon (a prior `daemon start`):** the same call dropped to **2.283 s** and **2.275 s** — a real improvement, but still far above any baseline budget.
- The CLI itself warns about it: `hint: this command started a temporary CBM daemon. 'codebase-memory-mcp daemon start' keeps one warm and removes this startup cost from every CLI command.`
- `daemon start` explicitly reports that it is **permanent**: `daemon: started (permanent, pid ...). It survives idle periods and session ends; 'codebase-memory-mcp daemon stop' retires it.`

## Claimed

The CLI actively documents itself with actionable warnings and hints on stderr (deprecations, suggesting the persistent daemon) — a higher level of runtime guidance than a typical CLI without such signals.

## Verified

- The tool list from `--help` matches exactly the one observed over MCP in `03-mcp-contracts.md`, confirming that both transports expose the same operation contract (though potentially different response fields depending on the version — see the coverage finding above).
- The latency improvement with a persistent daemon (6.8 s → 2.2 s) is reproducible and consistent across two runs each.

## Unknown

- **Whether the coverage fields (`parse_partial`/`skipped`/`not_indexed`) are new in HEAD compared with 0.8.1, or whether this session's MCP tool wrapper simply does not request/propagate them.** The cause could not be isolated without running the HEAD binary as an MCP server and comparing the exact same call — pending as the next research item before closing `12-integration-recommendation.md`.
- What the remaining ~2.2 s is due to even with a warm daemon: the startup of the CLI client binary itself (296 MB, loading ~180 grammars even though `index_status` does not use them), IPC to the daemon, or both? Not profiled in detail — outside the scope of this focused research.
- Whether `--tool-profile=analysis|scout` (mentioned in `--help` as "expose a restricted inspection surface") changes the contract of available tools — not tested.

## Risk

**High and directly actionable for Rationale's design.** Neither of the two measured CLI paths (cold: ~6.8 s; with a persistent daemon: ~2.2 s) comes close to Rationale's baseline budget (`Rationale_v0.5.md §20.5.2`: P50 ≤ 50 ms, P95 ≤ 150 ms, hard deadline ≤ 250 ms). If Rationale's baseline fast path invoked the Codebase Memory CLI on every read or search, it would violate its own budget by one to two orders of magnitude.

## Decision impact

1. **It empirically confirms a decision already made in the conceptual contract**: the "baseline fast path" (`Rationale_v0.5.md §20.5.1`) **cannot** invoke Codebase Memory (neither through the CLI nor, presumably, through a cold MCP session) on every read. It must depend exclusively on local storage already resolved by Rationale. This stops being a theoretical precaution and becomes a requirement backed by real measurement.
2. For the **intent-aware mode** (a tolerated budget of ~2 s, `Rationale_Arquitectura_Conceptual_v0.1.md §13.2`), a persistent MCP session (a long-lived process, not one `cli` subprocess per call) is the only viable path observed — it reinforces ADR-0002's hypothesis in favor of MCP over a CLI subprocess, pending measuring the same `index_status` through an already-live MCP session (not a new round trip) for a fair comparison.
3. The partial-coverage finding in HEAD (if confirmed real and not a session artifact) would be **good news** for Rationale: it would mean newer Codebase Memory versions do expose exactly the fields `05-revision-and-coverage.md` marked as absent. A follow-up research note is recommended before closing `12-integration-recommendation.md`.

## Reproduce

```bash
cd ~/Desktop/codebase-memory-mcp
./build/c/codebase-memory-mcp --help
./build/c/codebase-memory-mcp daemon start
time ./build/c/codebase-memory-mcp cli --json index_status --project Users-roor.osorio-Desktop-codebase-memory-mcp
time ./build/c/codebase-memory-mcp cli --json index_status --project Users-roor.osorio-Desktop-codebase-memory-mcp
./build/c/codebase-memory-mcp daemon stop
```
