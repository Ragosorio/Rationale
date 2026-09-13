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

## Conflictos con reglas fijadas

```bash
rationale conflicts --project-root /ruta/al/proyecto
rationale resolve <conflict-id> keep-pinned
```

Un conflicto aparece cuando un agente intentó reemplazar un Record `pinned`; su
afirmación no se escribió y espera la decisión humana.

## Integridad del canon y propuestas anteriores a 1.0

```bash
rationale doctor --check
rationale migrate --dry-run
```

## Ver el trabajo en vivo

```bash
rationale ui
```

El Control Room muestra operaciones, actividad y memoria sin escribir nada. Ver
[`docs/user-guide/control-room.md`](../user-guide/control-room.md).

## Actividad local

Nunca se envía a ningún servicio (`Arquitectura §11.14`) y vive en `.rationale-local/`, excluido de Git (ADR-0014). Qué contiene y qué nunca contiene: ADR-0017.

```bash
ls -t .rationale-local/activity/                                    # una sesión por proceso: rationale serve o una invocación de la CLI
tail -n 20 "$(ls -t .rationale-local/activity/*.ndjson | head -1)"  # eventos de la sesión más reciente
ls -t .rationale-local/operations/ | head                           # snapshots de operación: subgrafo y selección de cada prepare_change
RATIONALE_ACTIVITY=off rationale serve                              # desactiva el flujo de actividad
```

El `RunLog` de Fase D (`runs/vertical-slice.ndjson`) se retiró en vNext: la actividad lo reemplaza.

## Probar el servidor MCP directamente

Sin un agente de por medio. El servidor de Rationale habla JSON-RPC delimitado
por líneas sobre stdio (ADR-0007); `Content-Length` solo lo usa el cliente que
Rationale abre hacia Codebase Memory.

```bash
python3 - <<'PY'
import json, subprocess
proc = subprocess.Popen(["rationale", "serve", "--client", "diag"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
def call(message):
    proc.stdin.write(json.dumps(message) + "\n"); proc.stdin.flush()
    return json.loads(proc.stdout.readline())
print(call({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"diag","version":"0"}}}))
print(call({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"health","arguments":{}}}))
proc.stdin.close(); proc.wait()
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
