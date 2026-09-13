# Work item: alpha-release-mcp-cleanroom-hardening

## Problem

The cleanroom test of the public binary found three surfaces that are currently
confused with one another:

1. The landing page installs `v0.0.0-dogfood.7` because that is still the most
   recent public release. The code of `release/v0.1.0-alpha.1`, including
   `install-agent`, `uninstall-agent`, and the non-mutating `--help`, is not
   published yet.
2. `rationale serve` correctly stays open when run by hand, but does not
   complete the handshake with Codex. Rationale's server uses `Content-Length`
   framing; the standard MCP stdio transport uses newline-delimited JSON
   messages.
3. An explicit path without `.rationale/` reaches `expect(...)` and panics in
   commands like `health`, instead of producing a controlled user error.

There is also a version ambiguity: `0.9.0` corresponds to Codebase Memory in the
cleanroom conversation, not to a Rationale release. The next tag planned in this
repository is `v0.1.0-alpha.1`; there are no public tags `dogfood.8`,
`dogfood.9`, or `v0.9.0`.

## Goal

Publish an installable alpha whose website, installer, and binary declare the
same version; whose MCP server completes a real handshake with Codex; and whose
CLI turns invalid inputs into clear errors without a panic or backtrace.

## Non-goals

- Do not redesign the landing page.
- Do not modify Codebase Memory or read its internals.
- Do not adopt `rmcp` automatically: first evaluate the minimal fix that
  conforms to the standard stdio transport.
- Do not add a daemon or make `serve` detach from the parent process.
- Do not hide that `rationale serve` waits for traffic on stdin when run
  directly.
- Do not automatically approve ADRs, Subjects, proposals, or Records.
- Do not continue the functional pilot until the real MCP handshake passes.

## Base revision

`fa1c1cacb188b87081ec670466cdf43f36123169`
(`release/v0.1.0-alpha.1`).

## Evidence

### Coverage and review

- Codebase Memory was `ready` for `Users-roor.osorio-Desktop-Rationale`, with
  3,202 nodes and 6,672 edges.
- The graph was used to locate `src/mcp/server.rs::run`,
  `ProviderHandle::spawn`, `CodebaseMemoryClient::spawn_with`,
  `pipeline::health`, and the MCP tests.
- Files not covered by the graph were reviewed directly: installers, the release
  workflow, the landing page, runbooks, and ADRs.
- Coverage warning: the exact Sites deployment could not be resolved; the
  repository contains no `.openai/hosting.json` and the connected account listed
  no project. The local source and `dist` do declare `dogfood.7` and use
  `releases/latest`.

### Release and installer

- GitHub reported `v0.0.0-dogfood.7`, published on 2026-07-26, as the only
  `Latest` release, with the expected installers and packages.
- Local `git tag` also ends at `v0.0.0-dogfood.7`.
- The landing page runs
  `releases/latest/download/rationale-installer.sh`; the installer queries
  `releases/latest`. Therefore downloading `dogfood.7` is today the correct
  behavior of those two pieces, not a Sites caching failure.
- The base commit has two green CI runs (push and pull request), but no tag or
  release.

### MCP handshake

- The MCP specification for stdio requires newline-delimited JSON-RPC.
- `src/mcp/framing.rs`, `src/mcp/server.rs`, and `tests/mcp_server.rs` use
  `Content-Length` exclusively.
- Reproduction against the real binary of the base commit:
  - a newline-delimited `initialize` ended without a response;
  - the same `initialize` with `Content-Length` received a valid response.
- This explains Codex's exact symptom: client and server wait for different
  delimiters until the client reports a timeout.
- Raising `startup_timeout_sec` does not fix a framing incompatibility.
- The server also creates `ProviderHandle` before reading `initialize`; the
  server's handshake should not depend on Codebase Memory starting or being
  healthy.

### CLI panic

- `cmd_health` accepts the value of `--project-root` directly.
- `pipeline::health` calls
  `configuration::load(project_root).expect("cargar configuración")`.
- A wrong path produces `NoRationaleDirFound` and a panic. The error already has
  a readable representation in `ConfigError`; the `expect` discards it.

### Public help

