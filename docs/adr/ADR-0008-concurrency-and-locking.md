# ADR-0008: Concurrency and locking

**Status:** proposed — pending independent cross-review before `accepted`.
**Date:** 2026-07-26; extended 2026-07-28
**Deciders:** Claude Code (analysis and implementation); pending human approval and/or cross-review by another agent
**Supersedes / Superseded by:** none

## Context

Phase F1 turns `src/storage.rs` from read-only into read/write (`write_record`), the prerequisite for `finalize_change` (Phase F5) and `rationale review` (Phase F6) to persist proposals and approvals in `.rationale/`. `Arquitectura §11.6` requires "writing atomic changes" and `§15.3` requires "using temporary files and atomic rename" to protect writes — but neither specifies what happens under real **concurrent** writing (two CLI invocations, or an MCP server and a human review, writing the same Record at the same time).

`docs/dependencies/inventory.yaml` had already recorded a `known_gap` since the language spike: the Rust candidate's file-locking path was POSIX-only (`flock` through FFI), unexercised on Windows. Until now this did not matter because no real write existed to lock.

## Decision

1. **Atomic `rename` is the only correctness guarantee** for concurrent writing in Phase F — no file lock (`flock`, portable `File::lock`, or an external lock) is added yet. Two concurrent writes to the same Record produce "last rename wins": the final result is exactly one of the two complete Records, never a merge or a corrupt or truncated file.
2. **The Windows `known_gap` is formally deferred**, not resolved: not blocking until there is evidence of a real case of concurrent writing between processes (not only threads) on that OS.
3. The temporary file name of `write_record` was made unique **per invocation**, not only per process (PID + atomic counter), closing a real bug found during this same implementation (see Evidence).
4. Complete files administered by `install-agent` (Claude Code skills, and later
   the files of the `rationale` skill for Claude Code and Codex) use a **claim
   by identity**, not a general lock: Rationale atomically renames the existing
   entry to a unique sibling name, verifies the hash of that claimed identity,
   and only then removes it or publishes the replacement.
5. Publishing a skill uses **no-clobber** semantics through a complete, synced
   file followed by a `hard_link` to the destination. If another process
   recreates the path during the operation, publication fails closed, keeps the
   concurrent content, and reports where the claimed copy was left.
6. No project or skill lock is added. An advisory lock would only coordinate
   cooperative Rationale processes; it would not prevent an editor or another
   agent from replacing the path between the check and the deletion.
7. The manifest has no authority to choose freely how a path is reverted.
   `uninstall-agent` derives the exact tuple `path + agent + reversal` from
   code: instruction and MCP files always use `managed-part`; only the
   enumerated skill files use `owned-file`, and those require a non-empty hash.
8. Claim names include the PID, a nanosecond timestamp, and an atomic counter.
   Before the `rename`, an existing claim destination is treated as a collision
   and another name is sought; an abandoned quarantine is never reused or
   overwritten.

## Evidence

- **A real bug found and fixed during F1**: the first version of `write_record` named the temporary file with only the process PID (`.{file}.tmp-{pid}`). Two threads of the *same* process writing the same Record at once would have reused the same temporary name, letting one thread truncate the other's temporary file mid-write — real corruption, not hypothetical. Fixed by adding a per-process atomic counter (`AtomicU64`) to the temporary name.
- **A real concurrency test** (`storage::tests::concurrent_writes_to_same_record_never_corrupt_the_file`): 8 threads write the same Record simultaneously with different contents. Verified over 15 consecutive runs without failures: the final file is always a complete, valid Record (never corrupt), its content is exactly one of the 8 candidates (never a mixture), and no orphaned temporary file remains after contention.
- **An atomic write test under replacement** (`write_record_leaves_no_tmp_file_and_fully_replaces_existing`): a second write replaces the first completely, without merging old and new fields.
- **An existing precedent in the derived layer** (`cache::tests::concurrent_reads_do_not_corrupt_cache`, Phase E3): concurrent reads over SQLite in WAL mode are already covered; this ADR covers the missing piece, concurrent writing over the YAML canon.
- **A TOCTOU race reproduced as a unit**:
  `claimed_removal_never_deletes_a_recreated_destination` claims the file,
  recreates the path with user content, and verifies that the deletion removes
  only the claimed identity.
- **Publication without overwriting**:
  `no_clobber_publish_preserves_a_destination_that_reappeared` verifies that a
  concurrent destination survives byte for byte.
- **An edit kept and restored**:
  `edited_claim_is_restored_without_overwrite` verifies that an unexpected hash
  restores the file and leaves no quarantine in the normal case.
