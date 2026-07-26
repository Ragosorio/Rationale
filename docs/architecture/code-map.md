# Code map

Mapa factual de `src/` tal como existe tras Fase F. No reinterpreta `Rationale_Arquitectura_Conceptual_v0.1.md` — describe el código real, con sus nombres reales.

## Módulos y responsabilidad

| Módulo | Responsabilidad | No hace |
|---|---|---|
| [`storage.rs`](../../src/storage.rs) | Canonical Store: lee y escribe `Record` (`.rationale/records/`, `.rationale/proposals/`). Escritura atómica (temp file + rename). Cada struct anidado (`BindingDeclaration`, `Approval`, `Evidence`, `Risk`, `RecordSubjectRef`) captura campos no modelados en `extra: yaml_serde::Mapping` — fidelidad de round-trip verificada. | No decide autoridad ni resuelve Subjects — eso es `subjects.rs`. |
| [`subjects.rs`](../../src/subjects.rs) | Lee `Subject` (`.rationale/subjects/`). `resolve_by_id_or_alias` (paso 1 de v0.5 §9.1) y `resolve()` (pasos 2-5: nombre normalizado, overlap de bindings, scope, similitud léxica). | No decide autoridad; nunca crea un Subject — solo sugiere `action`. |
| [`assessment.rs`](../../src/assessment.rs) | Calcula `Assessment` (epistemic/authority/applicability/linkage) a partir de un `Record` + revisión Git + estado del proveedor. Nunca persiste por sí mismo. | No lee ni escribe archivos directamente. |
| [`revision.rs`](../../src/revision.rs) | `GitSnapshot`/`Consistency` — la revisión SIEMPRE se deriva de Git, nunca del proveedor estructural (ADR-0006). | No consulta ningún proveedor. |
| [`project.rs`](../../src/project.rs) | Resuelve `path::symbol` a un `Target` dentro del proyecto, canonicalizando y rechazando path traversal. | No resuelve símbolos contra un proveedor — solo el path. |
| [`providers/`](../../src/providers/) | `CodeIntelligenceProvider` (trait) + `CodebaseMemoryClient` (implementación real vía MCP persistente, ADR-0002) + `ProviderHandle` (desacopla spawn de uso — la CLI spawnea por invocación, el servidor MCP una sola vez). | Nunca lee el storage interno del proveedor (`constraint.no-provider-internal-access`). |
| [`cache.rs`](../../src/cache.rs) | Capa derivada: SQLite (WAL), assessments cacheados invalidados por revisión exacta, FTS5 sobre statements. 100% regenerable desde `.rationale/`. | Nunca la única copia de una decisión (Arquitectura §11.7). |
| [`retrieval.rs`](../../src/retrieval.rs) | Context Compiler: `compile_packet` con niveles de prioridad (v0.5 §18.1), budget explícito, detección determinista de conflicto con la intención. | Nunca recorta constraints críticas por presupuesto; nunca usa embeddings. |
| [`capture.rs`](../../src/capture.rs) | Captura mecánica (v0.5 §15.1): `diff --name-status` normalizado, revisión final, working tree, cobertura del proveedor. Todo `epistemic_status: observed`. | Nunca infiere una decisión normativa — solo hechos verificables. |
| [`signals.rs`](../../src/signals.rs) | Señales de alto valor (v0.5 §15.4) y niveles de captura 0-3 (v0.5 §16) — determinista, sin LLM. `CaptureLevel` no tiene variante para Nivel 4 (Critical invariant): estructuralmente inalcanzable fuera de `rationale review`. | Nunca decide bloqueo; nunca asigna autoridad. |
| [`pipeline.rs`](../../src/pipeline.rs) | El pipeline puro compartido entre CLI y servidor MCP: `prepare`, `explain`, `health`, `finalize`. Sin `println!`/`eprintln!` — devuelve `diagnostics: Vec<String>` y deja que cada caller decida dónde escribirlos. | No imprime nada — ni a stdout ni a stderr. |
| [`review.rs`](../../src/review.rs) | `rationale review` confirma propuestas y `mutate_record` implementa el lifecycle humano de Records (corregir, disputar, revocar, superseder, autoridad y evidencia), siempre con eventos auditables y claim/TOCTOU. | No corre dentro del servidor MCP — necesita un humano interactivo. |
| [`mcp/framing.rs`](../../src/mcp/framing.rs) | Framing JSON-RPC `Content-Length` (ADR-0007), compartido por cliente y servidor. Límites explícitos de tamaño (header y body) tras la revisión adversarial de Fase E. | No interpreta el contenido del mensaje — solo lo enmarca. |
| [`mcp/server.rs`](../../src/mcp/server.rs) | Servidor MCP: `prepare_change`, `explain_target`, `health`, `finalize_change`. Sesión de `ProviderHandle` persistente para toda la vida del proceso. `catch_unwind` normaliza panics de herramienta a `isError` sin tumbar la sesión. | stdout es EXCLUSIVAMENTE del protocolo — ver `Arquitectura §11.1`. |
| [`configuration.rs`](../../src/configuration.rs) | Localiza `.rationale/` subiendo directorios (como Git busca `.git/`) y carga `config.yaml`. | — |
| [`evaluation.rs`](../../src/evaluation.rs) | Instrumentación local (`.rationale-local/runs/*.ndjson`) — nunca se envía a ningún servicio. | — |

