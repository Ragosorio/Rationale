# Conceptos esenciales

Rationale separa la memoria que escriben los agentes, la autoridad que conservan
las personas y el estado estructural que informa el proveedor.

## Entidades

- **Record:** una pieza de conocimiento durable — `constraint`, `decision`,
  `risk` o `exception` — con statement, rationale que da la causa, severidad,
  bindings, evidencia y lifecycle. Una decisión por Record.
- **Binding:** ata un Record al código que gobierna: archivo o símbolo. Los
  símbolos los confirma el proveedor y se guardan con id portable. Los que nacen
  de código sin commitear quedan `provisional`.
- **Relationship binding:** explica por qué existe una relación entre dos nodos.
- **Subject:** identidad conceptual de un comportamiento o frontera, para que
  una decisión no quede atada accidentalmente a un solo archivo.
- **Evidence:** referencia verificable que respalda la afirmación.
- **Assessment:** evaluación derivada de epistemología, autoridad,
  aplicabilidad, linkage y consistencia de revisión.
- **Operation:** lo que abre `prepare_change` y cierra `finalize_change`, con un
  snapshot local de lo considerado y lo seleccionado.
- **Conflict:** un candidato que intentó reemplazar un Record fijado. No se
  escribe hasta que una persona decide.

## Procedencia y autoridad

Cada Record declara su **procedencia**: `agent_asserted` (con cliente, sesión y
operación), `human_stated` o `migrated` (del flujo de aprobación anterior a
1.0). Nunca se asciende en silencio.

Y su **autoridad**: `normal` por defecto, o `pinned` cuando el proyecto la fijó.
Un Record `pinned` gobierna igual que uno normal, pero ningún agente puede
reemplazarlo: el intento se vuelve un conflicto. La precedencia es `pinned`
sobre `normal`, y un `supersedes` explícito sobre la coexistencia; los SHAs de
Git nunca se ordenan.

## Estado estructural de las relaciones

| Estado | Significado |
|---|---|
| `observed` | La relación directa existe en el índice actual. |
| `indirect` | Ya no es directa, pero un camino acotado sigue conectando los extremos. |
| `orphaned` | No se puede localizar; la explicación puede estar obsoleta. Nunca se borra. |
| `unknown` | El proveedor no pudo verificarla. La ausencia no es prueba. |

## Canon y derivados

El canon versionado vive en `.rationale/`. La cache SQLite, los snapshots de
operación y la actividad local son derivados o locales y se pueden borrar sin
perder decisiones. Borrar un Record sí elimina historia: revócalo o reemplázalo
con `rationale review-record` para que el lifecycle conserve el motivo.

## Responsabilidades

- Codebase Memory aporta ubicación, símbolos y relaciones estructurales.
- Rationale aporta por qué importa, qué debe seguir siendo cierto y quién puede
  moverlo.
- El agente prepara contexto y captura conocimiento durable.
- La persona fija las reglas que importan y decide los conflictos.

Consulta el contrato original en [`Rationale_v0.5.md`](../../Rationale_v0.5.md)
y el cambio de modelo de la 1.0 en
[`docs/work-items/vnext-implementation-plan.md`](../work-items/vnext-implementation-plan.md).
