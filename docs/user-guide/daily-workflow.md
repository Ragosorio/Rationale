# Flujo diario

## Antes del cambio

El agente prepara el target con su intención real. Por CLI puedes ver el mismo
packet:

```bash
rationale prepare "src/auth/authorization.rs::resolve" --intent "cambiar la resolución de permisos"
```

El packet JSON va a stdout y los diagnósticos a stderr. Revisa sobre todo:

- `critical_constraints` y `decisions`, con su `authority` y `provenance`;
- `relationships`, con su estado (`observed`, `indirect`, `orphaned`, `unknown`);
- `intent_conflicts` — señal léxica, no contradicción probada;
- `snapshot` y `warnings` de cobertura, y `budget_overflow` si aparece.

Guarda el `operation_id`: `finalize_change` lo usa para cerrar la operación.

## Durante el cambio

Si el código parece extraño, el agente llama a `explain_target` antes de
simplificarlo. No trata una inferencia del proveedor como autoridad.

## Después del cambio

El agente llama a `finalize_change` con el `operation_id`, un `summary` y los
`candidates`: solo conocimiento que seguirá siendo cierto. Rationale descarta
ruido, notas transitorias, rationales que repiten el statement y duplicados —
siempre con un motivo — y escribe el resto como Records en la misma llamada.
Sin candidatos no se escribe memoria.

Revisa lo capturado como revisas el código: `.rationale/records/` viaja en el
mismo pull request.

## Mientras trabajas

```bash
rationale ui
```

El [Control Room](control-room.md) muestra cada operación en vivo: qué recibió
el agente, qué capturó, qué descartó y qué explicaciones están en riesgo.

## Autoridad humana

Fija una regla que ningún agente debe reemplazar:

```bash
rationale pin <record-id> --reason "invariante de pagos"
```

Si un agente intenta reemplazarla, `finalize_change` devuelve un conflicto y no
escribe nada. Decides tú:

```bash
rationale conflicts
rationale resolve <conflict-id> keep-pinned    # o adopt-new, con autoridad declarada
```

Para corregir, disputar, revocar, reemplazar, cambiar la autoridad o añadir
evidencia a un Record existente:

```bash
rationale review-record <record-id>
```

Todas estas acciones exigen una terminal interactiva y dejan eventos de
lifecycle auditables.

## Proyectos que vienen de antes de 1.0

Si `.rationale/proposals/` todavía tiene propuestas pendientes:

```bash
rationale migrate --dry-run
rationale migrate
```

Las válidas se vuelven Records con procedencia `migrated`; las ruidosas se
archivan con su motivo en `.rationale/archive/proposals/`. Nada se borra.
`rationale review` sigue disponible para confirmarlas una a una si lo prefieres.
