# 01 — Build and test (CBM-002, CBM-003)

**Source of evidence:** a real build run on the clone at HEAD `97ce23f9` (`~/Desktop/codebase-memory-mcp`), on the reference machine (`docs/environment/reference-development-machine.md`).

## Observed

- `scripts/build.sh` (standard target, without `--with-ui`) **built successfully**: `Built: build/c/codebase-memory-mcp`.
- Wall time: **2m48.74s** (`361.32s` user CPU, `19.11s` system, `225%` average CPU — it used several of the 10 available cores).
- The build compiles ~180 tree-sitter grammars (one per supported language) plus the runtime, vendored `sqlite3`, `lz4`, `zstd`, `mimalloc`, `yyjson`, and its own modules (`foundation`, `store`, `cypher`, `mcp`, `daemon`, `discover`, `pipeline`, `simhash`, `semantic`, `traces`, `watcher`, `git`, `cli`, `ui`).
- **Resulting binary: 296,196,432 bytes (≈296 MB)**, Mach-O 64-bit arm64.
- `./build/c/codebase-memory-mcp --version` reports **`codebase-memory-mcp dev`** — not a semantic version number, unlike the installed binary, which reports `0.8.1`.
- The installed release binary (`~/.local/bin/codebase-memory-mcp`) weighs 269,322,576 bytes (≈257 MB) — comparable in size to the local build, but not identical.
- **`make -f Makefile.cbm test-foundation` fails at link time**, with dozens of unresolved `_suite_*` symbols (`_suite_security`, `_suite_semantic`, `_suite_watcher`, `_suite_yaml`, `_suite_zstd`, etc.), ending with `ld: symbol(s) not found for architecture arm64`. Time to failure: ~6 s.
- **The full suite (`make -f Makefile.cbm test` / the `test-runner` target) was not run** in this session: the project's own recent commit history (visible in `git log`) explicitly mentions test sharding, parallel execution of C suites, and CI legs on 3 operating systems (macOS/Linux/Windows) with VMs — signs that the full suite is substantially heavier than `test-foundation` and outside the scope of this research spike.

## Claimed

`README.md` documents the standard build as reproducible with simple prerequisites (a C/C++ compiler, zlib, Git) and mentions no expected build or test time.

## Verified

- The build reproduces exactly the commands documented in `README.md §Build from Source` (`scripts/build.sh`).
- The `test-foundation` failure is reproducible (a second run was not attempted, but the error is a deterministic link error, not a flaky test).

## Unknown

- Whether `test-foundation` is an actively maintained target or fell out of sync with the rest of the suite as `tests/` grew (strongly suggested by the project's own commit history of "test sharding" and "parallel test process").
- How long the full suite (`test` / `test-par`) really takes — not run because of the time cost, given that this epic's scope is analyzing integration contracts, not certifying Codebase Memory's internal quality.
- Why the local build reports version `dev` instead of a commit identifier — whether that is intentional (a non-release build) or whether the official release injects the version through a build flag not captured by the standard `scripts/build.sh`.
- Whether `--with-ui` (the variant with graph visualization) substantially changes size or behavior — not tested.

## Risk

**Medium.** The 296 MB binary is consistent with a product that embeds support for ~180 languages through tree-sitter — not evidence of a problem, but a relevant data point for any future binary-size comparison if Rationale ever decided to vendor its own grammars (not in the current plan). The `test-foundation` failure does not block Rationale's integration (which consumes the release binary over MCP and does not compile CBM), but it does limit this research's ability to independently certify internal behavior through CBM's own unit tests.

## Decision impact

- It confirms that **the full source compiles on the reference machine** (the bootstrap phase of `Rationale_Arquitectura_Conceptual_v0.1.md §6.6` is fulfilled).
- The local build reporting `dev` instead of a SHA or version reaffirms the finding in `00-source-lock.md`: **do not depend on the binary identifying itself usefully**; Rationale's adapter must not try to infer capability compatibility from an unreliable version string.
- Spending additional time fixing `test-foundation` is not recommended — maintaining an external provider's test suite is not Rationale's responsibility; it is documented as a known limitation and the work moves on.

## Reproduce

```bash
cd ~/Desktop/codebase-memory-mcp
git rev-parse HEAD   # confirm 97ce23f9827177fff3858831156e9795c6832b18
time scripts/build.sh
./build/c/codebase-memory-mcp --version
ls -la build/c/codebase-memory-mcp
make -f Makefile.cbm test-foundation   # reproduces the link failure
```
