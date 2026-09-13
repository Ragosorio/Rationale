# beta.3 — agent migration and Codebase Memory identity

**Status:** end-to-end validation complete, including a real Cursor reload
(2026-07-29).

## Findings

1. Cursor loaded the configuration but did not resolve the logical command
   `rationale` from the graphical application.
2. Codex checked only the registered name, not the command and arguments.
3. Rationale reconstructed the name derived by Codebase Memory. The provider
   collapses consecutive separators; Rationale did not.
4. `resolve_target` re-indexed on every query and chose the first symbol by name
   without narrowing by file.
5. The MCP client did not correlate JSON-RPC responses by `id`.

## Proposed decisions

- ADR-0016 moves the server to global configuration with an absolute path and
  leaves instructions/skills in the project.
- The index identity comes from `list_projects.root_path` or from the public
  response of `index_repository`; it is never reconstructed and private storage
  is never read.
- The public handle is stored in
  `.rationale-local/codebase-memory-project.json`, tied to the `root_path`.
  An existing index is queried; during the migration, indexing happens only if a
  new provider process does not report the path and no such link exists yet.
- Symbol resolution also passes `file_pattern`.

## Re-indexing, IDs, and clones

- Rationale does not persist Codebase Memory node IDs. Its canonical bindings are
  its own paths/symbols and survive a new generation of the graph.
- Re-indexing can replace the derived database and change internal identifiers.
  It must not break the canon, but it creates a transient state; that is why the
  automatic per-query re-indexing was removed.
- Two clones of the same remote at different paths are two distinct derived
  projects. Rationale keeps one `.rationale/` canon per checkout.

## Unknown

The installed version prints `0.8.1` in the CLI while its MCP handshake announces
`0.10.0`. That discrepancy belongs to the provider and is not resolved by reading
its internal storage.

## Risk

An extremely deep path made the provider fail when dumping an index with a long
derived name. Rationale must degrade honestly; it cannot fix an internal limit by
inventing another naming algorithm.

## Next experiment

Publish the authorized tag and verify the cross-platform CI artifacts before
promoting beta.3 as a release.

## Local validation evidence

- Formatter and strict Clippy: pass.
- Full suite: 273 tests pass (unit, CLI, MCP, concurrency, schemas, and the
  dogfood chain).
- Clean room beta.2 → beta.3: Claude Code, Codex, and Cursor converge; three
  foreign servers remain; the second run keeps byte-identical hashes; the global
  reversal removes only Rationale.
- macOS arm64 package: checksum verified and expected content.
- Real installation: `/Users/roor.osorio/.local/bin/rationale` reports
  `v0.1.0-beta.3`; `~/.claude.json`, `~/.cursor/mcp.json`, and
  `codex mcp get rationale` point at that path.
- Real stdio MCP: `initialize` reports beta.3 and `tools/call health` returns
  `provider_status=successful`, `provider_coverage=complete`.
- `prepare src/agents.rs::install` resolves
  `Users-roor.osorio-Desktop-Rationale.src.agents.install`, not a documentation
  match.
- The deep clone used by Cursor reports complete health after storing its local
  public handle; its legacy `.mcp.json` entries were removed and the global
  registration remains.
- Cursor, reloaded, shows `user-rationale` connected (`ready`), exposes
  `prepare_change`, `explain_target`, `finalize_change`, and `health`, and the
  real `health` call returns `provider_status=successful` and
  `provider_coverage=complete`.
