# Rationale

Rationale es un compilador local de contexto causal para agentes de
programación. Combina la memoria estructural del código con decisiones,
restricciones, autoridad y evidencia para que un agente sepa no solo **dónde**
está el código, sino también **por qué** debe seguir funcionando así.

> Git remembers what changed. Rationale remembers why it still matters.

## Elige tu recorrido

- **Solo quiero usarlo:** empieza por [Quickstart](docs/quickstart.md).
- **Quiero conectarlo a un agente:** sigue [Agentes y MCP](docs/user-guide/agents-and-mcp.md).
- **Quiero contribuir:** lee [CONTRIBUTING.md](CONTRIBUTING.md).
- **Quiero investigar una decisión:** consulta [Conceptos](docs/user-guide/concepts.md) y los [ADRs](docs/adr/).
- **Tengo un problema:** abre un issue siguiendo [SUPPORT.md](SUPPORT.md).

## Estado actual

La rama `release/v0.1.0-alpha.1` contiene el núcleo funcional y el ciclo
completo de captura y revisión. La Release pública verificable actual es
`v0.1.0-alpha.7`. Consulta los
gates y la historia en [`CHANGELOG.md`](CHANGELOG.md) y
[`docs/runbooks/release.md`](docs/runbooks/release.md).

El núcleo implementa `Subject`, `Evidence`, `Assessment` y `Record`, un store
canónico YAML, una capa derivada SQLite + FTS, el compilador de contexto y un
servidor MCP. Ningún agente aprueba decisiones automáticamente: MCP consulta y
prepara; la CLI interactiva es la frontera humana de aprobación y lifecycle.

## Instalación rápida

### macOS y Linux

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/Ragosorio/Rationale/releases/download/v0.1.0-alpha.7/rationale-installer.sh | sh
rationale --help
```

### Windows PowerShell

```powershell
$installer = Join-Path $env:TEMP "rationale-installer.ps1"
Invoke-WebRequest https://github.com/Ragosorio/Rationale/releases/download/v0.1.0-alpha.7/rationale-installer.ps1 -OutFile $installer
& $installer
rationale.exe --help
```

Después, desde la raíz del proyecto que quieres gobernar:

```bash
rationale init
rationale health
```

El instalador verifica SHA-256, instala el binario y conserva el canon
`.rationale/` al actualizar o desinstalar. La guía completa está en
[`docs/runbooks/install.md`](docs/runbooks/install.md).

## Primer flujo en cinco minutos

1. Instala Rationale y ejecuta `rationale init`.
2. Comprueba `rationale health`.
3. Configura el agente con `rationale install-agent --dry-run` y luego
   `rationale install-agent` si quieres aplicar la integración.
4. Antes de un cambio, el agente llama a `prepare_change` y recibe contexto
   relevante, restricciones, autoridad, vigencia y conflictos con la intención.
5. Después del cambio, el agente llama a `finalize_change`. Si el cambio tiene
   señales suficientes, se escribe una propuesta pendiente en
   `.rationale/proposals/`.
6. Una persona ejecuta `rationale review`, corrige, rechaza o aprueba la
   propuesta con confirmación explícita.
7. En cambios futuros, `prepare_change` puede recuperar el Record aprobado.

El recorrido guiado está en [`docs/quickstart.md`](docs/quickstart.md); el
flujo diario detallado está en [`docs/user-guide/daily-workflow.md`](docs/user-guide/daily-workflow.md).

## Cómo funciona

```text
Codebase Memory (dónde/cómo) ─┐
                              ├─> Context Compiler ─> packet para el agente
Canon .rationale (por qué) ───┘

agente cambia código ─> finalize_change ─> propuesta pendiente
                                              │
                                      review humana por CLI
                                              │
                                      Record versionado
```

Codebase Memory es un proveedor estructural opcional. Sin él, Rationale sigue
funcionando con cobertura degradada y lo informa mediante `health`; nunca lee
la base interna del proveedor ni lo trata como fuente de autoridad.

Rationale no es otro indexador de código, no reemplaza Git, no es un SaaS, no
guarda conversaciones, no usa embeddings remotos obligatorios y no decide por
sí solo que una afirmación es verdadera.

## Comandos y MCP

| Necesidad | CLI | MCP |
|---|---|---|
| Inicializar | `rationale init` | — |
| Salud y revisión | `rationale health` | `health` |
| Preparar contexto | `rationale prepare <target>` | `prepare_change` |
| Explicar un target | — | `explain_target` |
| Capturar una propuesta | — | `finalize_change` |
| Revisar propuestas | `rationale review` | — |
| Lifecycle de Records | `rationale review-record <id>` | — |
| Registrar/revertir agente | `install-agent` / `uninstall-agent` | — |

Las mutaciones humanas son deliberadamente interactivas. MCP no aprueba,
revoca, supersede ni cambia autoridad.

## Documentos fundacionales (leer en este orden)

1. [`Rationale_v0.5.md`](Rationale_v0.5.md) — contrato de producto: problema,
   entidades, confianza y roadmap.
2. [`Rationale_Arquitectura_Conceptual_v0.1.md`](Rationale_Arquitectura_Conceptual_v0.1.md)
   — fronteras técnicas y decisiones de arquitectura.
3. [`Rationale_Proceso_Construccion_Agentes_v0.1.md`](Rationale_Proceso_Construccion_Agentes_v0.1.md)
   — proceso de trabajo, revisión cruzada y gates de calidad.

## Datos, privacidad y archivos

| Capa | Ubicación | Git |
|---|---|---|
| Subjects, Records y propuestas | `<proyecto>/.rationale/` | Sí |
| Logs de ejecución | `<proyecto>/.rationale-local/` | No |
| SQLite/FTS derivado | `~/.cache/rationale/projects/<id>/` | No |
| Binario | `~/.local/bin/rationale` o destino configurado | No |

Rationale es local-first: no sube código, prompts, Records o secretos por
defecto. Revisa [`docs/user-guide/configuration.md`](docs/user-guide/configuration.md)
antes de usarlo en repositorios con datos sensibles.

## Documentación

El índice por audiencia está en [`docs/README.md`](docs/README.md).

- [Quickstart](docs/quickstart.md) — primera ejecución.
- [Guía de usuario](docs/user-guide/) — conceptos, flujo diario, CLI, MCP y configuración.
- [Runbooks](docs/runbooks/) — instalación, diagnóstico, proveedor, cache y release.
- [Arquitectura factual](docs/architecture/code-map.md) — módulos y flujos reales.
- [ADRs](docs/adr/) — decisiones y estado de aprobación.
- [Seguridad](docs/security/) — límites y baseline.
- [Investigación Codebase Memory](docs/research/codebase-memory/) — integración y límites.

## Contribuir y obtener ayuda

- Contribuciones: [`CONTRIBUTING.md`](CONTRIBUTING.md).
- Vulnerabilidades: [`SECURITY.md`](SECURITY.md).
- Soporte y bugs: [`SUPPORT.md`](SUPPORT.md).
- Conducta comunitaria: [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md).
- Historial de cambios: [`CHANGELOG.md`](CHANGELOG.md).

## Licencia

MIT. Ver [`LICENSE`](LICENSE).
