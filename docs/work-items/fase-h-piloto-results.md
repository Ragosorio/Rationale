# Fase H — primera verificación read-only

Fecha: 2026-07-26. Binario local: `target/release/rationale` construido desde
la rama `release/fase-g-mvp-local`. Se inicializó únicamente la estructura
`.rationale/` vacía en Monorepo y BoostAPI; no se escribieron Records,
Subjects, propuestas ni configuraciones de agentes.

## Health inicial

| Proyecto | Git HEAD | Proveedor | Cobertura antes de consultar |
|---|---|---|---|
| Monorepo | `8588e14329a39e8f206296a94abcc1b840964de9` | successful | complete |
| BoostAPI | `3102e5d2b9d65861fe9eb5756a5e34d7aeeae96c` | successful | complete |

## Consultas read-only

| Proyecto | Target | Exit | Cobertura | Warnings | Resultado |
|---|---|---:|---|---:|---|
| Monorepo | `apps/web/app/api/employees/route.ts::resolveBoostApiUrl` | 0 | complete | 0 | packet y target resuelto |
| Monorepo | `packages/crm-services/src/http/services.ts::createHttpCrmServices` | 0 | complete | 0 | packet y target resuelto |
| BoostAPI | `src/employees/employees.controller.ts::EmployeesController.findAll` | 0 | unknown | 1 | packet honesto, símbolo fuera de cobertura |
| BoostAPI | `src/orders/orders.service.ts::OrdersService.create` | 0 | unknown | 1 | packet honesto, símbolo fuera de cobertura |

El primer intento contra los dos repositorios vacíos reveló un panic de
`prepare`; se corrigió en `pipeline::prepare`, que ahora devuelve packet vacío y
diagnóstico explícito cuando no hay Records. El test de regresión es
`tests/empty_project.rs`; las cuatro consultas posteriores terminaron con
exit 0.

## Gate actual

La integración local read-only funciona en ambos repositorios y no produce
escrituras fuera de la estructura inicial. El piloto comparativo de 20–30 casos
todavía no está cerrado: BoostAPI requiere una investigación específica de la
cobertura `unknown` y ground truth autorizado antes de habilitar captura
asistida. Los `.rationale/` recién creados quedan sin commit en esos repos para
que el dueño los revise.
