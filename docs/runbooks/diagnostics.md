# Diagnostics

## Overall state

```bash
rationale health --project-root /path/to/project
```

It reports `project_id`, `git_revision`, `working_tree_dirty`,
`provider_status`, and `provider_coverage` — and, when the provider did not
respond, `provider_error` with the real message, never hidden.

## See what Rationale decided about a target

```bash
rationale prepare "src/auth/authorization.ts::resolveEntityRole"
```

`stderr` carries the step-by-step diagnostics (resolved Subject, resolved
target, cache HIT/MISS, computed applicability, linkage, and authority);
`stdout` carries only the `ContextPacket` JSON. They are never mixed
(`Arquitectura §11.1`).

## Conflicts with pinned rules

```bash
rationale conflicts --project-root /path/to/project
rationale resolve <conflict-id> keep-pinned
```

A conflict appears when an agent tried to replace a `pinned` Record; its
assertion was not written and waits for a human decision.

## Canon integrity and pre-1.0 proposals

```bash
rationale doctor --check
rationale migrate --dry-run
```

## Agents and skills

```bash
rationale install-agent --dry-run
```

It prints what the installer would change without writing anything: protocol
blocks, skill files kept because you edited them, skills with unknown
provenance, and skill directories left alone because they are symbolic links.
In Claude Code, `/rationale-health` combines the MCP `health` tool with
`rationale doctor`.

## Watch work live

```bash
rationale ui
```

The Control Room shows operations, activity, and memory without writing
anything. See [`docs/user-guide/control-room.md`](../user-guide/control-room.md).

## Local activity

It is never sent to any service (`Arquitectura §11.14`) and lives in
`.rationale-local/`, excluded from Git (ADR-0014). What it contains and never
contains: ADR-0017.

```bash
ls -t .rationale-local/activity/                                    # one session per process: rationale serve or one CLI invocation
tail -n 20 "$(ls -t .rationale-local/activity/*.ndjson | head -1)"  # events of the most recent session
ls -t .rationale-local/operations/ | head                           # operation snapshots: subgraph and selection of each prepare_change
RATIONALE_ACTIVITY=off rationale serve                              # disables the activity stream
```

The Phase D `RunLog` (`runs/vertical-slice.ndjson`) was retired in vNext; the
activity stream replaces it.

## Talk to the MCP server directly

Without an agent in between. Rationale's server speaks line-delimited JSON-RPC
over stdio (ADR-0007); `Content-Length` framing is used only by the client
Rationale opens toward Codebase Memory.

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

## Check that a Record or Subject schema has not diverged

```bash
cargo test --test schema_validation
```

It compares the `required` fields of the seven JSON schemas with the
non-`Option` fields of the real Rust structs.

## Round-trip a Record (check that writing loses no data)

```bash
cargo test storage::tests::real_record_roundtrip_loses_no_data
```
