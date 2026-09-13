# Code map

Mapa factual de `src/` tal como existe en 1.0. No reinterpreta `Rationale_Arquitectura_Conceptual_v0.1.md` — describe el código real, con sus nombres reales.

## Módulos y responsabilidad

| Módulo | Responsabilidad | No hace |
|---|---|---|
| [`storage.rs`](../../src/storage.rs) | Canonical Store: lee y escribe `Record` (`.rationale/records/`; `.rationale/proposals/` solo para migrar propuestas anteriores a 1.0). Campos vNext tipados: `provenance.kind`, `authority`, `supersedes`, `relationship_bindings`. Escritura atómica (temp file + rename). Cada struct anidado (`BindingDeclaration`, `Approval`, `Evidence`, `Risk`, `RecordSubjectRef`) captura campos no modelados en `extra: yaml_serde::Mapping` — fidelidad de round-trip verificada. | No decide autoridad ni resuelve Subjects — eso es `subjects.rs`. |
| [`subjects.rs`](../../src/subjects.rs) | Lee `Subject` (`.rationale/subjects/`). `resolve_by_id_or_alias` (paso 1 de v0.5 §9.1) y `resolve()` (pasos 2-5: nombre normalizado, overlap de bindings, scope, similitud léxica). | No decide autoridad; nunca crea un Subject — solo sugiere `action`. |
| [`assessment.rs`](../../src/assessment.rs) | Calcula `Assessment` (epistemic/authority/applicability/linkage) a partir de un `Record` + revisión Git + estado del proveedor. Nunca persiste por sí mismo. | No lee ni escribe archivos directamente. |
| [`revision.rs`](../../src/revision.rs) | `GitSnapshot`/`Consistency` — la revisión SIEMPRE se deriva de Git, nunca del proveedor estructural (ADR-0006). | No consulta ningún proveedor. |
| [`project.rs`](../../src/project.rs) | Resuelve `path::symbol` a un `Target` dentro del proyecto, canonicalizando y rechazando path traversal. | No resuelve símbolos contra un proveedor — solo el path. |
| [`providers/`](../../src/providers/) | `CodeIntelligenceProvider` (trait) + `CodebaseMemoryClient` (implementación real vía MCP persistente, ADR-0002) + `ProviderHandle` (desacopla spawn de uso — la CLI spawnea por invocación, el servidor MCP una sola vez). | Nunca lee el storage interno del proveedor (`constraint.no-provider-internal-access`). |
| [`cache.rs`](../../src/cache.rs) | Capa derivada: SQLite (WAL), assessments cacheados invalidados por revisión exacta, FTS5 sobre statements. 100% regenerable desde `.rationale/`. | Nunca la única copia de una decisión (Arquitectura §11.7). |
| [`retrieval.rs`](../../src/retrieval.rs) | Context Compiler: `compile_packet` con niveles de prioridad (v0.5 §18.1), budget explícito, detección determinista de conflicto con la intención. | Nunca recorta constraints críticas por presupuesto; nunca usa embeddings. |
| [`capture.rs`](../../src/capture.rs) | Captura mecánica (v0.5 §15.1): `diff --name-status` normalizado, revisión final, working tree, cobertura del proveedor. Todo `epistemic_status: observed`. | Nunca infiere una decisión normativa — solo hechos verificables. |
| [`signals.rs`](../../src/signals.rs) | Señales de alto valor (v0.5 §15.4) y niveles de captura 0-3 (v0.5 §16) — determinista, sin LLM. Informan a `finalize_change` como hechos observados; nunca escriben memoria por sí mismas. | Nunca decide bloqueo; nunca asigna autoridad. |
| [`pipeline.rs`](../../src/pipeline.rs) | El pipeline puro compartido entre CLI y servidor MCP: `prepare`, `explain`, `health`, `finalize`. Sin `println!`/`eprintln!` — devuelve `diagnostics: Vec<String>` y deja que cada caller decida dónde escribirlos. | No imprime nada — ni a stdout ni a stderr. |
| [`review.rs`](../../src/review.rs) | `rationale review` confirma propuestas anteriores a 1.0 (legado) y `mutate_record` implementa el lifecycle humano de Records (corregir, disputar, revocar, superseder, autoridad y evidencia), siempre con eventos auditables y claim/TOCTOU. | No corre dentro del servidor MCP — necesita un humano interactivo. |
| [`prompts.rs`](../../src/prompts.rs) | Fuente única de las seis acciones pre-hechas y sustitución de argumentos para prompts MCP y skills de Claude Code. | No ejecuta herramientas ni aprueba Records; solo describe acciones. |
| [`agents.rs`](../../src/agents.rs) | Detecta agentes, registra `serve --client <agente>` por usuario (migrando la forma anterior), converge instrucciones y genera skills de Claude Code con escritura atómica y reversión por hash; retira skills de acciones retiradas solo si conservan su hash. | Nunca borra un skill editado por el usuario ni instala skills para Codex/Cursor. |
| [`mcp/framing.rs`](../../src/mcp/framing.rs) | Codecs JSON-RPC separados: stdio newline para el servidor de Rationale y `Content-Length` para el cliente hacia Codebase Memory (ADR-0007). Límites explícitos tras la revisión adversarial. | No interpreta el contenido del mensaje — solo lo enmarca. |
| [`mcp/server.rs`](../../src/mcp/server.rs) | Servidor MCP: cinco tools (`health`, `prepare_change`, `explain_target`, `finalize_change`, `resolve_conflict`) y seis prompts (`prompts/list`/`prompts/get`). Sesión de `ProviderHandle` persistente para toda la vida del proceso. `catch_unwind` normaliza panics de herramienta a `isError` sin tumbar la sesión. | stdout es EXCLUSIVAMENTE del protocolo — ver `Arquitectura §11.1`. |
| [`configuration.rs`](../../src/configuration.rs) | Localiza `.rationale/` subiendo directorios (como Git busca `.git/`) y carga `config.yaml`. | — |
| [`evaluation.rs`](../../src/evaluation.rs) | Timestamps RFC3339 (`now_iso8601`, `now_rfc3339_millis`) y lectura tolerante de valores legados `epoch:`. | No escribe logs: la actividad vive en `activity.rs`. |
| [`canon.rs`](../../src/canon.rs) | Canon autónomo (vNext): gate de candidatos, deduplicación, commit, supersesión explícita, conflictos con Records `pinned` y migración de propuestas pre-vNext. | Nunca otorga `pinned` por cuenta de un agente; nunca ordena SHAs de Git. |
| [`context.rs`](../../src/context.rs) | Vecindario estructural acotado del target y relaciones explicadas con su estado derivado; selección por rol con techo y claves derivadas por el núcleo. | Nunca vuelca el grafo completo. |
| [`relationships.rs`](../../src/relationships.rs) | Estado derivado de una `RelationshipBinding`: observed / indirect (camino compatible ≤3 saltos) / orphaned / unknown. | Nunca persiste el estado ni borra la explicación. |
| [`operations.rs`](../../src/operations.rs) | Operación de `prepare_change`: `operation_id`, snapshot local del subgrafo considerado y seleccionado, cierre por `finalize_change`. | No es canon: estado derivado y regenerable. |
| [`doctor.rs`](../../src/doctor.rs) | Integridad del canon: severidades y autoridades inválidas, Records sin bindings, `path_hint` rotos, Subjects colgantes, propuestas sin migrar. `--repair` pide confirmación por hallazgo. | Nunca repara sin confirmación humana. |
| [`ui/`](../../src/ui/) | `rationale ui`: servidor HTTP/1.1 solo con `std` en `127.0.0.1` (`http.rs`: GET/HEAD, lista de Hosts, límites, CSP), vistas REST y SSE sobre canon, operaciones y actividad (`api.rs`, `mod.rs`) y assets embebidos desde `ui/dist` por `build.rs` (`assets.rs`). El frontend vive en `ui/` (React + Three). | Nunca escribe estado ni sirve rutas del sistema de archivos. |
| [`activity.rs`](../../src/activity.rs) | Actividad local por sesión (`.rationale-local/activity/<session>.ndjson`, ADR-0017): `Recorder`, `Scope`, lectura combinada y `Tail` para el stream en vivo. Los payloads solo se arman con `activity::payload`. | Nunca registra código, statements ni rationale; nunca hace fallar una herramienta. |

