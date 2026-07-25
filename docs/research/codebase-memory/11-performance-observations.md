# 11 — Performance observations (CBM-011: measure CLI vs MCP)

## Observed

### CLI (medición formal con `time`, ver `04-cli-contracts.md`)

| Escenario | Wall clock | CPU usuario |
|---|---:|---:|
| `cli index_status`, sin daemon (crea uno temporal por invocación) — corrida 1 | 6.811s | 2.19s |
| `cli index_status`, sin daemon — corrida 2 | 6.873s | 2.26s |
| `cli index_status`, con `daemon start` previo — corrida 1 | 2.283s | 2.17s |
| `cli index_status`, con `daemon start` previo — corrida 2 | 2.275s | 2.18s |

El propio binario advierte activamente sobre este costo (`hint: this command started a temporary CBM daemon...`).

### MCP (medición formal, B1.1 — cliente stdio propio contra el binario compilado en HEAD)

Se escribió un cliente mínimo en Python que habla JSON-RPC 2.0 framed con `Content-Length` directamente por stdio contra `build/c/codebase-memory-mcp` (confirmado en `src/mcp/mcp.c`), sin pasar por ninguna sesión de agente ya conectada — mide el proceso desde cero.

Protocolo: spawn del proceso → `initialize` → `notifications/initialized` → tres `tools/call` sucesivos de `index_status` en la misma sesión. Tres corridas independientes:

| Etapa | Corrida 1 | Corrida 2 | Corrida 3 |
|---|---:|---:|---:|
| Spawn del proceso | 5.2ms | 4.7ms | 1.3ms |
| **`initialize` (handshake, una vez)** | **6.837s** | **6.791s** | **6.859s** |
| Primer `tools/call` (justo después del handshake) | 27.7ms | 27.5ms | 22.8ms |
| Segundo `tools/call` (misma sesión) | 16.4ms | 16.7ms | 16.5ms |
| Tercer `tools/call` (misma sesión) | 16.2ms | 16.5ms | 14.9ms |

**Hallazgo central: el costo de ~6.8s vive enteramente en el handshake `initialize`, una única vez por proceso.** Coincide, dentro del margen de error, con el 6.8s medido para la CLI fría en `04-cli-contracts.md` — confirma que ambos transportes pagan el mismo costo de arranque (probablemente carga de las ~180 gramáticas tree-sitter y apertura de SQLite), no un costo distinto de IPC. **Una vez completado el handshake, cada llamada de herramienta cuesta 15-30ms** — dentro del presupuesto de baseline de `Rationale_v0.5.md §20.5.2` (P95 ≤ 150ms).

## Claimed

Ninguna documentación de CBM publica benchmarks de latencia CLI vs MCP.

## Verified

- Las cuatro mediciones de CLI (`04-cli-contracts.md`) son reproducibles, con diferencia <5% entre corridas.
- Las mediciones MCP son reproducibles: tres corridas completas e independientes (proceso nuevo cada vez) con `initialize` consistentemente entre 6.79s y 6.86s, y llamadas subsecuentes consistentemente entre 15ms y 28ms.

## Unknown

- Si el costo de `initialize` es dominado por la carga de gramáticas tree-sitter, apertura/verificación de las bases SQLite existentes en `~/.cache/codebase-memory-mcp/`, o ambos — no perfilado a ese nivel de detalle (fuera del alcance razonable de esta epic).
- Si existe una diferencia de latencia entre el transporte MCP stdio y una eventual variante de red — fuera de alcance, CBM parece operar únicamente sobre stdio local.
- Si el `daemon` persistente de CBM (`06-daemon-and-watcher.md`) permite que un cliente MCP nuevo se salte el `initialize` de 6.8s conectándose a un proceso ya inicializado — no probado; el cliente de este research siempre lanzó un proceso nuevo. Si el daemon lo permitiera, el costo de 6.8s se pagaría una sola vez por máquina, no por sesión de agente.

## Risk

**Medio — refinado respecto a la evaluación inicial en `04-cli-contracts.md`.** El riesgo real no es que "todo MCP sea lento": es que **el primer arranque de una sesión paga ~6.8s**, y ninguna superficie de alta frecuencia de Rationale (lectura, búsqueda) puede depender de un proceso MCP que se reinicia por operación. Si Rationale mantiene una sesión (o se conecta al daemon persistente de CBM, pendiente de confirmar), el costo por-operación medido (15-30ms) sí es viable.

## Decision impact

1. **Confirma con evidencia formal, no solo cualitativa, la recomendación de `04-cli-contracts.md`:** el fast path baseline de Rationale no debe lanzar un proceso CLI ni una sesión MCP nueva por operación — el costo de ~6.8s de `initialize` es indistinguible del costo medido en CLI fría, así que ninguno de los dos transportes por-invocación es viable para el baseline.
2. **A favor de ADR-0002 (MCP sobre CLI subprocess):** una vez pagado el `initialize`, el costo por-llamada de MCP (15-30ms) es sustancialmente mejor que reinvocar la CLI (que repetiría el costo completo, `04-cli-contracts.md`). Esto es evidencia concreta a favor de que el adaptador de Rationale mantenga **una sesión MCP persistente de larga duración** (un solo `initialize` por vida del proceso de Rationale), en vez de subprocesos CLI repetidos.
3. Próximo research item, ahora más acotado: confirmar si conectar contra el `daemon` persistente de CBM (`daemon start`) permite evitar el costo de `initialize` en clientes MCP nuevos — determinaría si Rationale puede reconectar rápido tras un reinicio propio sin pagar 6.8s de nuevo.

## Reproducir

```bash
cd ~/Desktop/codebase-memory-mcp
# Cliente mínimo: spawn, initialize, 3x tools/call de index_status, medir cada etapa.
# Ver docs/research/language/ (fase C) para el equivalente en el lenguaje elegido.
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

