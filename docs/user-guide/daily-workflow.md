# Flujo diario

## Antes del cambio

Pide al agente que prepare el target con su intención. Por CLI:

```bash
rationale prepare "src/auth/authorization.rs::resolve" --intent "cambiar la resolución de permisos"
```

El resultado es un packet JSON en stdout. Los diagnósticos se mantienen en
stderr. Revisa especialmente `health`, constraints críticas, conflictos con la
intención y advertencias de cobertura.

## Durante el cambio

El agente puede consultar `explain_target` si encuentra código que parece
extraño. No debe tratar una inferencia del proveedor como aprobación ni editar
el canon sin un flujo explícito.

## Después del cambio

El agente llama `finalize_change` con el target, revisión base, intención y una
afirmación propuesta cuando el cambio contiene una señal de alto valor. Rationale
captura el diff y escribe una propuesta pendiente; un cambio mecánico puede no
producir propuesta.

## Revisión humana

```bash
rationale review
```

Cada propuesta aparece resumida, no como YAML completo. Las opciones son:

- `approve` o `approve-critical` según severidad;
- `c` para corregir el statement y luego confirmarlo;
- `r` para rechazarla conservando historial;
- cualquier otra entrada para saltarla.

Para un Record ya aprobado:

```bash
rationale review-record <record-id>
```

Ese lifecycle permite corregir, disputar, revocar, superseder, cambiar
autoridad y añadir evidencia. Las mutaciones son interactivas y dejan eventos
auditables.

## Revisión de cambios fuera del flujo

Si el cambio ya ocurrió, no inventes una aprobación retrospectiva. Usa la
evidencia de Git y documentos existentes para preparar una propuesta, marca lo
que sea desconocido y deja la decisión pendiente hasta que una persona la
revise.
