Estás trabajando en un proyecto que usa Rationale para conservar el contexto de las decisiones.

Usa este protocolo al inicio de cada conversación que pueda cambiar código:

1. Si Codebase Memory está instalado, úsalo primero para localizar el símbolo objetivo, sus callers y los archivos relevantes. Te dice dónde está el código y cómo se conecta; no decide por qué debe conservarse así.
2. Antes de cambiar código no trivial, llama a `prepare_change(target, intent)` de Rationale usando el target encontrado y tu cambio real. Lee las restricciones, autoridad, evidencia, linkage, cobertura del proveedor y conflictos con la intención.
3. Si el packet reporta una restricción gobernante o un conflicto con tu intención, dilo explícitamente. Compara el cambio propuesto con el Record; no avances en silencio ni llames contradicción semántica probada a un conflicto indeterminado. Pide aclaración cuando la decisión no te corresponda.
4. Si el código parece innecesariamente complejo, redundante o “raro”, llama a `explain_target(target)` antes de simplificarlo. Puede ser una valla de Chesterton cuyo motivo vive en el canon.
5. Haz el cambio mínimo consistente con el contexto aprobado. Mantén a la vista los tests, la evidencia y la autoridad declarada del proyecto.
6. Después de un cambio no trivial, ejecuta los tests relevantes y llama a `finalize_change(...)` para que los hechos observados y el diff se conviertan en una propuesta pendiente cuando la política de captura lo requiera.
7. Una propuesta no es un Record aprobado. Nunca afirmes que una decisión está aprobada hasta que una persona complete `rationale review`.

Cuando Codebase Memory no esté disponible, continúa con la cobertura que reporte Rationale y declara esa limitación. Nunca inventes resolución de símbolos, autoridad, aprobación, evidencia ni resultados del proveedor.
