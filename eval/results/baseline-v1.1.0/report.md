# Rationale evaluation — synthetic `parcel` project

Generated 2026-10-14T15:47:00Z on `macOS-26.5.2-arm64-arm-64bit`.
**Synthetic data.** The project, its Records and its structural index are invented to
reproduce reported failure modes; these numbers are not results from any real repository.
Token estimator: ASCII chars/2.7 + code chars/2.0 + 1 per non-ASCII char (see eval/README.md).

## Summary

| Metric | v1.1.0 |
|---|---|
| Binary | rationale v1.1.0 |
| Retrieval scenarios passed | 1/14 |
| Superseded/revoked Records served as authority | 6 |
| Records of another symbol served as authority | 3 |
| Required Records missing | 10 |
| Irrelevant Records served | 8 |
| Response tokens p50 / p95 / max | 741 / 2213 / 2427 |
| Warm prepare_change ms p50 / p95 | 46.19 / 131.9 |
| Warm explain_target ms p50 / p95 | 1.07 / 2.53 |
| Cold start + prepare ms p50 / p95 | 43.3 / 96.55 |
| New-symbol capture governs only its symbol | False |
| Open operations visible and abandonable | False |
| Doctor detects (of 4) | 1 |
| Retired module repaired reversibly | False |
| Compatible with the other binary's canon | — |

## Scenarios

| Scenario | v1.1.0 |
|---|---|
| Q1 explain a symbol whose decision was superseded (both sides marked) | FAIL, 625 tok — forbidden: decision.rate-cache-ttl-5m; 2 irrelevant |
| Q2 explain a symbol whose old decision is replaced only through `supersedes` | FAIL, 741 tok — forbidden: constraint.no-weekend-pickups, decision.label-retry-fixed-3x; 2 irrelevant |
| Q3 prepare a symbol in a file with Records bound to other symbols | FAIL, 2213 tok — forbidden: decision.label-retry-fixed-3x; 2 irrelevant |
| Q4 prepare a folder | FAIL, 419 tok — missing: constraint.money-in-integer-cents, constraint.shipment-idempotency-key, decision.cancel-requires-carrier-ack, decision.dhl-sandbox-env-flag, decision.label-retry-exponential-backoff, decision.rate-cache-ttl-60s, decision.remote-zone-flat-surcharge |
| Q5 prepare a billing symbol that calls the shipping invariant | FAIL, 1416 tok — missing: constraint.money-in-integer-cents |
| Q6 prepare the secret rotation symbol with an orphaned exception | FAIL, 1581 tok — orphaned_exception_flagged: the orphaned exception is not flagged |
| Q7 prepare a symbol that Records of a retired module still bind | FAIL, 2427 tok — 1 irrelevant; stale_records_flagged: served without a stale flag: ['constraint.comunicaciones-template-ids-stable', 'decision.comunicaciones-email-via-outbox']; no_phantom_nodes: nodes from files that no longer exist: ['src/comunicaciones/outbox.ts'] |
| Q8 explain a folder | FAIL, 140 tok — missing: decision.notifications-replaces-comunicaciones, decision.sms-max-160-chars |
| Q9 prepare a symbol created on the branch and missing from the index | FAIL, 1972 tok — forbidden: constraint.shipment-idempotency-key, decision.cancel-requires-carrier-ack, decision.label-retry-exponential-backoff, decision.label-retry-fixed-3x; index_incomplete_reported: retrieval.status=None |
| Q10 prepare with a tight budget | FAIL, 1391 tok — 1 irrelevant; within_budget: 1391 estimated tokens for a budget of 700 |
| Q11 prepare a path of the retired module | FAIL, 316 tok — retired_target_explained: retrieval.status=None, records_bound_to_missing_path=0 |
| Q12 prepare a whole file | pass, 1039 tok |
| Q13 prepare a symbol whose pinned rule superseded an older rule | FAIL, 470 tok — forbidden: constraint.rotate-secrets-90d; orphaned_exception_flagged: the orphaned exception is not flagged |
| Q14 prepare a symbol no Record governs | FAIL, 731 tok — none_found_reported: retrieval.status=None |
