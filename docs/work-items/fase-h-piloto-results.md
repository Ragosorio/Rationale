# Phase H — first read-only verification

Date: 2026-07-26. Local binary: `target/release/rationale` built from the
`release/fase-g-mvp-local` branch. Only the empty `.rationale/` structure was
initialized in Monorepo and BoostAPI; no Records, Subjects, proposals, or agent
configurations were written.

## Initial health

| Project | Git HEAD | Provider | Coverage before querying |
|---|---|---|---|
| Monorepo | `8588e14329a39e8f206296a94abcc1b840964de9` | successful | complete |
| BoostAPI | `3102e5d2b9d65861fe9eb5756a5e34d7aeeae96c` | successful | complete |

## Read-only queries

| Project | Target | Exit | Coverage | Warnings | Result |
|---|---|---:|---|---:|---|
| Monorepo | `apps/web/app/api/employees/route.ts::resolveBoostApiUrl` | 0 | complete | 0 | packet and resolved target |
| Monorepo | `packages/crm-services/src/http/services.ts::createHttpCrmServices` | 0 | complete | 0 | packet and resolved target |
| BoostAPI | `src/employees/employees.controller.ts::EmployeesController.findAll` | 0 | unknown | 1 | honest packet, symbol outside coverage |
| BoostAPI | `src/orders/orders.service.ts::OrdersService.create` | 0 | unknown | 1 | honest packet, symbol outside coverage |

The first attempt against the two empty repositories revealed a panic in
`prepare`; it was fixed in `pipeline::prepare`, which now returns an empty packet
and an explicit diagnostic when there are no Records. The regression test is
`tests/empty_project.rs`; the four later queries exited with code 0.

## Current gate

The local read-only integration works in both repositories and produces no writes
outside the initial structure. The comparative pilot of 20–30 cases is not closed
yet: BoostAPI requires a specific investigation of the `unknown` coverage and an
authorized ground truth before enabling assisted capture. The newly created
`.rationale/` directories stay uncommitted in those repositories so the owner can
review them.
