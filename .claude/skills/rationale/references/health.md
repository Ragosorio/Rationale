# Health

Find out what works, what is degraded, and what was not checked, without
turning a missing provider into a missing canon.

## Steps

1. Call the `health` tool. When the tool is unavailable, run `rationale health`
   in a terminal, or go to "The tools are missing" below.
2. Run `rationale doctor` for canon integrity. It is read-only.
3. Report in three groups with the health template in
   `assets/report-templates.md`: what works, what is degraded, and what was not
   checked.

## Reading `health`

| Field | Healthy value | Otherwise |
|---|---|---|
| `project_root`, `project_id` | The repository you are working in | Pass `project_root`, or run from the right directory |
| `git_revision` | A commit hash | Not a Git repository, or Git is missing |
| `working_tree_dirty` | Either value | `true` means new bindings are `provisional` until the code is committed |
| `provider_status` | `successful` | `degraded` or `unavailable`: structure and symbol bindings are not fully verified |
| `provider_coverage` | `complete` | `partial` or `unknown`: say which parts may be missing |
| `provider_error` | `null` | Quote it |

A degraded provider never means the canon is empty or that nothing governs a
target. Records with file bindings are still served.

## Common problems

| Symptom | Likely cause | What to do |
|---|---|---|
| The `rationale` tools are missing | The MCP server is not registered, or the agent was not restarted | Ask the user to run `rationale install-agent` and restart the agent. Registration is per user and uses the binary's absolute path |
| `rationale: command not found` | Rationale is not installed, or not on `PATH` | Point the user to the installer in the project README |
| `provider_status: unavailable` | Codebase Memory is not installed or not on `PATH` | Continue with degraded coverage and say so; suggest installing it |
| The provider lost the project it indexed | A stale project mapping | Rationale re-resolves it on the next call; call `health` again |
| A Record does not appear in `prepare_change` | Its bindings point at moved or deleted code, or it was superseded or revoked | Run `rationale doctor --check`, then follow `references/maintain.md` |
| `finalize_change` discarded every candidate | The candidates failed the gate | Read each `reason` in `references/records.md` |
| `rationale ui` serves an instructions page | A source build without the embedded Control Room | Release binaries include it; from source, build `ui/` before the binary |
| `rationale serve` prints nothing when started by hand | Expected: it waits for JSON-RPC on stdin | Nothing to fix |

## The tools are missing

Without the MCP tools, you cannot prepare or capture through Rationale. Say so
plainly, then:

- offer `rationale prepare <target> --intent "<intent>"` in a terminal to read
  the packet;
- keep the change minimal and list the knowledge you would have captured, so
  the user can capture it once the server works;
- never claim a Record was written.
