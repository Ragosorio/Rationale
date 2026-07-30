# Configuración, archivos y privacidad

## Estructura del proyecto

```text
.rationale/
├── subjects/     # identidad conceptual
├── records/      # decisiones aprobadas
├── proposals/    # propuestas pendientes o rechazadas
├── bindings/     # referencias canónicas cuando existan
├── approvals/    # estructura reservada del canon
├── schemas/      # schemas locales
└── migrations/   # migraciones del formato
```

`.rationale-local/` contiene logs de instrumentación y no se versiona. La capa
derivada SQLite/FTS vive en `~/.cache/rationale/` y se puede regenerar.

## Variables del instalador

- `RATIONALE_VERSION`: fija una versión de Release.
- `RATIONALE_CHANNEL`: selecciona `preview` (por defecto mientras el proyecto
  sea pre-1.0) o `stable` para que el instalador y `rationale update`
  resuelvan el canal correspondiente. `stable` usa `GET /releases/latest` de
  GitHub, que excluye prereleases — con solo alfas publicadas, resolvería a
  una Release anterior a `install-agent` y al helper de actualización.
- `RATIONALE_INSTALL_DIR`: cambia el directorio del binario.
- `RATIONALE_SKIP_AGENT_CONFIG=1`: evita registrar el agente automáticamente.

## Privacidad

Rationale es local-first y no envía repositorios, prompts, Records ni secretos
por defecto. Aun así, `.rationale/` puede contener decisiones internas y debe
tratarse como parte del repositorio. Excluye `.env`, llaves, tokens, dumps y
datos personales según la política del equipo.

## Desinstalar sin perder decisiones

```bash
rationale uninstall-agent
rationale uninstall-agent --global-only
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-uninstall.sh | sh
```

La desinstalación elimina el binario y los bloques que Rationale escribió en la
configuración del agente, pero conserva `.rationale/`. Consulta
[`docs/runbooks/uninstall.md`](../runbooks/uninstall.md) antes de borrar el
canon manualmente.
