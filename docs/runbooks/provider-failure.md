# Provider failure

Rationale never blocks when the structural provider (`codebase-memory-mcp`) is
unavailable: it fails open (`Arquitectura §13.5`). This runbook explains what
degradation looks like and how to diagnose it.

## What a provider failure looks like

```bash
rationale health
```

```json
{"provider_status":"unavailable","provider_coverage":"unknown","provider_error":"No such file or directory (os error 2)"}
```

`provider_status` is `successful`, `degraded`, or `unavailable`, and
`provider_coverage` is `complete`, `partial`, or `unknown`.

No Rationale command fails because of this. `prepare_change` still returns the
full packet (constraints, conflicts, risks); structural fields lose coverage,
`resolved_target` may be `null`, and a warning appears in `warnings`. Warning
text is currently emitted in Spanish, for example
`"no se pudo iniciar Codebase Memory: ..."` ("could not start Codebase Memory").

## Diagnosis

1. **Is the binary on `PATH`?**

   ```bash
   which codebase-memory-mcp
   ```

2. **Does it respond directly?** It uses the same framing Rationale does; see
   `docs/research/codebase-memory/11-performance-observations.md` for the
   reference script.

   ```bash
   codebase-memory-mcp --version
   ```

3. **Is Rationale's MCP server using a stale session?** When `rationale serve`
   has been running for a long time, its provider session was established once
   at startup (ADR-0002/ADR-0007). A provider problem that appeared after the
   server started does not fix itself: restart `rationale serve`, usually by
   restarting the agent.

4. **Did Codebase Memory lose the project it had indexed?** Rationale detects a
   stale project mapping, forgets it, and re-resolves the project through the
   provider's public tools on the next call. Run `rationale health` again.

## What Rationale never does in this situation

- It never invents a `resolved_target`.
- It never treats "the provider did not respond" as "the symbol does not exist"
  (`Rationale_v0.5.md §19.2`: absence of evidence is not evidence of absence).
- It never stops responding: a provider timeout kills the child process and
  reports `unavailable` within seconds
  (`providers::codebase_memory::tests::provider_timeout_reports_unavailable_and_kills_process`
  verifies it).
