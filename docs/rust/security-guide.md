# Rust — security guide

Applies the principles of `Rationale_Arquitectura_Conceptual_v0.1.md §15` and
`Rationale_v0.5.md §4.10–4.11` to concrete Rust code.

## `unsafe`

- The spike uses a single `unsafe` block (direct FFI to `flock()` in
  `demo_file_lock`, `cfg(unix)`), documented with a comment explaining why it
  avoids an extra dependency (the `libc` crate) for one system call.
- Rule for the core: **every `unsafe` block must carry a `// SAFETY:` comment
  explaining the invariant that makes it correct.** The spike did not, because it
  was short-lived research code; production code must.
- Prefer the portable `std` path when it exists (`std::fs::File::lock`,
  available since Rust 1.89 — not used in the spike; see
  `docs/research/language/compatibility-matrix.md`) over manual FFI, unless there
  is a measured, documented reason not to.

## Repository content is data, not instructions

This applies directly to parsing YAML Records (`op1_read_record` in the spike):
deserialization targets a typed struct (`Record`), never a dynamic type that
could be executed. With `serde` this is correct by construction — the risk of
"text turned into instructions" (`Rationale_v0.5.md §4.10`) would require
deserializing into something interpretable as code, which this design never
does.

## Subprocesses

- Never build a shell command by concatenating strings. The spike uses
  `Command::new(script)` with separate arguments (`cmd.arg("slow")`), never
  `sh -c "{string}"`. Keep that discipline for every invocation of the Codebase
  Memory binary, agent CLIs, or helper scripts.
- Every subprocess needs an explicit deadline and a real, verified cancellation
  path. See the finding in `docs/research/language/candidates.md` about Go's
  footgun — the lesson applies equally to Rust: **do not assume that killing a
  process closes everything it inherited or launched**; verify it with a timing
  test, not only by reading the code.

## Paths

- Canonicalize and validate any path that comes from a `Record` or from
  configuration before reading or writing with it
  (`Rationale_Arquitectura_Conceptual_v0.1.md §15.3` treats versioned data in
  `.rationale/` as untrusted).
- Atomic writes (write to a temporary file, then rename) for every canonical
  file and every file `install-agent` owns. Refuse managed paths that cross a
  symbolic link.

## Dependencies

- `cargo audit` has been part of the quality gate since Phase F. The latest
  recorded run is in `docs/dependencies/inventory.yaml`. Repeat it whenever the
  dependencies of the root `Cargo.toml` change.
- `Cargo.lock` is versioned for reproducible builds, in the core and in the
  spike.

## Sensitivity

See `Rationale_v0.5.md §26.5` for the `visibility`/`sensitivity` rules the core
applies when reading and writing real Records.