## Flujo: `rationale prepare` (CLI)

```text
main.rs:cmd_prepare
  → pipeline::prepare (PrepareRequest, ProviderHandle, activity::Recorder)
      1. configuration::load
      2. storage::list_records
      3. project::resolve_target
      4. subjects::resolve_by_id_or_alias   (si el Record referencia un Subject)
      5. revision::snapshot + operations::Operation::new
                                            → actividad: context.requested, provider.started
      6. ProviderHandle → resolve_target    (sesión CBM, spawneada por cmd_prepare)
      7. revision::check_consistency
      8. cache::open / rebuild_fts / search_candidates / get_cached_assessment
      9. assessment::compute + cache::cache_assessment
      10. context::gather                   → actividad: target.resolved, relationship.*, provider.finished
      11. retrieval::compile                (packet vNext; el presupuesto es un techo)
      12. operations::save                  → actividad: packet.compiled
  ← PrepareOutcome { packet, assessment, diagnostics, latency_ms, operation, activity }
main.rs: diagnostics → stderr, packet → stdout, actividad: packet.delivered, session.ended
```

## Flujo: `rationale serve` (MCP)

```text
main.rs → mcp::server::run()
  ProviderHandle::spawn()   ← UNA sola vez para toda la vida del proceso
  loop { framing::read_message → despacho por method → framing::write_message }
    "initialize"   → Session::observe_initialize → actividad: agent.connected
    "prompts/list" → prompts::ACTIONS
    "prompts/get"  → prompts::render
    "tools/call" → handle_tools_call
      catch_unwind( match tool_name {
        "prepare_change"  → pipeline::prepare
        "explain_target"  → pipeline::explain
        "health"          → pipeline::health
        "finalize_change" → pipeline::finalize
        "resolve_conflict" → canon::resolve_conflict → actividad: conflict.resolved
      })
      → { content: [...], isError } — nunca deja escapar un panic
```

