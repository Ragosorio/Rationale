# Build and test

Requires the Rust toolchain (`rustc`/`cargo`). In non-interactive shells, make
sure it is on `PATH`:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

## Full verification (what runs before every commit)

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
bash scripts/check-docs.sh
```

All of them must pass cleanly. `cargo test` runs the unit tests in `src/` and
the integration binaries in `tests/`: `cli.rs`, `mcp_server.rs` (against the
real compiled binary, including the attacks from the adversarial reviews),
`dogfood_governance_chain.rs`, `empty_project.rs`, `review_concurrency.rs`, and
`schema_validation.rs`.

The unit tests also guard the `rationale` skill: the bundle in
`src/skill_bundle.rs` must match `skills/rationale/`, `SKILL.md` must respect
the Agent Skills limits, every reference must be linked, and the candidate
validator must mirror the capture gate's rules.

## Development versus release builds

```bash
cargo build              # debug, target/debug/rationale
cargo build --release    # optimized, target/release/rationale
```

A release build embeds the Control Room only when `ui/dist` exists; build it
first with `npm --prefix ui ci && npm --prefix ui run build`.

## Control Room and website

```bash
npm --prefix ui ci && npm --prefix ui run typecheck && npm --prefix ui test && npm --prefix ui run build
npm --prefix site ci && npm --prefix site run check
```

## Skill validator fixtures

```bash
python3 skills/rationale/scripts/check_candidates.py skills/rationale/evals/fixtures/candidates-valid.json    # exit 0
python3 skills/rationale/scripts/check_candidates.py skills/rationale/evals/fixtures/candidates-invalid.json  # exit 2
```

## Dependency audit

```bash
cargo audit
```

Requires `cargo-audit` (`cargo install cargo-audit --locked`). The last recorded
audit and the dependency inventory are in `docs/dependencies/inventory.yaml`.
Run it again whenever the dependencies in the root `Cargo.toml` change.

## Regenerate the vertical-slice fixture

```bash
bash fixtures/vertical-slice/setup.sh
```

It generates a deterministic Git repository in `fixtures/vertical-slice/repo/`
(ignored by Git, see `.gitignore`) that several tests use, such as
`storage::tests::reads_fixture_record_with_approval_and_binding`.

## Stability under load

The suite is deterministic even when run many times in a row; it should not fail
intermittently. If a test fails only under repeated `cargo test` and never in
isolation, that is a real contention signal. Phase F5 found two real flakiness
bugs exactly this way: an `SQLITE_BUSY` caused by a missing `busy_timeout` and
redundant DDL in `cache.rs`, and temporary-directory name collisions caused by
insufficient clock resolution. Reproduce with:

```bash
for i in $(seq 1 20); do cargo test 2>&1 | grep -E "test result|FAILED"; done
```
