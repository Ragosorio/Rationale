# Evaluation

A reproducible check of what Rationale serves to an agent: whether the context
is current, scoped to the target, within its budget, and honest about what it
does not know. It runs any Rationale binary — a published release or a local
build — against the same inputs, so results can be compared before and after a
change.

## What is synthetic and what is real

- **`run_eval.py` — synthetic.** It builds the invented `parcel` project from
  [`fixtures/parcel/`](fixtures/parcel/): TypeScript files, 29 Records in the
  v1.1 format, and a structural index (`graph.json`) served by the fixture
  provider. The project reproduces, at small scale, failure modes reported by
  a team that uses Rationale: superseded Records served as current, a symbol
  query that returns the whole file, an empty folder query, a packet over its
  budget, operations left open, a stale index that cannot bind a new symbol, a
  constraint that does not reach a module sharing its invariant, a retired
  module whose Records stay active (`src/comunicaciones/`, 12 Records standing
  in for the 116 reported), and an orphaned exception. **None of these numbers
  are results from a real repository.**
- **`real_repo.py` — real, on one repository.** It clones a repository at
  `HEAD` into a temporary directory and runs `prepare_change` over a list of
  targets with the real Codebase Memory provider and an isolated `HOME` (the
  first call pays a cold index). The committed results use Rationale's own
  repository and [`real_targets.txt`](real_targets.txt).

Neither script touches your installation, your caches, or the repository you
point it at: every run uses a temporary copy and a temporary `HOME`.

## Running it

```bash
cargo build --release
python3 -I eval/run_eval.py \
  --binary v1.1.0=/path/to/rationale-v1.1.0 \
  --binary dev=target/release/rationale \
  --out eval/results/latest
python3 -I eval/real_repo.py --binary dev=target/release/rationale \
  --repo . --targets eval/real_targets.txt --out eval/results/latest
```

With two binaries, the first is treated as the previous release: the
compatibility suite writes a Record with each binary and reads it with the
other, and the repair suite checks how the previous release sees a canon after
`rationale retire`.

## What is measured

| Suite | Measures |
|---|---|
| Retrieval (14 scenarios) | Records served as current authority versus the expected set: required Records missing, superseded or revoked Records served, Records of another symbol served, irrelevant Records; per-scenario checks (orphaned exception flagged, stale Records flagged, no nodes from deleted files, index gap reported, budget respected, abstention reported) |
| Tokens | The estimate of the **complete serialized tool response** the agent receives, not only the packet |
| Latency | Warm calls in one server session (p50/p95) and cold starts (process start, initialize, one `prepare_change`) |
| Capture | A Record bound to a symbol the index does not know must govern that symbol and no other |
| Operations | Open operations are visible in `health` and `rationale operations`, and can be abandoned without being recorded as finalized |
| Doctor | Orphaned exception, inconsistent supersession, retired scope grouped, bindings to missing paths |
| Repair | `rationale retire` plans without writing, applies, stops serving retired Records, and its undo restores the canon byte for byte |
| Compatibility | A Record written by each binary is read as current by the other |

## Token estimate

Rationale has no tokenizer offline. Up to v1.1 it estimated `UTF-8 bytes / 4`
over the packet only. Anthropic's current tokenizer (Claude 4.7 and later)
produces about 30% more tokens than earlier models, and a third-party
measurement with `count_tokens` (playcode.io, July 2026, one fixture per
content type) found about 3.0 characters per token for JSON, 3.3 for English,
and 2.2–2.45 for code. Bytes/4 therefore undercounts by roughly 1.3–1.8×.

The estimator used here and in `src/tokens.rs` counts ASCII characters at 2.7
per token, re-counts source code at 2.0, and counts every non-ASCII character
as one token. It is meant to be an upper bound for typical responses: by the
measurements above it overestimates JSON and prose by about 10–25% and code by
about 10–20%. It has not been validated against `count_tokens` in this
repository (that needs network access and an API key); the reports show the
legacy bytes/4 figure next to it for comparison.

## Results

- [`results/baseline-v1.1.0/`](results/baseline-v1.1.0/) — the published
  `v1.1.0` binary before this work.
