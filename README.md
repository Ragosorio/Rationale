# Rationale

Rationale es memoria causal local para agentes de programación. Antes de que un
agente cambie tu código, le entrega las reglas, decisiones y relaciones que
gobiernan ese código; después del cambio, el conocimiento que sigue siendo
cierto vuelve a tu repositorio — de forma automática, y con las reglas que fijas
fuera del alcance de cualquier agente.

> Git remembers what changed. Rationale remembers why it still matters.

## Elige tu recorrido

- **Solo quiero usarlo:** empieza por [Quickstart](docs/quickstart.md).
- **Quiero conectarlo a un agente:** sigue [Agentes y MCP](docs/user-guide/agents-and-mcp.md).
- **Quiero ver qué hacen mis agentes:** abre el [Control Room](docs/user-guide/control-room.md).
- **Quiero contribuir:** lee [CONTRIBUTING.md](CONTRIBUTING.md).
- **Quiero investigar una decisión:** consulta [Conceptos](docs/user-guide/concepts.md) y los [ADRs](docs/adr/).
- **Tengo un problema:** abre un issue siguiendo [SUPPORT.md](SUPPORT.md).

## Estado actual

`v1.0.0` es la primera Release estable. El ciclo completo — contexto antes del
cambio, captura autónoma después, autoridad humana sobre las reglas fijadas y el
Control Room en vivo — está implementado, probado y en uso sobre el propio
repositorio de Rationale. Qué se verificó y qué sigue abierto:
[`docs/work-items/v1.0-release-verification.md`](docs/work-items/v1.0-release-verification.md)
y [`CHANGELOG.md`](CHANGELOG.md).

## Instalación rápida

### macOS y Linux

```bash
curl -fsSL https://raw.githubusercontent.com/DeusData/codebase-memory-mcp/main/install.sh | bash   # recomendado
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-installer.sh | sh
rationale --version
```

### Windows PowerShell

```powershell
$installer = Join-Path $env:TEMP "rationale-installer.ps1"
Invoke-WebRequest https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-installer.ps1 -OutFile $installer
& $installer
rationale.exe --version
```

Después, desde la raíz del proyecto que quieres proteger:

```bash
rationale init
rationale health
rationale ui
```

El instalador verifica SHA-256, usa el canal `stable`, registra el servidor MCP
en los agentes detectados y nunca toca `.rationale/` al actualizar o
desinstalar. La guía completa está en [`docs/runbooks/install.md`](docs/runbooks/install.md).

## El ciclo en cinco pasos

1. **Localizar.** El agente encuentra el código con Codebase Memory.
2. **Preparar.** Llama a `prepare_change(target, intent)` y recibe un packet
   acotado: constraints y decisiones que gobiernan el target (con autoridad y
   procedencia), relaciones explicadas con su estado estructural, la vecindad
   del código y conflictos con su intención.
3. **Cambiar.** Hace el cambio mínimo coherente con ese contexto.
4. **Capturar.** Llama a `finalize_change` con solo el conocimiento durable.
   Rationale descarta ruido y duplicados y escribe el resto como Records
   canónicos en la misma llamada. No hay cola de aprobación.
5. **Fijar.** Tú fijas con `rationale pin` las reglas que ningún agente puede
   reemplazar. Si uno lo intenta, no se escribe nada: recibes un conflicto que
   resuelves con `rationale resolve`.

El recorrido guiado está en [`docs/quickstart.md`](docs/quickstart.md); el flujo
diario en [`docs/user-guide/daily-workflow.md`](docs/user-guide/daily-workflow.md).

## Cómo funciona

```text
Codebase Memory (dónde/cómo) ─┐
Git (qué/cuándo) ─────────────┼─> compilador de contexto ─> packet ─> agente
Canon .rationale (por qué) ───┘

agente cambia código ─> finalize_change ─> gate de captura ─> Records canónicos
                                               │
                              choque con una regla fijada ─> decisión humana

actividad y operaciones locales ─> rationale ui (solo lectura, 127.0.0.1)
```

Codebase Memory es un proveedor estructural opcional. Sin él, Rationale sigue
funcionando con cobertura degradada y lo informa; nunca lee la base interna del
proveedor ni lo trata como fuente de autoridad.

Rationale no es otro indexador de código, no reemplaza Git, no es un SaaS, no
guarda conversaciones, no usa embeddings remotos y no decide por sí solo el
significado de una afirmación.

## Comandos y MCP

| Necesidad | CLI | MCP |
|---|---|---|
| Inicializar | `rationale init` | — |
| Salud | `rationale health` | `health` |
| Preparar contexto | `rationale prepare <target>` | `prepare_change` |
| Explicar un target | — | `explain_target` |
| Capturar conocimiento | — | `finalize_change` |
| Ver el trabajo en vivo | `rationale ui` | — |
| Fijar / desfijar una regla | `rationale pin` / `unpin` | — |
| Decidir un conflicto | `rationale conflicts` / `resolve` | `resolve_conflict` (con la respuesta literal de la persona) |
| Lifecycle de un Record | `rationale review-record <id>` | — |
| Migrar propuestas pre-1.0 | `rationale migrate` | — |
| Integridad del canon | `rationale doctor` | — |
| Registrar / revertir agentes | `install-agent` / `uninstall-agent` | — |

Los agentes escriben memoria; las personas conservan la autoridad. Fijar,
desfijar y reemplazar una regla fijada exigen una terminal interactiva y un
actor declarado en `.rationale/config.yaml`.

## Documentos fundacionales

1. [`Rationale_v0.5.md`](Rationale_v0.5.md) — contrato de producto: problema,
   entidades, confianza y roadmap.
2. [`Rationale_Arquitectura_Conceptual_v0.1.md`](Rationale_Arquitectura_Conceptual_v0.1.md)
   — fronteras técnicas y decisiones de arquitectura.
3. [`Rationale_Proceso_Construccion_Agentes_v0.1.md`](Rationale_Proceso_Construccion_Agentes_v0.1.md)
   — proceso de trabajo, revisión cruzada y gates de calidad.

La 1.0 reemplazó la cola de aprobación de esos documentos por captura autónoma
con autoridad humana sobre las reglas fijadas; el detalle está en
[`docs/work-items/vnext-implementation-plan.md`](docs/work-items/vnext-implementation-plan.md).

## Datos, privacidad y archivos

| Capa | Ubicación | Git |
|---|---|---|
| Records, Subjects y configuración | `<proyecto>/.rationale/` | Sí |
| Actividad, operaciones y conflictos pendientes | `<proyecto>/.rationale-local/` | No (excluido automáticamente) |
| Cache SQLite de búsqueda | `~/.cache/rationale/projects/<id>/` | No |
| Binario | `~/.local/bin/rationale` o destino configurado | No |

Rationale es local-first: no sube código, prompts, Records ni secretos. La
actividad local guarda identificadores y una intención de una línea, nunca el
contenido de un Record (ADR-0017); `RATIONALE_ACTIVITY=off` la desactiva. Revisa
[`docs/user-guide/configuration.md`](docs/user-guide/configuration.md) antes de
usarlo en repositorios con datos sensibles.

## Documentación

El índice por audiencia está en [`docs/README.md`](docs/README.md). El sitio
publica la misma documentación en inglés y español.

- [Quickstart](docs/quickstart.md) — primera ejecución.
- [Guía de usuario](docs/user-guide/) — conceptos, flujo diario, Control Room, CLI, MCP y configuración.
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
- Avisos de terceros: [`THIRD_PARTY.md`](THIRD_PARTY.md).

## Licencia

MIT. Ver [`LICENSE`](LICENSE).