- The base commit already includes global and per-subcommand help, with tests
  verifying that `--help` does not create `.rationale/`, agent files, or MCP
  configuration.
- The `dogfood.7` release predates those changes; that is why the installed user
  does not see `install-agent` or `uninstall-agent`.

## Risks

- Changing the shared framing without separating roles would break Rationale's
  client toward Codebase Memory, which currently uses `Content-Length`.
- Answering `initialize` with an incompatible protocol version can make Codex
  close the session even after fixing the framing.
- Marking the alpha as a GitHub prerelease while keeping `releases/latest` can
  keep the landing page installing `dogfood.7`: GitHub does not treat
  prereleases as the stable `latest` release.
- Publishing before the cleanroom smoke test would repeat installing a binary
  whose content does not match the documentation.
- Converting only `health` to `Result` would leave equivalent panics in
  `prepare`, `review`, `review-record`, `install-agent`, and `uninstall-agent`.

## Plan

### P0 — block promotion and fix the version contract

1. Do not create `v0.1.0-alpha.1` until P1–P4 are closed.
2. Define two explicit channels in ADR-0010:
   - `stable`: may use GitHub `releases/latest`;
   - `preview`: must point at an explicit alpha tag or at a versioned manifest
     that includes prereleases.
3. Choose a single version source for the package, binary, installer, landing
   page, runbook, and release notes.
4. Add `rationale --version` and make the release package report the real tag,
   not only `Cargo.toml = 0.0.0`.

### P1 — fix the MCP server

1. Reopen ADR-0007 with the cleanroom evidence. The shared `Content-Length`
   decision does not conform on the stdio server side.
2. Separate the codecs per boundary:
   - Rationale client → Codebase Memory: keep the framing the provider requires
     while that is its real contract;
   - Rationale server ← Codex: line-delimited JSON conforming to MCP stdio.
3. Answer `initialize` and `tools/list` before starting Codebase Memory. Create
   the provider lazily on the first tool that needs it.
4. Negotiate and test a protocol version Codex accepts; do not infer
   compatibility from the provider's version number.
5. Keep stdout exclusively for MCP messages and stderr for diagnostics.

### P2 — remove panics from user input

1. Make `pipeline::health`, `prepare`, `explain`, and `finalize` propagate typed
   errors instead of using `expect` for expected configuration or I/O.
2. Centralize the resolution and validation of `--project-root`.
3. Turn CLI errors into a short message + a non-zero exit code, without
   `thread 'main' panicked` or the `RUST_BACKTRACE` hint.
4. Turn the same errors into `isError: true` for MCP without relying on
   `catch_unwind` as control flow.
5. Audit the other subcommands that accept `--project-root`.

### P3 — complete help and diagnostics

1. Keep the current tests of non-mutating `--help`.
2. Expand `rationale serve --help` to explain that the process uses stdio, stays
   open, and is normally started by the agent.
3. Add `--version` to the global help and to the package test.
4. Document how to tell a real MCP call from a CLI run:
   `Called rationale.health` versus `Ran rationale health`.
5. Print no banners on stdout during `serve`.

### P4 — make the website, release, and installer consistent

1. Remove the duplicated `dogfood.7` string in Astro and JavaScript and derive
   it from a single versioned source.
2. For the alpha stage, show and install `v0.1.0-alpha.1` explicitly; do not
   depend on `releases/latest` if the release will be a prerelease.
3. Make the installer fail clearly if it cannot resolve a tag or assets are
   missing, and print the installed version, origin, and path.
4. Add a CI check that compares the tag, asset names, `rationale --version`
   output, the landing page command, and the runbook.
5. Recover or persist `.openai/hosting.json` before the next deploy so the
   published version is auditable from the repository.

### P5 — validation and publication

1. Run the formatter, Clippy, release tests, audit, and the landing page build.
2. Run MCP conformance with an independent newline client and then with real
   Codex:
   - handshake without a timeout;
   - `tools/list` returns four tools;
   - a native call to `health`;
   - a degraded provider without Codebase Memory and a complete one after
     indexing.
3. Run the installer smoke test on the produced packages before publishing.
4. Get an independent adversarial review of the transport, errors, and release
   contract.
