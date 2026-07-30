# Uninstall

La desinstalación elimina primero los registros MCP globales que todavía
apuntan a ese binario y después elimina el ejecutable. Conserva todo el canon
`.rationale/`.

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-uninstall.sh | sh
```

Los datos canónicos no se eliminan nunca de forma automática.

Esto solo desinstala el binario. Si `rationale init`/`install-agent` avisó a
algún agente **dentro de un proyecto** (bloque en `CLAUDE.md`/`AGENTS.md` o
`.cursor/rules/rationale.mdc`), revertir eso es por proyecto:

```bash
cd /ruta/al/proyecto
rationale uninstall-agent
```

Para revertir solo Claude Code, Codex y Cursor a nivel del usuario, sin quitar
los bloques de ningún proyecto:

```bash
rationale uninstall-agent --global-only
```

Borra solo lo que `install-agent` escribió (según `.rationale-local/installed-agent-files.json`), dejando cualquier contenido previo del usuario en esos archivos intacto.

## Lo que es seguro borrar siempre

```bash
rm -rf ~/.cache/rationale                    # capa derivada de TODOS los proyectos — ver cache-reset.md
rm -f /ruta/al/proyecto/.mcp.json            # si solo lo usabas para Rationale
rm -rf /ruta/a/Rationale/target              # artefactos de build
```

`.rationale-local/` de cada proyecto (logs de instrumentación, nunca versionado) también es seguro de borrar:

```bash
rm -rf /ruta/al/proyecto/.rationale-local
```

## Lo que NUNCA se debe borrar sin pensarlo (es el canon, versionado en Git)

```
.rationale/records/      # decisiones aprobadas — borrar esto pierde autoridad real
.rationale/subjects/      # identidad conceptual de cada comportamiento gobernado
.rationale/approvals/
.rationale/bindings/
```

Si de verdad quieres quitar Rationale de un proyecto por completo:

```bash
git rm -r .rationale/ .mcp.json   # queda en el historial de Git, recuperable
git commit -m "remove Rationale from this project"
```

**Nunca `rm -rf .rationale/` seguido de un force-push** — eso sí sería destruir decisiones aprobadas sin posibilidad de recuperación. Usar `git rm` deja el historial intacto.

## Propuestas pendientes o rechazadas

```bash
rm -rf .rationale/proposals/          # nunca se auto-generan sin que finalize_change las haya escrito
```

Ninguna propuesta pendiente o rechazada tiene autoridad — son siempre seguras de borrar si de verdad no interesan; `git rm` es igualmente válido si quieres conservarlas en el historial.