- **An abandoned claim kept**:
  `claim_skips_an_abandoned_destination_instead_of_overwriting_it` shows that an
  occupied name is not replaced and that both the current file and the earlier
  quarantine remain intact byte for byte.
- **A manifest without destructive authority**:
  `uninstall_rejects_owned_file_reversal_for_managed_part_files` covers
  `CLAUDE.md` and `.mcp.json`; even when the manifest includes the correct hash,
  it cannot turn them into complete files owned by Rationale.

## Alternatives considered

- **Real file locking (`flock` through FFI, or the portable `std::fs::File::lock` since Rust 1.89)**: discarded *for now*. Adding a lock requires deciding its scope (per Record? per whole project?), its behavior when dead processes do not release it, and exercising it on Windows — real work that today has no concrete use case to justify it (`AGENTS.md`: "do not create a daemon before measuring the need" applies to locking by the same principle). Revisited if evidence of lost writes appears in real use.
- **A simple advisory file lock (`.rationale/.lock`) per project**: discarded for now — it would serialize every write in the project (not only the Record in conflict), a disproportionate cost without evidence that collisions are frequent. A reasonable candidate if the revisit trigger fires.
- **A per-skill lock**: discarded as a way to close this risk. It would prevent
  two simultaneous Rationale installations if both cooperate, but it does not
  protect against editors, scripts, or other agents that do not take the lock.
  It could be added later to improve messages and avoid duplicate work, but it
  does not replace the atomic capture of identity.
- **Checking the hash twice before `remove_file(path)`**: discarded. It only
  narrows the window; there is still an instant between the second check and
  the deletion by name.
- **A transactional database for the canon** (instead of YAML files): out of scope — it contradicts `Arquitectura §26.1` (the canon must be readable and reviewable in a PR without the tool, ADR-0003).

## Consequences

- `write_record` never blocks waiting for a lock — consistent with "fail open" and with the absence of a persistent daemon in this phase.
- Under a real collision (two writes to the same Record in the same time window), one of the two is lost silently from the perspective of whoever issued it — there is no "your write was overwritten" notification. This is acceptable for the current usage pattern (one agent + one human reviewing sequentially, `rationale review` in Phase F6), not for truly concurrent multi-agent writing.
- The Windows `known_gap` remains open and now lives in this ADR instead of only in `inventory.yaml`.
- Uninstalling a complete file no longer deletes the name it observed before:
  it deletes the unique claimed name. A file that reappears at the destination
  belongs to the concurrent process and stays intact.
- File systems that do not support hard links or safe publication make the
  operation fail without overwriting. This prioritizes keeping data over
  silently completing the installation.

## Risks

- **Silent loss of a write under a real collision** — partially mitigated because the intended flow (`finalize_change` writes into `.rationale/proposals/`, never directly to `records/`; `rationale review` is the only path that writes to `records/`) drastically reduces the real collision window: normally a single human process runs `rationale review` at a time.
- **The Windows gap could show up sooner than expected** if Rationale is used there with two real concurrent processes. Mitigation: the revisit trigger below is concrete and verifiable.
- **A process with an already-open descriptor can keep writing to the claimed
  identity after the rename.** Rationale will never confuse that identity with a
  new path, but perfect coordination with non-cooperative writers would require
  OS-specific exclusive primitives. The current behavior avoids overwriting the
  concurrent entry and keeps the claimed copy when it cannot restore it; that
  limit must remain visible.
- **There is a minimal window between checking that the unique claim name is
  free and running `rename`.** The identity includes a nanosecond timestamp and
  an atomic counter, and detected abandoned names are skipped, so an accidental
  collision no longer depends on reusing a PID. Closing a hostile creation in
  exactly that window as well would require an OS-specific `rename-no-replace`
  primitive or cooperative locking; the current check is not presented as a
  guarantee against a local attacker.

## Validation

The tests described in Evidence run as part of `cargo test` in every phase
verification. The canon concurrency test uses threads (not separate processes)
because it exercises the same code path (`std::fs::rename` on the same
filesystem) with far less test overhead. The skill tests explicitly separate the
claim, the recreation of the destination, and the finalization to make the
TOCTOU window deterministic, where sleeps would make it probabilistic.

## Revisit trigger

Reopen when: (a) a real case appears of two processes (not threads) writing the
same Record within a measurable collision window — for example, two agents
working on the same project simultaneously in Phase G/H; (b) the monorepo pilot
(Phase H) runs on Windows and `rename` and `hard_link` need empirical
verification on NTFS; or (c) evidence appears of writers that keep a descriptor
open on a skill while `install-agent` replaces it. Case (c) would justify
studying per-platform exclusive handles, not an advisory lock presented as a
universal guarantee.
