# Rationale

Rationale es un compilador local de contexto causal y una capa de procedencia, autoridad y vigencia para agentes de programación. Conserva por qué se hicieron cambios importantes, qué decisiones y restricciones gobiernan el comportamiento del sistema, quién podía aprobarlas y qué evidencia las respalda — y compila únicamente el contexto confiable, relevante y accionable que una tarea concreta necesita.

> Git remembers what changed. Rationale remembers why it still matters.

## Estado real del proyecto

**Pre-1.0, núcleo funcional con ciclo de captura completo.** El lenguaje del núcleo está decidido (Rust, `docs/adr/ADR-0001-core-language.md`) y ya existe un binario real en `src/` que implementa el modelo canónico completo (`Subject`, `Evidence`, `Assessment`, `Record`), una capa derivada local (SQLite + FTS, invalidada por revisión de Git), un Context Compiler con niveles de prioridad y presupuesto explícito, y una superficie MCP con cuatro herramientas: `prepare_change`, `explain_target`, `health` y `finalize_change`. `finalize_change` captura mecánicamente un cambio (diff, señales de alto valor, Subject candidato) y escribe una propuesta **pendiente** — nunca aprobada automáticamente; `rationale review` en la CLI es la única vía que la promueve a decisión real, con confirmación humana explícita.

**Ningún ADR está `accepted` todavía** — los 9 existentes siguen `proposed`, pendientes de revisión cruzada y aprobación humana (`docs/adr/index.md`, Subject `evaluation.no-self-certification`). No hay empaquetado ni distribución; se corre desde el código fuente con `cargo`.

## Instalación y uso

Requiere el toolchain de Rust (`rustc`/`cargo`) y, opcionalmente, [`codebase-memory-mcp`](docs/research/codebase-memory/) en el `PATH` como proveedor estructural — sin él, Rationale sigue funcionando pero con cobertura degradada (`provider_status: unavailable`), nunca bloquea.

```bash
cargo build --release
./target/release/rationale init                    # crea .rationale/ en el proyecto actual
./target/release/rationale health                   # revisión Git, proveedor, cobertura
./target/release/rationale prepare <path::symbol>    # packet de contexto para un target
./target/release/rationale serve                     # servidor MCP: prepare_change/explain_target/health/finalize_change
./target/release/rationale review                    # confirma propuestas pendientes, una a la vez
```

Para que un agente MCP (como esta misma sesión de Claude Code) pueda llamar al servidor, este repo ya trae [`.mcp.json`](.mcp.json) registrado — requiere reiniciar la sesión del agente para que lo cargue.

## Documentos fundacionales (leer en este orden)

1. [`Rationale_v0.5.md`](Rationale_v0.5.md) — contrato de producto: qué es, qué problema resuelve, modelo de entidades, modelo de confianza, roadmap.
2. [`Rationale_Arquitectura_Conceptual_v0.1.md`](Rationale_Arquitectura_Conceptual_v0.1.md) — contrato técnico: fronteras, componentes, qué está decidido y qué requiere investigación.
3. [`Rationale_Proceso_Construccion_Agentes_v0.1.md`](Rationale_Proceso_Construccion_Agentes_v0.1.md) — manual operativo para agentes que construyen el proyecto.

## Para agentes

Empezar por [`AGENTS.md`](AGENTS.md), no por estos tres documentos completos. `AGENTS.md` indica qué leer según el tipo de tarea, en qué fase está el proyecto ahora mismo, y qué NO hacer.

## Más documentación

- [`docs/architecture/code-map.md`](docs/architecture/code-map.md) — mapa de los módulos reales de `src/` y cómo fluyen `prepare`/`serve` de punta a punta.
- [`docs/runbooks/`](docs/runbooks/) — build, test, instalación, reseteo de cache, fallo de proveedor, diagnóstico, desinstalación.
- [`docs/adr/`](docs/adr/) — decisiones arquitectónicas con su evidencia, todas en estado `proposed`.

## Licencia

MIT. Ver [`LICENSE`](LICENSE).