5. Publish the alpha, deploy the exact landing page, and repeat the cleanroom
   test from a new macOS user.

## Tests

### MCP

- A newline `initialize` answers within a short deadline even when Codebase
  Memory does not exist or is slow.
- `notifications/initialized`, `tools/list`, and consecutive calls use one JSON
  object per line and keep stdout clean.
- `health`, `prepare_change`, `explain_target`, and `finalize_change` appear in a
  real Codex session.
- A malformed message returns a JSON-RPC error and the session stays alive.
- The internal client toward Codebase Memory keeps its separate contract tests;
  no test implicitly shares the framing of both boundaries.

### CLI

- `health --project-root <without-.rationale>` exits non-zero with a readable
  error and no panic.
- The same invariant covers `prepare`, `review`, `review-record`,
  `install-agent`, and `uninstall-agent`.
- Every `--help` exits zero and writes no files.
- `serve --help` returns immediately.
- `--version` matches the packaged tag.

### Release and website

- Every expected asset exists and its checksum passes.
- The pinned installer installs exactly the requested tag.
- The command shown by the landing page installs that same version.
- Reinstalling is idempotent and does not duplicate MCP entries or instructions.
- Uninstalling preserves `.rationale/`.
- The built HTML contains no stale references to `dogfood.7`.

### Quality gates

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --release
cargo audit
npm --prefix site run build
```

## Docs

- Review ADR-0007 and ADR-0010 without self-accepting them.
- Update `docs/architecture/code-map.md` to separate both codecs.
- Update `docs/runbooks/diagnostics.md`: newline stdio, the expected silent
  process, and a native test from Codex.
- Update the install, release, and rollback runbooks with the
  `stable`/`preview` channels.
- Update the quickstart, CLI reference, and agents guide.
- Record the cleanroom result with version, SHA, OS, Codex, and evidence of a
  native MCP call.

## Success criterion

- The landing page installs the tag it shows.
- `rationale --version` confirms that same tag.
- `rationale --help` shows every public command.
- No subcommand panics on an invalid user path.
- Codex starts Rationale without a timeout warning and calls `rationale.health`
  as an MCP tool, not through a shell.
- The test without a provider degrades honestly and the test with an index
  reports real coverage.
- CI and every quality gate pass.
- An independent review tries to falsify the solution.
- The affected ADRs are reviewed, but only the human decides their status.
- The full cleanroom test passes from installation through persistence after
  reinstalling, without deleting `.rationale/`.

## Implementation status — 2026-07-26

Changes P1–P4 and the local validation of P5 were implemented:

- `serve` uses line-delimited JSON for its MCP boundary and keeps
  `Content-Length` only for the internal client toward Codebase Memory; the
  provider starts lazily after the handshake.
- `health`, `prepare`, `explain`, `finalize`, `review`, `review-record`,
  `install-agent`, `uninstall-agent`, and `init` turn expected path or
  configuration errors into controlled messages, without a panic.
- `--help`, `--version`, and `rationale update` are covered by tests; the
  installer installs the update helper next to the binary.
- The helper distinguishes `stable` and `preview`: the packaged alpha looks for
  the most recent prerelease and cannot silently degrade to the stable `latest`.
- The landing page and quickstart point explicitly at `v0.1.0-alpha.4` so a
  prerelease is not confused with GitHub `latest`; the workflow marks tags with a
  hyphen as prereleases and publishes the helpers.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --release`, `cargo audit --no-fetch --no-yanked`,
  `bash scripts/check-docs.sh`, and `npm run build` in `site/` passed.
- The independent stdin/stdout `initialize` smoke test answered in under a
  second, and the 11 MCP tests kept persistent sessions.

The first alpha publications (`v0.1.0-alpha.1` to `v0.1.0-alpha.3`) exposed
defects in the helper's preview pipeline (`curl` received SIGPIPE because of a
premature `head`). The fix will be published as `v0.1.0-alpha.4`; the landing
page and runbooks point at that tag so the update path is really usable.

Pending outside the local checkout: repeating the installation from a clean
macOS account and deploying the landing page through Sites. The Sites connector
exposed no project or `.openai/hosting.json` in this environment; that deployment
was not simulated.
