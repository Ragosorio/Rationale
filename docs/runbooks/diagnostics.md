# Diagnostics

## Estado general

```bash
rationale health --project-root /ruta/al/proyecto
```

Reporta: `project_id`, `git_revision`, `working_tree_dirty`, `provider_status`, `provider_coverage` — y, si el proveedor no respondió, `provider_error` con el mensaje real (nunca oculto).

## Ver qué decidió Rationale sobre un target concreto

```bash
rationale prepare "src/auth/authorization.ts::resolveEntityRole"
```

`stderr` trae el diagnóstico paso a paso (Subject resuelto, target resuelto, cache HIT/MISS, applicability/linkage/authority calculados); `stdout` trae solo el `ContextPacket` JSON — nunca mezclados (`Arquitectura §11.1`).

## Ver propuestas pendientes de revisión

```bash
ls .rationale/proposals/*.yaml 2>/dev/null
rationale review --project-root /ruta/al/proyecto
```

## Logs de instrumentación local

Nunca se envían a ningún servicio (`Arquitectura §11.14`) — viven en `.rationale-local/` (ignorado por Git):

```bash
cat .rationale-local/runs/vertical-slice.ndjson       # cada rationale prepare: latencia, revisión, cobertura
cat .rationale-local/runs/review-decisions.ndjson     # cada decisión de rationale review: aprobado/rechazado/saltado y tiempo hasta confirmar
```

## Probar el servidor MCP directamente

Sin un agente de por medio, hablando el framing `Content-Length` a mano (mismo patrón que `docs/research/codebase-memory/11-performance-observations.md`):

```bash
python3 - <<'PY'
import json, subprocess
proc = subprocess.Popen(["target/release/rationale", "serve"], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
def send(o):
    b = json.dumps(o).encode()
    proc.stdin.write(f"Content-Length: {len(b)}\r\n\r\n".encode() + b); proc.stdin.flush()
def recv():
    h = b""
    while not h.endswith(b"\r\n\r\n"): h += proc.stdout.read(1)
    n = int([l for l in h.decode().split("\r\n") if l.lower().startswith("content-length")][0].split(":")[1])
    return json.loads(proc.stdout.read(n))
send({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"diag","version":"0"}}})
print(recv())
send({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"health","arguments":{}}})
print(recv())
proc.terminate()
PY
```

## Verificar que el schema de un Record/Subject no divergió

```bash
cargo test --test schema_validation
```

Compara los campos `required` de los 7 schemas JSON contra los campos no-`Option` de los structs Rust reales.

## Round-trip de un Record (verificar que escribir no pierde datos)

```bash
cargo test storage::tests::real_record_roundtrip_loses_no_data
```
