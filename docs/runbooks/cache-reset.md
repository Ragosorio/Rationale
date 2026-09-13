# Cache reset

The derived layer (ADR-0004/0005) lives under `~/.cache/rationale/projects/` —
**never inside the repository**, and **never the only copy of a decision**
(`Arquitectura §11.7`). Deleting it is always safe: it rebuilds itself on the
next query.

## Find a project's cache

```bash
rationale health --project-root /path/to/project
```

Or compute it by hand. The directory name sanitizes the project's absolute path
by replacing `/` with `-`:

```bash
echo "$HOME/.cache/rationale/projects/$(realpath /path/to/project | sed 's#^/##; s#/#-#g')"
```

## Delete one project's cache

```bash
rm -rf "$HOME/.cache/rationale/projects/<sanitized-path>"
```

## Delete all of Rationale's cache (every project)

```bash
rm -rf "$HOME/.cache/rationale"
```

## What is lost and what is not

| Lost (recomputed automatically) | Never lost (lives in `.rationale/`, versioned in Git) |
|---|---|
| Cached assessments | Records, Subjects, and configuration |
| FTS5 index of statements and titles | Archived and pre-1.0 proposals |

Verified by a test (`cache::tests::cache_rebuild_from_scratch_never_loses_canonical_data`):
deleting the cache and rebuilding it produces identical results from the same
real Records.

## When to do it

- The cache is corrupted (very rare; SQLite in WAL mode is robust against abrupt
  shutdowns).
- You suspect an assessment kept stale data from an earlier schema version.
- You are debugging and want to confirm a result does not depend on cached
  state.
