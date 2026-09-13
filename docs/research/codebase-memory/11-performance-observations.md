# 11 — Performance observations (CBM-011: measure CLI vs MCP)

## Observed

### CLI (formal measurement with `time`, see `04-cli-contracts.md`)

| Scenario | Wall clock | User CPU |
|---|---:|---:|
| `cli index_status`, no daemon (creates a temporary one per invocation) — run 1 | 6.811 s | 2.19 s |
| `cli index_status`, no daemon — run 2 | 6.873 s | 2.26 s |
| `cli index_status`, with a prior `daemon start` — run 1 | 2.283 s | 2.17 s |
| `cli index_status`, with a prior `daemon start` — run 2 | 2.275 s | 2.18 s |

The binary itself actively warns about this cost (`hint: this command started a temporary CBM daemon...`).

### MCP (formal measurement, B1.1 — our own stdio client against the binary built at HEAD)

A minimal Python client was written that speaks JSON-RPC 2.0 framed with `Content-Length` directly over stdio against `build/c/codebase-memory-mcp` (confirmed in `src/mcp/mcp.c`), without going through any already-connected agent session — it measures the process from scratch.

Protocol: spawn the process → `initialize` → `notifications/initialized` → three successive `tools/call` of `index_status` in the same session. Three independent runs:

| Stage | Run 1 | Run 2 | Run 3 |
|---|---:|---:|---:|
| Process spawn | 5.2 ms | 4.7 ms | 1.3 ms |
| **`initialize` (handshake, once)** | **6.837 s** | **6.791 s** | **6.859 s** |
| First `tools/call` (right after the handshake) | 27.7 ms | 27.5 ms | 22.8 ms |
| Second `tools/call` (same session) | 16.4 ms | 16.7 ms | 16.5 ms |
| Third `tools/call` (same session) | 16.2 ms | 16.5 ms | 14.9 ms |

**Central finding: the ~6.8 s cost lives entirely in the `initialize` handshake, once per process.** It matches, within the margin of error, the 6.8 s measured for the cold CLI in `04-cli-contracts.md` — confirming that both transports pay the same startup cost (probably loading the ~180 tree-sitter grammars and opening SQLite), not a different IPC cost. **Once the handshake completes, each tool call costs 15–30 ms** — within the baseline budget of `Rationale_v0.5.md §20.5.2` (P95 ≤ 150 ms).

## Claimed

No CBM documentation publishes CLI versus MCP latency benchmarks.

## Verified

- The four CLI measurements (`04-cli-contracts.md`) are reproducible, with <5% difference between runs.
- The MCP measurements are reproducible: three complete, independent runs (a new process each time) with `initialize` consistently between 6.79 s and 6.86 s, and subsequent calls consistently between 15 ms and 28 ms.

## Unknown

- Whether the cost of `initialize` is dominated by loading tree-sitter grammars, opening/checking the existing SQLite databases in `~/.cache/codebase-memory-mcp/`, or both — not profiled at that level of detail (outside the reasonable scope of this epic).
- Whether there is a latency difference between the stdio MCP transport and an eventual network variant — out of scope; CBM appears to operate only over local stdio.
- Whether CBM's persistent `daemon` (`06-daemon-and-watcher.md`) lets a new MCP client skip the 6.8 s `initialize` by connecting to an already-initialized process — not tested; this research's client always launched a new process. If the daemon allowed it, the 6.8 s cost would be paid once per machine, not per agent session.

## Risk

**Medium — refined relative to the initial assessment in `04-cli-contracts.md`.** The real risk is not that "all of MCP is slow": it is that **the first startup of a session pays ~6.8 s**, and no high-frequency surface of Rationale (reading, searching) can depend on an MCP process restarted per operation. If Rationale keeps a session (or connects to CBM's persistent daemon, pending confirmation), the measured per-operation cost (15–30 ms) is viable.

## Decision impact

1. **It confirms with formal, not only qualitative, evidence the recommendation of `04-cli-contracts.md`:** Rationale's baseline fast path must not launch a CLI process or a new MCP session per operation — the ~6.8 s `initialize` cost is indistinguishable from the cost measured for the cold CLI, so neither per-invocation transport is viable for the baseline.
2. **In favor of ADR-0002 (MCP over a CLI subprocess):** once `initialize` is paid, MCP's per-call cost (15–30 ms) is substantially better than re-invoking the CLI (which would repeat the full cost, `04-cli-contracts.md`). This is concrete evidence in favor of Rationale's adapter keeping **one long-lived persistent MCP session** (a single `initialize` per life of Rationale's process) instead of repeated CLI subprocesses.
3. The next research item, now narrower: confirm whether connecting to CBM's persistent `daemon` (`daemon start`) avoids the `initialize` cost for new MCP clients — it would determine whether Rationale can reconnect quickly after its own restart without paying 6.8 s again.

## Note (E7, adversarial review of Phase E) — the 6.8 s figure did not reproduce in the current development environment

`docs/work-items/adversarial-review-fase-e5-e6.md` (finding G) measured a fresh `initialize` against `codebase-memory-mcp` 0.8.1 (the same version, the same binary) at **~15–20 ms**, not 6.8 s, on the development machine where the Phase E5 MCP surface was implemented. The discrepancy is not yet explained — candidates not ruled out: an already-warm on-disk cache (`~/.cache/codebase-memory-mcp/*.db`, confirmed non-empty) that would avoid the cost of the original indexing measured under the name "`initialize`"; a measurement environment different from this note's; or a real behavior change between the date of this research and today.

This **does not invalidate Phase E5's amortization mechanism** (the persistent session is still strictly better than a single-shot CLI, and a real ~4–5x improvement, 150 ms→33 ms, was measured in that same environment) — but the dramatic magnitude that motivated ADR-0002 and this research note may be overstated for local development with a warm cache. Suggested next experiment: measure `initialize` with an empty `~/.cache/codebase-memory-mcp/` (a clean container) to isolate the cache variable and confirm whether the original 6.8 s corresponded to cold indexing rather than to the protocol handshake itself.

## Reproduce

```bash
cd ~/Desktop/codebase-memory-mcp
# Minimal client: spawn, initialize, 3x tools/call of index_status, time each stage.
# See docs/research/language/ (phase C) for the equivalent in the chosen language.
python3 - <<'PY'
import json, subprocess, time
BIN = "build/c/codebase-memory-mcp"
def send(p, o):
    b = json.dumps(o); p.stdin.write(f"Content-Length: {len(b)}\r\n\r\n{b}".encode()); p.stdin.flush()
def read(p):
    h = b""
    while b"\r\n\r\n" not in h: h += p.stdout.read(1)
    n = int([l for l in h.decode().split("\r\n") if l.lower().startswith("content-length:")][0].split(":")[1])
    return json.loads(p.stdout.read(n).decode())
proc = subprocess.Popen([BIN], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
t0=time.time(); send(proc, {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"probe","version":"0.0.1"}}}); read(proc)
print("initialize:", time.time()-t0)
proc.terminate()
PY
```