## Flujo: `rationale prepare` (CLI)

```text
main.rs:cmd_prepare
  → pipeline::prepare (PrepareRequest)
      1. configuration::load
      2. storage::list_records
      3. project::resolve_target
      4. subjects::resolve_by_id_or_alias   (si el Record referencia un Subject)
      5. ProviderHandle → resolve_target    (sesión CBM, spawneada por cmd_prepare)
      6. revision::snapshot + check_consistency
      7. cache::open / rebuild_fts / search_candidates / get_cached_assessment
      8. assessment::compute
      9. cache::cache_assessment
      10. retrieval::compile_packet
  ← PrepareOutcome { packet, assessment, diagnostics, latency_ms }
main.rs: diagnostics → stderr, packet → stdout, evaluation::record_run
```

## Flujo: `rationale serve` (MCP)

```text
main.rs → mcp::server::run()
  ProviderHandle::spawn()   ← UNA sola vez para toda la vida del proceso
  loop { framing::read_message → despacho por method → framing::write_message }
    "tools/call" → handle_tools_call
      catch_unwind( match tool_name {
        "prepare_change"  → pipeline::prepare
        "explain_target"  → pipeline::explain
        "health"          → pipeline::health
        "finalize_change" → pipeline::finalize
      })
      → { content: [...], isError } — nunca deja escapar un panic
```

## Flujo: captura → propuesta → aprobación (Fase F, el ciclo completo)

```text
Agente hace un cambio real en el repo
  → MCP tools/call "finalize_change" { target, base_revision, intent, statement, record_id, subject_id/title, ... }
      → pipeline::finalize
          1. capture::capture           (diff mecánico, revisión final, cobertura)
          2. signals::signals_from_paths/text + determine_level
             → si Nivel 0 (GitOnly): NO se escribe nada, se reporta y termina
          3. subjects::resolve           (contra el canon real de Subjects)
             → si candidato fuerte sin novelty_reason: SE BLOQUEA, no escribe nada
          4. Construye el Record propuesto (approvals: [] siempre)
          5. storage::write_record       → .rationale/proposals/<record_id>.yaml
  ← FinalizeOutcome { level, signals, capture, subject_resolution, proposal_written, ... }

Humano corre `rationale review` (CLI, nunca el servidor MCP)
  → review::list_pending(.rationale/proposals/)
  → por cada propuesta: review::describe_effect (una pantalla, no el YAML)
     entrada del humano:
       "approve" / "approve-critical" → review::approve → .rationale/records/<id>.yaml (con Approval real)
       "c" (corregir) → nuevo statement → confirmar con la misma palabra → approve
       "r" → review::reject → .rationale/proposals/.rejected/<id>.yaml (nunca se borra)
       cualquier otra cosa → se salta, sigue pendiente
  → review::log_decision → .rationale-local/runs/review-decisions.ndjson
```

## Frontera canónico vs derivado (Arquitectura §11.7)

```text
CANÓNICO (Git, versionado)          DERIVADO (local, regenerable)
.rationale/subjects/                 ~/.cache/rationale/projects/<id>/derived.sqlite3
.rationale/records/                    - assessments_cache (invalidado por revisión exacta)
.rationale/proposals/                  - records_fts (FTS5)
.rationale/approvals/ (embebido en Record.approvals)
.rationale/bindings/  (embebido en Record.binding_declarations)
```

Borrar todo lo derivado nunca pierde una decisión — se reconstruye desde el canónico (verificado: `cache::tests::cache_rebuild_from_scratch_never_loses_canonical_data`).
