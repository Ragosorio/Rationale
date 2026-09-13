# Internal dogfood evidence — Phase G

Date: 2026-07-26. Binary: commit `fabeb92400b485080278276105bd60b0a3e295c5`.
Repository: Rationale. Mode: `rationale prepare`, without writing Records or
proposals.

## Cases run

| # | Target | Provider | Coverage | Warnings | Tokens |
|---:|---|---|---|---:|---:|
| 1 | `src/review.rs::approve` | successful | complete | 0 | 207 |
| 2 | `src/review.rs::mutate_record` | successful | complete | 0 | 209 |
| 3 | `src/storage.rs::write_record` | successful | complete | 0 | 209 |
| 4 | `src/mcp/server.rs::call_prepare_change` | successful | complete | 0 | 211 |
| 5 | `src/subjects.rs::resolve` | successful | complete | 0 | 247 |
| 6 | `src/pipeline.rs::prepare` | successful | complete | 0 | 218 |
| 7 | `src/capture.rs::capture` | successful | complete | 0 | 220 |
| 8 | `src/configuration.rs::ResolvedConfig.authority_for_actor` | successful | unknown | 1 | 194 |
| 9 | `src/retrieval.rs::compile_packet` | successful | complete | 0 | 231 |
| 10 | `Cargo.toml` | successful | unknown | 1 | 194 |

Observed consistency: `working-tree-ahead` in every case, reported honestly
because the branch contains documentation, packaging, and artifacts from this
same run. It was not presented as an exact revision.

## Result

- 10/10 processes exited with code 0.
- 10/10 produced a ContextPacket.
- 8/10 had full structural coverage; the two `unknown` ones correspond to a
  method the provider did not expose and to a non-symbolic manifest.
- 8/10 had no warnings; the 2 warnings were explicit and did not become a false
  "does not exist".
- Median token proxy: 210; maximum: 247.
- The local logs show recent latencies of 48–80 ms with the cache and provider
  available (`.rationale-local/runs/vertical-slice.ndjson`).

The first pass inside the sandbox produced SQLite warnings because of an
unwritable cache path. It was repeated outside the sandbox, as a real local
environment, and the derived cache opened correctly; that incident is not
counted as a product defect, but it remains a requirement for the installation
smoke test.

## Gate

The internal dogfood supports the core for the dogfood tag. It does not yet
demonstrate comparative value against Codebase Memory, nor does it authorize
assisted capture on work repositories. That is left to
[`fase-h-piloto.md`](fase-h-piloto.md).
