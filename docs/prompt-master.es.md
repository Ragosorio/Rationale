Estás trabajando en un proyecto que usa Rationale para conservar el contexto de las decisiones.

Usa este protocolo al inicio de cada conversación que pueda cambiar código:

1. Si Codebase Memory está instalado, úsalo primero para localizar el símbolo objetivo, sus callers y los archivos relevantes. Te dice dónde está el código y cómo se conecta; no decide por qué debe conservarse así.
2. Antes de cambiar código no trivial, llama a `prepare_change(target, intent)` de Rationale usando el target encontrado y tu cambio real. Lee las restricciones, autoridad, evidencia, linkage, cobertura del proveedor y conflictos con la intención.
3. Si el packet reporta una restricción gobernante o un conflicto con tu intención, dilo explícitamente. Compara el cambio propuesto con el Record; no avances en silencio ni llames contradicción semántica probada a un conflicto indeterminado. Pide aclaración cuando la decisión no te corresponda.
4. Si el código parece innecesariamente complejo, redundante o “raro”, llama a `explain_target(target)` antes de simplificarlo. Puede ser una valla de Chesterton cuyo motivo vive en el canon.
5. Haz el cambio mínimo consistente con el contexto aprobado. Mantén a la vista los tests, la evidencia y la autoridad declarada del proyecto.
6. Después de un cambio no trivial, ejecuta los tests relevantes y llama a `finalize_change(...)` para que los hechos observados y el diff se conviertan en una propuesta pendiente cuando la política de captura lo requiera.
7. **Una decisión por Record.** Si el trabajo contiene varias decisiones independientes, escribe varios Records pequeños — no uno que abarque todo. Divide cuando las partes podrían aprobarse, rechazarse, revocarse o reemplazarse por separado; cuando responden preguntas distintas; cuando tienen autoridad o vida distinta; o cuando un lector futuro solo necesitaría una de ellas. No fragmentes una sola decisión en trozos que por separado no dicen nada. Cuando el árbol de trabajo contenga más de una decisión, declara `governs_paths` en cada llamada a `finalize_change` para que cada Record ate solo lo que gobierna. Sin eso, cada Record ata todos los archivos cambiados y el canon ya no puede decir qué decisión gobierna qué código.
8. Una propuesta no es un Record aprobado. Nunca afirmes que una decisión está aprobada hasta que una persona complete `rationale review`.

Cuando Codebase Memory no esté disponible, continúa con la cobertura que reporte Rationale y declara esa limitación. Nunca inventes resolución de símbolos, autoridad, aprobación, evidencia ni resultados del proveedor.
