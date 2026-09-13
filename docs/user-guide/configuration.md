# Configuración, archivos y privacidad

## Estructura del proyecto

```text
.rationale/
├── config.yaml   # identidad del proyecto y autoridad declarada
├── records/      # Records canónicos
├── subjects/     # identidad conceptual
├── schemas/      # schemas locales
├── migrations/   # migraciones del formato
├── approvals/    # estructura reservada del canon
└── archive/      # propuestas anteriores a 1.0 archivadas por `migrate`

.rationale-local/            # nunca se versiona
├── activity/                # un NDJSON por sesión (ADR-0017)
├── operations/              # snapshots de cada prepare_change
├── conflicts/               # conflictos pendientes con Records fijados
└── installed-agent-files.json
```

Rationale añade `.rationale-local/` a `.git/info/exclude` antes de su primera
escritura. La cache SQLite de búsqueda vive en `~/.cache/rationale/` y se puede
regenerar.

## Autoridad declarada

```yaml
authority:
  "user:tu-nombre <tu-correo@example.com>":
    role: architecture-owner
```

El actor es tu identidad de Git. Quien no aparece aquí es contributor: puede
usar Rationale con normalidad, pero no fijar, desfijar ni adoptar un reemplazo
sobre una regla fijada.

## Variables de entorno

- `RATIONALE_PROVIDER=none`: desactiva el proveedor estructural.
- `RATIONALE_ACTIVITY=off`: desactiva la actividad local y los snapshots.
- `RATIONALE_NO_MASCOT=1`: silencia a Chestie.
- `RATIONALE_SKIP_AGENT_CONFIG=1`: evita configurar agentes en `init` y en el
  instalador.

Del instalador y de `rationale update`:

- `RATIONALE_CHANNEL`: `stable` (por defecto desde 1.0, `GET /releases/latest`)
  o `preview` (la Release más reciente, incluidas pre-releases).
- `RATIONALE_VERSION`: fija una versión concreta, por ejemplo para rollback.
- `RATIONALE_INSTALL_DIR`: cambia el directorio del binario.

## Privacidad

Rationale es local-first y no envía repositorios, prompts, Records ni secretos.
La actividad local guarda identificadores y la intención declarada (una línea,
280 caracteres como máximo), nunca contenido de Records, código ni
conversaciones; retiene 14 días, 500 sesiones y 200 operaciones. El Control Room
escucha solo en `127.0.0.1` y no escribe nada.

Aun así, `.rationale/` puede contener decisiones internas y debe tratarse como
parte del repositorio. Excluye `.env`, llaves, tokens, dumps y datos personales
según la política del equipo.

## Desinstalar sin perder decisiones

```bash
rationale uninstall-agent
rationale uninstall-agent --global-only
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-uninstall.sh | sh
```

La desinstalación elimina el binario y lo que Rationale escribió en la
configuración de los agentes, pero conserva `.rationale/`. Consulta
[`docs/runbooks/uninstall.md`](../runbooks/uninstall.md) antes de borrar el canon
manualmente.
