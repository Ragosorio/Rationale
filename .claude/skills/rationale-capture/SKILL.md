---
description: "Captura hechos y una propuesta pendiente después de un cambio."
argument-hint: "[statement]"
arguments: ["statement"]
disable-model-invocation: false
---

Cierra el cambio actual con Rationale. Statement opcional del humano:

`$statement`

Contexto Git vivo inyectado por el skill:

- HEAD actual: !`git rev-parse HEAD`
- Estado: !`git status --short`
- Diff desde HEAD: !`git diff --no-ext-diff HEAD`

Si esas líneas todavía aparecen como literales `!`comando`` (por ejemplo,
porque recibiste esta acción mediante un prompt MCP), obtiene los mismos datos
con las herramientas Git disponibles antes de continuar.

1. Usa el `base_revision` real reportado por el preflight; si no existe, determina y declara la revisión base correcta en vez de inventarla.
2. Revisa el diff y las pruebas ejecutadas. Separa hechos observados de intención o inferencia.
3. Cuenta cuántas decisiones independientes contiene el cambio. **Una decisión por Record.** Divide cuando las partes podrían aprobarse, rechazarse, revocarse o reemplazarse por separado; cuando responden preguntas distintas; cuando tienen autoridad o vida distinta; o cuando un lector futuro solo necesitaría una de ellas. No fragmentes una sola decisión en trozos que por separado no dicen nada.
4. Llama `finalize_change(...)` con target, base_revision, intent, statement, severity y metadatos de Subject/Record reales. Usa el statement de arriba solo si no está vacío y refleja la decisión.
5. Si hay más de una decisión en el árbol de trabajo, haz una llamada por decisión y declara `governs_paths` en cada una con las rutas que esa decisión gobierna. Sin eso, cada Record ata todos los archivos del diff y el canon ya no puede decir qué decisión gobierna qué código.
6. Reporta si se escribió una propuesta pendiente o si el cambio fue mecánico. Nunca llames aprobada a una propuesta: solo `rationale review` humano puede aprobarla.
