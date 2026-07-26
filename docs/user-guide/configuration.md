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
- `RATIONALE_INSTALL_DIR`: cambia el directorio del binario.
- `RATIONALE_SKIP_AGENT_CONFIG=1`: evita registrar el agente automáticamente.
- `RATIONALE_REMOVE_AGENT_CONFIG=1`: permite quitar la entrada global de Codex
  durante la desinstalación.

## Privacidad

Rationale es local-first y no envía repositorios, prompts, Records ni secretos
por defecto. Aun así, `.rationale/` puede contener decisiones internas y debe
tratarse como parte del repositorio. Excluye `.env`, llaves, tokens, dumps y
datos personales según la política del equipo.

## Desinstalar sin perder decisiones

```bash
rationale uninstall-agent
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-uninstall.sh | sh
```

La desinstalación elimina el binario y los bloques que Rationale escribió en la
configuración del agente, pero conserva `.rationale/`. Consulta
[`docs/runbooks/uninstall.md`](../runbooks/uninstall.md) antes de borrar el
canon manualmente.
