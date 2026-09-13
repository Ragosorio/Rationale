# Security baseline for the stable 1.0 series

This document is the security gate for the stable series. It does not claim that
the system is secure in general; it records minimum properties demonstrated by
tests, CI, or reproducible evidence, and it keeps the limits that are still open
separate.

## Boundaries and data

- All text from the repository, Records, paths, issues, or providers is
  untrusted data, never an instruction.
- Rationale is local-first: by default it does not upload code, prompts,
  Records, or secrets.
- Integration starts with discovery and preflight; any mutation is limited to
  the repository and the paths the person put in scope.
- `.env` files, private keys, tokens, dumps, and personal data are excluded
  unless explicitly and documentedly authorized.

## Integrity

- IDs pass `validate_safe_id`; traversal, separators, and NUL are rejected.
- Writes use a temporary file, `sync_all`, and an atomic rename.
- Review claims use an exclusive rename and leave `.in-review/` recoverable.
- Record mutations compare against the original YAML before overwriting.
- A corrupt YAML file produces a per-file diagnostic and does not bring down the
  resolver.
- `.rationale/` is never deleted during uninstall.

## Terminal and agents

- Free text is stripped of ANSI and control sequences before display.
- MCP can capture Records and carry conflicts forward, but never pin or unpin
  authority on its own.
- `pin`, `unpin`, and adopting the replacement of a pinned Record require
  declared authority; resolving a conflict through MCP requires the human's
  literal answer.
- Authority is resolved only from the project's canonical configuration.
- An undeclared actor cannot elevate itself.
- `install-agent` writes skills only inside the project, never through a
  symbolic link, with a content hash per file, and never overwrites a file the
  user edited. The `rationale` skill's validator script reads its input and
  `.rationale/records/`, writes nothing, and makes no network calls.

## Supply chain and release

- `cargo fmt`, `clippy -D warnings`, release-profile tests, and `cargo audit`
  are gates.
- Dependencies and licenses are reviewed from `docs/dependencies/inventory.yaml`.
- Every release artifact has SHA-256 and provenance/attestation.
- Installers are tested on a clean machine, and for update, rollback, and
  uninstall.

## 1.0 evidence

- CI verified tests, Clippy, and packaging on Windows; the suite covers
  concurrent claims and writes, recovery, and round-trip fidelity.
- The release builds five targets, including macOS and Linux ARM64, with
  checksums and attestations. The workflow blocks packaging until the source tag
  verification passes.
- The macOS ARM64 artifact was exercised as installed: CLI, MCP server, embedded
  Control Room, HTTP guards, and isolated migration of agent registrations.
- `cargo audit` reported no vulnerabilities in the 1.0 lockfile.

## Open limits (not P0/P1)

- Linux and Windows artifacts are compiled and packaged in CI, but there is no
  independent post-publication functional smoke test on each system yet.
- Several ADRs of the 1.0 series are still `proposed`; that limits their
  documentary authority, not the reproducible guards listed above.
- Each pilot remains responsible for reviewing its data exclusions, stale
  bindings, and canon before declaring `doctor --check` clean.
