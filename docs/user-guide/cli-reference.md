# Referencia de la CLI

La ayuda del binario es la referencia ejecutable:

```bash
rationale --help
```

## Comandos

| Comando | Uso |
|---|---|
| `init` | Crea la estructura `.rationale/` y ofrece integrar agentes detectados. |
| `health` | Reporta proyecto, revisión Git, estado del proveedor y cobertura. |
| `prepare <target>` | Compila un ContextPacket para un path o símbolo. |
| `serve` | Inicia el servidor MCP persistente. |
| `review` | Revisa propuestas pendientes con confirmación humana. |
| `review-record <id>` | Ejecuta lifecycle sobre un Record aprobado. |
| `install-agent` | Añade instrucciones y registro MCP de forma idempotente. |
| `uninstall-agent` | Revierte solamente lo escrito por `install-agent`. |
| `update` | Descarga e instala la última Release mediante el helper local. |

Opciones frecuentes:

```bash
rationale health --project-root /ruta/proyecto
rationale prepare "src/lib.rs::funcion" --project-root /ruta/proyecto
rationale install-agent --project-root /ruta/proyecto --dry-run
rationale update
```

La CLI no ofrece una vía automática para aprobar Records. Si una versión
publicada muestra comandos distintos, reporta el desvío antes de actualizar la
documentación.