## Flujo: captura autónoma y autoridad humana (1.0)

```text
Agente hace un cambio real en el repo
  → MCP tools/call "finalize_change" { operation_id, summary, candidates: [...] }
      → pipeline::finalize
          1. operations::load                (base del diff: declarada, la de prepare, o HEAD)
          2. capture::capture                (diff mecánico, revisión final, cobertura)
          3. signals::*                      (hechos observados; informan, no escriben)
          4. canon::capture_candidates       (bajo CanonLock)
             por candidato: validación → durabilidad → ruido mecánico → bindings
                            → duplicados → supersedes explícito
             → descartado con motivo, o
             → Record canónico en .rationale/records/ (provenance: agent_asserted), o
             → si reemplaza un Record `pinned`: conflicto en .rationale-local/conflicts/ (no se escribe)
          5. relationships::assess_records   (estado de las relaciones que explican los Records nuevos)
  ← FinalizeOutcome { committed, discarded, conflicts, superseded, relationships, signals, capture }

Humano (CLI interactiva, nunca un agente por su cuenta)
  rationale pin <id> / unpin <id>         → autoridad declarada + confirmación → evento de lifecycle
  rationale conflicts
  rationale resolve <id> keep-pinned      → la regla fijada sigue gobernando
  rationale resolve <id> adopt-new        → exige autoridad declarada; el reemplazo hereda `pinned`
  (o MCP resolve_conflict con human_answer literal, que se guarda para auditoría)
```

## Frontera canónico vs derivado (Arquitectura §11.7)

```text
CANÓNICO (Git, versionado)          DERIVADO (local, regenerable)
.rationale/config.yaml               ~/.cache/rationale/projects/<id>/derived.sqlite3
.rationale/subjects/                   - assessments_cache (invalidado por revisión exacta)
.rationale/records/                    - records_fts (FTS5)
.rationale/archive/proposals/        .rationale-local/ (excluido de Git)
                                       - activity/<session>.ndjson
                                       - operations/, conflicts/
```

Borrar todo lo derivado nunca pierde una decisión — se reconstruye desde el canónico (verificado: `cache::tests::cache_rebuild_from_scratch_never_loses_canonical_data`).
