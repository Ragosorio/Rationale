# Conceptos esenciales

Rationale separa hechos mecánicos, conocimiento humano y estado de vigencia.

## Entidades

- **Subject:** identidad conceptual de un comportamiento o frontera. Evita que
  una decisión quede atada accidentalmente a un solo archivo.
- **Record:** afirmación versionada sobre ese Subject, con severidad, razón,
  evidencia, bindings y autoridad.
- **Evidence:** referencia verificable que respalda la afirmación.
- **Assessment:** evaluación derivada de epistemología, autoridad,
  aplicabilidad, linkage y consistencia de revisión.
- **Proposal:** Record pendiente capturado por el agente; no tiene autoridad.
- **Approval:** evento humano que otorga autoridad a un Record.

## Canon y derivados

El canon versionado vive en `.rationale/`. El SQLite/FTS y los logs locales son
derivados regenerables. Borrar la cache no borra decisiones; borrar Records sí
puede eliminar autoridad histórica y debe hacerse mediante Git y revisión.

## Confianza

Una propuesta observada por captura mecánica no equivale a una decisión
aprobada. La autoridad proviene de una aprobación humana compatible con la
configuración declarada del proyecto. La aplicabilidad depende de la revisión
Git y del estado del proveedor.

## Responsabilidades

- Codebase Memory aporta ubicación, símbolos y relaciones estructurales.
- Rationale aporta por qué importa, qué restricciones existen y quién puede
  aprobarlas.
- El agente consulta y prepara contexto.
- La persona decide y deja evidencia auditable.

Consulta el contrato completo en [`Rationale_v0.5.md`](../../Rationale_v0.5.md).
