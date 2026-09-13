# 07 — Storage and cache

**Source of evidence:** direct inspection of the filesystem on the reference machine. The internal content of no SQLite database was read (`Rationale_Arquitectura_Conceptual_v0.1.md §7.2` forbids depending on that as a contract) — only file metadata (paths, sizes, permissions).

## Observed

- Default derived storage: **`~/.cache/codebase-memory-mcp/`**, with one SQLite file per indexed project: `<Sanitized-Project-Id>.db` (+ `.db-wal` and `.db-shm` with WAL mode active).
- The `Sanitized-Project-Id` is the project's absolute path with separators replaced (for example `/Users/roor.osorio/Desktop/Monorepo` → `Users-roor.osorio-Desktop-Monorepo`) — it matches exactly the `name` returned by `list_projects`.
- Observed sizes: 3 MB–86 MB per project, roughly proportional to the size/complexity of the indexed code (CBM itself, the largest, weighs 83 MB; Monorepo 27 MB).
- Cache directory permissions: `drwx------` (700, owner only) — correct default behavior.
- Global config in `~/.cache/codebase-memory-mcp/config.json`: only `{"ui_enabled": true, "ui_port": 9749}` — no visible secrets or tokens.
- An additional `_config.db` next to the per-project `.db` files — probably installation-level configuration/metadata, not inspected internally.
- `logs/cbm-daemon.log` exists inside the same cache directory (permissions `-rw-------`, 600) — consistent with the `hook_augment.c` timeout log documented in `06-daemon-and-watcher.md` (`~/.cache/codebase-memory-mcp/logs/hook-augment-timeouts.log`).
- **No `.codebase-memory/` directory was found inside the indexed repositories** (neither in `~/Desktop/Monorepo` nor in `~/Desktop/codebase-memory-mcp`) — that is, by default CBM **writes nothing inside the user's own repository**; all of its persistence lives outside, in the user cache. This would change only if `index_repository(persistence=true)` is invoked (seen in `03-mcp-contracts.md`), which would write `.codebase-memory/graph.db.zst` inside the repository for team sharing — that option was not enabled in any indexing in this epic.

## Claimed

No public claim inspected in this epic about the exact internal format of the `.db` databases — consistent with `Rationale_Arquitectura_Conceptual_v0.1.md §7.2`, which already assumes it must not be treated as a stable contract.

## Verified

- The file naming convention (`<sanitized-path>.db`) is consistent between the 7 projects observed through `list_projects` and the real files in `~/.cache/codebase-memory-mcp/`.
- The restrictive permissions (700/600) are real, not only documented.

## Unknown

- The exact internal format of the SQLite tables — deliberately not inspected (outside the acceptable integration boundaries, `Rationale_Arquitectura_Conceptual_v0.1.md §7.2`: "Rationale shall not read Codebase Memory's internal tables directly").
- The exact content and format of `_config.db`.
- The exact behavior of `persistence: true` in `index_repository` — not run (to avoid modifying evidence repositories with an unrequested shared artifact).
- The invalidation/expiry policy of these `.db` files — whether there is a size limit, LRU, or whether they grow indefinitely with every re-indexing.

## Risk

**Low.** The observed behavior (a cache outside the repository, restrictive permissions, no visible secrets in the config) is consistent with good practice and contradicts none of Rationale's principles. The only genuine point of attention is the `persistence: true` option, which, if a developer enabled it unknowingly, would start versioning a binary index artifact inside the repository — something Rationale must actively avoid for its own derived layer.

## Decision impact

- It confirms that the "derived layer outside the repository, regenerable, with restrictive permissions" pattern (`Rationale_v0.5.md §26.2`, Subject `storage.canonical-vs-derived`) is already validated by the provider itself — Rationale can adopt an analogous cache naming convention (`~/.cache/rationale/projects/<project-id>/`, already anticipated in `Rationale_Arquitectura_Conceptual_v0.1.md §10.2`) with confidence that it is a proven pattern in the same class of tool.
- It reinforces that Rationale must **never** offer, not even as an option, to write its derived index inside the versioned repository (unlike CBM's `persistence: true` option) — keeping the strict canonical/derived separation `Rationale_v0.5.md §4.19` requires.
- ADR-0005 (Cache root and project identity): CBM's path-to-file-name sanitization convention is a reasonable precedent to evaluate, with the caution that very long paths or paths with special characters could collide or be truncated — not tested here.

## Reproduce

```bash
ls -la ~/.cache/codebase-memory-mcp/
cat ~/.cache/codebase-memory-mcp/config.json
du -sh ~/.cache/codebase-memory-mcp/*.db
find ~/Desktop/Monorepo ~/Desktop/codebase-memory-mcp -maxdepth 1 -iname ".codebase-memory*"
```
