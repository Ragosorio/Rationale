# Fixture: vertical slice (Fase D2)

Fixture determinista de **un solo paquete** (deliberadamente, ver `docs/research/codebase-memory/08-workspaces-and-monorepos.md` — un fixture multi-paquete probaría una capacidad de resolución cross-package que el proveedor hoy no entrega de forma confiable).

## Contenido

```text
fixtures/vertical-slice/
├── source/            # fuente versionada del fixture (auth.resolveEntityRole)
├── setup.sh            # genera repo/ de forma determinista (mismo SHA siempre)
├── repo/               # generado por setup.sh — NO versionado (regenerable)
└── .rationale/
    ├── subjects/authorization.entity-scoped-staff-access.yaml
    └── records/constraint.no-global-admin-for-staff.yaml   (con approvals + binding)
```

`repo/` no se versiona porque es 100% regenerable: `setup.sh` fija autoría y fechas (`GIT_AUTHOR_DATE`/`GIT_COMMITTER_DATE`) para que el commit resultante tenga **siempre el mismo SHA**, en cualquier máquina.

## Uso

```bash
bash fixtures/vertical-slice/setup.sh
# Revisión determinista: cb878c9d598e54a2a9aa3993395513f7ccfff325
```

El `bound_revision` en `constraint.no-global-admin-for-staff.yaml` debe coincidir exactamente con este SHA. Si `source/` cambia intencionalmente, actualizar ambos: el `EXPECTED_SHA` en `setup.sh` y el `bound_revision` en el Record, en el mismo commit.

## Qué prueba

Este fixture es el input de la vertical slice de Fase D (`init → leer Record → resolver target → consultar CBM → verificar revisión → devolver una constraint compacta`). El caso reutiliza el ejemplo canónico de `Rationale_v0.5.md §2, §9, §27` (autorización por entidad) — no es un dato de proyecto real.
