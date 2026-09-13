# Reference development machine

Generated with `scripts/dev/collect-environment.sh`. This is the anonymized,
versionable version; the full JSON lives in `.rationale-local/environment.json`
(ignored by Git, regenerable on any machine).

Deliberately **not** recorded here: serial number, hardware UUID, provisioning
UDID, real hostname, or any personal path
(`Rationale_Arquitectura_Conceptual_v0.1.md §5`).

## Profile

| Field | Value |
|---|---|
| Machine | MacBook Air |
| Chip | Apple M4 |
| Cores | 10 (4 performance + 6 efficiency) |
| Memory | 16 GB |
| Architecture | arm64 (Apple Silicon) |
| Operating system | macOS 26.5.2 (build 25F84) |
| Git | 2.50.1 |
| Clang | Apple clang 21.0.0 |
| Xcode Command Line Tools | installed |
| Rust | rustc 1.97.1, cargo 1.97.1 (through rustup, stable) — installed for the language spike (Phase C) |
| Go | go1.26.5 darwin/arm64 (through Homebrew) — installed for the language spike (Phase C) |

**PATH note:** `rustc`/`cargo` live in `~/.cargo/bin`. `.zshenv` already adds it
to `PATH` for normal interactive shells, but some non-interactive shell
invokers do not read it; in that case, prefix the command with
`PATH="$HOME/.cargo/bin:$PATH"`. `scripts/dev/collect-environment.sh` already
does this internally.

## Design implications (`Rationale_Arquitectura_Conceptual_v0.1.md §5.2`)

- Avoid unnecessary resident processes; no mandatory daemon.
- Do not duplicate full indexes in memory.
- Large-scale tests must have explicit limits.
- Benchmarks must record peak memory.
- A daemon, if one ever exists, must be optional and lean.
- Frequent operations must use a local cache.
- Support Apple Silicon from the start; early development may prioritize
  macOS arm64.
- The core must not use macOS-only APIs.

## Reproduce

```bash
bash scripts/dev/collect-environment.sh
cat .rationale-local/environment.json
```
