//! Servidor MCP de Rationale — Fase E5 (ADR-0007).
//!
//! Bucle síncrono sobre stdin/stdout con el transporte stdio newline de
//! `mcp::framing`, sin
//! runtime async (ADR-0007 §Decision). Mantiene **una sola** sesión de
//! `ProviderHandle` viva durante toda la vida del proceso — la amortización
//! real que la CLI de un solo disparo no puede ofrecer. Este es exactamente
//! el gap que la revisión adversarial encontró en ADR-0002
//! (`docs/work-items/adversarial-review-adr-0001-0002-0006.md`).
//!
//! Regla no negociable (`Arquitectura §11.1`): stdout es EXCLUSIVAMENTE del
//! protocolo MCP. Todo diagnóstico va a stderr; el pipeline nunca imprime
//! por sí mismo (sus `eprintln!` viejos ahora son `diagnostics` que este
//! servidor reenvía a stderr, nunca a stdout).

use crate::mcp::framing;
use crate::providers::{Coverage, ProviderHandle, ProviderStatus};
use crate::{activity, canon, configuration, context, pipeline, retrieval};
use serde_json::{json, Value};
use std::io::{self, BufReader};
use std::path::PathBuf;

const PROTOCOL_VERSION: &str = "2024-11-05";

/// Estado de la sesión MCP: quién es el cliente y qué sesión de actividad
/// representa este proceso.
pub struct Session {
    pub actor: canon::ActorContext,
    /// Actividad local de esta sesión (ADR-0017).
    pub recorder: activity::Recorder,
}

impl Session {
    pub fn new(client_flag: Option<String>) -> Self {
        let (client, client_source) = match client_flag.as_deref().and_then(normalize_client) {
            Some(client) => (client, "flag"),
            None => ("unknown".to_string(), "unknown"),
        };
        let session_id = canon::generate_id("session");
        Session {
            recorder: activity::Recorder::new(Some(&session_id)),
            actor: canon::ActorContext {
                client,
                client_source: client_source.to_string(),
                session_id: Some(session_id),
                operation_id: None,
            },
        }
    }

    /// El flag `--client` (escrito por `install-agent`) gana. Sin flag, el
    /// nombre que el propio cliente declara en `initialize.clientInfo` es un
    /// dato reportado — se registra con su fuente, nunca como certeza.
    fn observe_initialize(&mut self, params: &Value) {
        if self.actor.client_source == "flag" {
            return;
        }
        let reported = params
            .get("clientInfo")
            .and_then(|info| info.get("name"))
            .and_then(Value::as_str)
            .and_then(normalize_client);
        if let Some(client) = reported {
            self.actor.client = client;
            self.actor.client_source = "mcp-client-info".to_string();
        }
    }
}

/// Nombres de cliente normalizados. Los conocidos se reducen a su nombre de
/// producto (`codex-mcp-client` → `codex`); cualquier otro se conserva si es
/// un identificador simple. Texto arbitrario nunca llega a logs ni a la UI.
pub fn normalize_client(raw: &str) -> Option<String> {
    let lower = raw.trim().to_ascii_lowercase();
    let known = if lower.starts_with("claude") {
        Some("claude-code")
    } else if lower.starts_with("codex") {
        Some("codex")
    } else if lower.starts_with("cursor") {
        Some("cursor")
    } else {
        None
    };
    if let Some(known) = known {
        return Some(known.to_string());
    }
    let simple = !lower.is_empty()
        && lower.len() <= 40
        && lower
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
    simple.then_some(lower)
}

pub fn run(client_flag: Option<String>) {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();
    let mut session = Session::new(client_flag);

    // El proveedor se inicializa de forma lazy. El handshake MCP y
    // `tools/list` no deben depender de que Codebase Memory esté instalado,
    // indexado o responda dentro de su propio deadline.
    let mut provider = None;

    loop {
        let msg = match framing::read_stdio_message(&mut reader) {
            framing::Frame::Message(msg) => msg,
            // Fin real de la sesión — el cliente cerró stdin.
            framing::Frame::Eof => break,
            // E7 hallazgo B: antes esto era indistinguible de EOF y
            // terminaba la sesión completa en silencio ante un mensaje
            // simplemente malformado. Ahora se responde con el error
            // JSON-RPC estándar de parseo y la sesión persistente —el
            // activo que Fase E5 existe para amortizar— sigue viva.
            framing::Frame::Invalid(reason) => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": {"code": -32700, "message": format!("parse error: {reason}")}
                });
                let _ = framing::write_stdio_message(&mut writer, &resp);
                continue;
            }
        };

        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = msg.get("id").cloned();

        match method {
            "initialize" => {
                if let Some(params) = msg.get("params") {
                    session.observe_initialize(params);
                }
                // La conexión se registra en el proyecto del cwd, si existe;
                // cualquier otro proyecto que la sesión toque la recibe al
                // encabezar su archivo.
                let protocol = msg
                    .get("params")
                    .and_then(|params| params.get("protocolVersion"))
                    .and_then(Value::as_str);
                let scope = std::env::current_dir()
                    .ok()
                    .and_then(|cwd| activity::Scope::for_project(&cwd, &session.actor));
                session.recorder.connected(
                    scope.as_ref(),
                    activity::payload::agent_connected(&session.actor, protocol),
                );
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": PROTOCOL_VERSION,
                        "capabilities": {"tools": {}, "prompts": {}},
                        "serverInfo": {"name": "rationale", "version": env!("RATIONALE_BUILD_VERSION")}
                    }
                });
                let _ = framing::write_stdio_message(&mut writer, &resp);
            }
            "notifications/initialized" => {
                // Sin respuesta — es una notificación.
            }
            "tools/list" => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {"tools": tool_definitions()}
                });
                let _ = framing::write_stdio_message(&mut writer, &resp);
            }
            "tools/call" => {
                let resp = handle_tools_call(&msg, id, &mut provider, &session);
                let _ = framing::write_stdio_message(&mut writer, &resp);
            }
            "prompts/list" => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {"prompts": prompt_definitions()}
                });
                let _ = framing::write_stdio_message(&mut writer, &resp);
            }
            "prompts/get" => {
                let resp = handle_prompts_get(&msg, id);
                let _ = framing::write_stdio_message(&mut writer, &resp);
            }
            _ => {
                if let Some(id) = id {
                    let resp = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {"code": -32601, "message": "method not found"}
                    });
                    let _ = framing::write_stdio_message(&mut writer, &resp);
                }
            }
        }
    }
    session.recorder.end(&session.actor);
}

fn prompt_definitions() -> Vec<Value> {
    crate::prompts::ACTIONS
        .iter()
        .map(|action| {
            let arguments: Vec<Value> = action
                .arguments
                .iter()
                .map(|name| {
                    json!({
                        "name": name,
                        "required": !(action.name == "capture" && *name == "statement")
                    })
                })
                .collect();
            json!({
                "name": action.name,
                "description": action.description,
                "arguments": arguments
            })
        })
        .collect()
}

fn handle_prompts_get(msg: &Value, id: Option<Value>) -> Value {
    let params = msg.get("params").cloned().unwrap_or_else(|| json!({}));
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let Some(action) = crate::prompts::action(name) else {
        // Un cliente que actualizó Rationale puede seguir pidiendo un prompt
        // por su nombre anterior: se le dice qué lo reemplaza.
        let message = match crate::prompts::retired(name) {
            Some(retired) => format!("prompt '{name}' was retired: {}", retired.replacement),
            None => format!("unknown prompt: '{name}'"),
        };
        return json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": -32602, "message": message}
        });
    };
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    match crate::prompts::render(action, &arguments) {
        Ok(text) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "description": action.description,
                "messages": [{
                    "role": "user",
                    "content": {"type": "text", "text": text}
                }]
            }
        }),
        Err(message) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": -32602, "message": message}
        }),
    }
}

/// Cada herramienta en su propio `json!`: un único literal con las cinco
/// supera el límite de recursión del macro.
fn tool_definitions() -> Value {
    Value::Array(vec![
        json!({
            "name": "prepare_change",
            "description": "Call before a non-trivial change. Opens an operation and returns the minimal sufficient context: the constraints and decisions that govern the target with their authority and provenance, relationships explained by Records with their structural state (observed/indirect/orphaned/unknown) and why they exist, the relevant subgraph, the target's code, risks, unknowns, and coverage. Keep the operation_id and pass it to finalize_change. When Records govern the target or conflict with the intent, state whether the intent respects, contradicts, or leaves each one undetermined before editing.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "target": {"type": "string", "description": "path or path::symbol inside the project, e.g. src/main.rs::cmd_prepare"},
                    "intent": {"type": "string", "description": "The change you actually intend. When present, intent-conflict detection runs automatically"},
                    "mode": {"type": "string", "enum": ["baseline", "intent-aware"], "description": "Usually unnecessary: the mode follows from whether 'intent' is present. Pass 'baseline' to force plain retrieval even with an intent."},
                    "project_root": {"type": "string", "description": "Default: the Rationale project that contains the server's working directory"},
                    "repo_path": {"type": "string"},
                    "max_tokens": {"type": "integer"},
                    "max_critical_constraints": {"type": "integer"},
                    "max_risks": {"type": "integer"},
                    "max_nodes": {"type": "integer", "description": "Ceiling for structural nodes in the packet (default 12)"},
                    "max_relationships": {"type": "integer", "description": "Ceiling for structural relationships in the packet (default 16); relationships explained by the canon are never cut"}
                },
                "required": ["target"]
            }
        }),
        json!({
            "name": "explain_target",
            "description": "Explains why a target exists: the Records that govern it by exact binding, its Subject, and what is known versus unknown. Call it before simplifying or removing code that looks redundant or odd.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "target": {"type": "string"},
                    "project_root": {"type": "string"},
                    "repo_path": {"type": "string"}
                },
                "required": ["target"]
            }
        }),
        json!({
            "name": "health",
            "description": "Git revision, working tree, structural provider status and coverage, and what Rationale does not know.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_root": {"type": "string"}
                }
            }
        }),
        json!({
            "name": "finalize_change",
            "description": "Closes a change. Send in `candidates` only knowledge that will stay true after the change: why the code is the way it is and what must be preserved. Rationale discards noise and duplicates with a reason and writes the rest as canonical Records in the same call; there is no approval queue. Without candidates, no memory is written. If a candidate tries to replace a pinned Record, the result contains `conflicts`: ask the person which statement should govern, then call `resolve_conflict` with their answer.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "operation_id": {"type": "string", "description": "The operation_id returned by prepare_change, if any"},
                    "summary": {"type": "string", "description": "What changed, in one or two sentences. Reported, never stored as memory"},
                    "target": {"type": "string", "description": "path::symbol of the main target (diagnostics only)"},
                    "base_revision": {"type": "string", "description": "Git revision to capture the diff from. Default: the HEAD prepare_change saw for this operation, or HEAD without an operation"},
                    "candidates": {
                        "type": "array",
                        "description": "Durable knowledge. One Record per decision: split when the parts could be replaced or revoked separately.",
                        "items": {
                            "type": "object",
                            "required": ["kind", "statement", "rationale", "durability", "bindings"],
                            "properties": {
                                "kind": {"type": "string", "enum": ["constraint", "decision", "risk", "exception"]},
                                "statement": {"type": "string", "description": "The assertion that must stay true"},
                                "rationale": {"type": "string", "description": "Why: the cause, not a restatement of the statement"},
                                "durability": {"type": "string", "enum": ["durable", "transient"], "description": "'durable' only if it stays true after this change"},
                                "bindings": {"type": "array", "items": {"type": "string"}, "description": "Code the Record governs: 'src/x.rs' or 'src/x.rs::symbol'"},
                                "relationships": {"type": "array", "description": "Relationships this Record explains, as path::symbol endpoints; kind is calls, uses, writes, imports, defines, implements, tests, configures, depends_on, http_calls, contains, or decorates. A resolved relationship also anchors the Record", "items": {"type": "object", "required": ["source", "kind", "target"], "properties": {"source": {"type": "string"}, "kind": {"type": "string"}, "target": {"type": "string"}}}},
                                "supersedes": {"type": "array", "items": {"type": "string"}, "description": "ids of active Records this one explicitly replaces"},
                                "severity": {"type": "string", "enum": ["critical", "high", "medium", "low"]},
                                "id": {"type": "string", "description": "Optional: '<kind>.<slug>'; generated when absent"},
                                "risks": {"type": "array", "items": {"type": "string"}},
                                "evidence": {"type": "array", "items": {"type": "object", "required": ["path"], "properties": {"path": {"type": "string"}, "type": {"type": "string"}, "note": {"type": "string"}}}},
                                "subject": {"type": "object", "required": ["id", "title"], "properties": {"id": {"type": "string"}, "title": {"type": "string"}, "type": {"type": "string"}}}
                            }
                        }
                    },
                    "project_root": {"type": "string"},
                    "repo_path": {"type": "string"}
                }
            }
        }),
        json!({
            "name": "resolve_conflict",
            "description": "Applies a person's decision on a conflict returned by finalize_change. Use it only after asking the person which statement should govern; never decide for them. keep_pinned keeps the pinned Record; adopt_new replaces it and requires authority declared in .rationale/config.yaml.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "conflict_id": {"type": "string"},
                    "decision": {"type": "string", "enum": ["keep_pinned", "adopt_new"]},
                    "human_answer": {"type": "string", "description": "The person's literal answer, kept for audit"},
                    "project_root": {"type": "string"},
                    "repo_path": {"type": "string"}
                },
                "required": ["conflict_id", "decision", "human_answer"]
            }
        }),
    ])
}

fn handle_tools_call(
    msg: &Value,
    id: Option<Value>,
    provider: &mut Option<ProviderHandle>,
    session: &Session,
) -> Value {
    let params = msg.get("params").cloned().unwrap_or_else(|| json!({}));
    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_string();
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    // Un panic dentro de una herramienta (p.ej. `.expect()` sobre un
    // proyecto sin Records) nunca debe tumbar la sesión completa — se
    // normaliza a `isError` y el servidor sigue vivo para la llamada
    // siguiente (regla no negociable de E5).
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match name.as_str() {
        "prepare_change" => call_prepare_change(&arguments, provider_mut(provider), session),
        "explain_target" => call_explain_target(&arguments),
        "health" => call_health(&arguments, provider_mut(provider)),
        "finalize_change" => call_finalize_change(&arguments, provider_mut(provider), session),
        "resolve_conflict" => call_resolve_conflict(&arguments, session),
        other => Err(format!("herramienta desconocida: '{other}'")),
    }));

    match outcome {
        Ok(Ok(value)) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [{"type": "text", "text": serde_json::to_string(&value).unwrap_or_default()}],
                "isError": false
            }
        }),
        Ok(Err(message)) => error_result(id, &message),
        Err(_) => error_result(
            id,
            "error interno inesperado al ejecutar la herramienta — la sesión sigue viva",
        ),
    }
}

fn provider_mut(provider: &mut Option<ProviderHandle>) -> &mut ProviderHandle {
    provider.get_or_insert_with(ProviderHandle::spawn)
}

fn error_result(id: Option<Value>, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [{"type": "text", "text": message}],
            "isError": true
        }
    })
}

fn resolve_roots(args: &Value) -> Result<(PathBuf, PathBuf), String> {
    let project_root = match args.get("project_root").and_then(|v| v.as_str()) {
        Some(p) => PathBuf::from(p),
        None => default_project_root()?,
    };
    let repo_path = match args.get("repo_path").and_then(|v| v.as_str()) {
        Some(p) => PathBuf::from(p),
        None => project_root.clone(),
    };
    Ok((project_root, repo_path))
}

fn default_project_root() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("no se pudo determinar cwd: {e}"))?;
    configuration::find_project_root(&cwd)
        .ok_or_else(|| "no se encontró .rationale/; pasa 'project_root' explícito".to_string())
}

fn provider_status_label(status: &ProviderStatus) -> &'static str {
    match status {
        ProviderStatus::Successful => "successful",
        ProviderStatus::Degraded => "degraded",
        ProviderStatus::Unavailable => "unavailable",
    }
}

fn coverage_label(coverage: &Coverage) -> &'static str {
    match coverage {
        Coverage::Complete => "complete",
        Coverage::Partial => "partial",
        Coverage::Unknown => "unknown",
    }
}

fn call_prepare_change(
    args: &Value,
    provider: &mut ProviderHandle,
    session: &Session,
) -> Result<Value, String> {
    let target = args
        .get("target")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "falta el argumento requerido 'target'".to_string())?
        .to_string();
    let intent = args
        .get("intent")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let mode = args.get("mode").and_then(|v| v.as_str());
    let (project_root, repo_path) = resolve_roots(args)?;

    // v0.5 §4.18 define el modo por la presencia de intencion ("baseline:
    // intencion ausente o incompleta" / "intent-aware: target + intencion"),
    // no por un flag separado que el caller deba recordar ademas de `intent`.
    // El prompt maestro documentado (docs/prompt-master.md) solo ensena
    // `prepare_change(target, intent)` -- nunca `mode` -- asi que exigir
    // `mode: "intent-aware"` aparte descartaba la intencion en silencio para
    // todo caller que siguiera el protocolo oficial. `mode: "baseline"`
    // explicito sigue siendo la manera de forzar retrieval puro sin
    // deteccion de conflictos, aunque venga `intent`.
    let effective_intent = if mode == Some("baseline") {
        None
    } else {
        intent
    };

    let default_budget = retrieval::Budget::default();
    let budget = retrieval::Budget {
        max_tokens: args
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(default_budget.max_tokens),
        max_critical_constraints: args
            .get("max_critical_constraints")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(default_budget.max_critical_constraints),
        max_risks: args
            .get("max_risks")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(default_budget.max_risks),
    };

    let structural_default = context::StructuralBudget::default();
    let arg_usize = |field: &str| args.get(field).and_then(Value::as_u64).map(|n| n as usize);
    let structural_budget = context::StructuralBudget {
        max_nodes: arg_usize("max_nodes").unwrap_or(structural_default.max_nodes),
        max_relationships: arg_usize("max_relationships")
            .unwrap_or(structural_default.max_relationships),
        snippet_chars: structural_default.snippet_chars,
    };

    let outcome = pipeline::prepare(
        &pipeline::PrepareRequest {
            target_spec: target,
            intent: effective_intent,
            project_root,
            repo_path,
            budget,
            structural_budget,
            actor: session.actor.clone(),
        },
        provider,
        &session.recorder,
    )?;

    let response = json!({
        "operation_id": outcome.operation.operation_id,
        "packet": outcome.packet,
        "assessment": outcome.assessment,
        "diagnostics": outcome.diagnostics,
        "latency_ms": outcome.latency_ms,
    });
    session.recorder.emit(
        &outcome.activity,
        "packet.delivered",
        activity::payload::packet_delivered(
            outcome.latency_ms,
            outcome.operation.selection.packet_bytes,
        ),
    );
    Ok(response)
}

fn call_explain_target(args: &Value) -> Result<Value, String> {
    let target = args
        .get("target")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "falta el argumento requerido 'target'".to_string())?;
    let (project_root, repo_path) = resolve_roots(args)?;

    let outcome = pipeline::explain(target, &project_root, &repo_path)?;

    Ok(json!({
        "resolved_target": outcome.resolved_target,
        "governing_records": outcome.governing_records,
        "subject": outcome.subject,
        "known": outcome.known,
        "unknown": outcome.unknown,
        "diagnostics": outcome.diagnostics,
    }))
}

fn call_health(args: &Value, provider: &mut ProviderHandle) -> Result<Value, String> {
    let project_root = match args.get("project_root").and_then(|v| v.as_str()) {
        Some(p) => PathBuf::from(p),
        None => default_project_root()?,
    };

    let outcome = pipeline::health(&project_root, provider)?;

    Ok(json!({
        "project_id": outcome.project_id,
        "project_root": outcome.project_root.display().to_string(),
        "git_revision": outcome.git_revision,
        "working_tree_dirty": outcome.working_tree_dirty,
        "provider_status": provider_status_label(&outcome.provider_status),
        "provider_coverage": coverage_label(&outcome.provider_coverage),
        "provider_error": outcome.provider_error,
    }))
}

fn call_finalize_change(
    args: &Value,
    provider: &mut ProviderHandle,
    session: &Session,
) -> Result<Value, String> {
    let optional_str = |field: &str| -> Option<String> {
        args.get(field)
            .and_then(Value::as_str)
            .map(str::to_string)
            .filter(|s| !s.trim().is_empty())
    };
    let candidates: Vec<Result<canon::Candidate, String>> = match args.get("candidates") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                serde_json::from_value::<canon::Candidate>(item.clone()).map_err(|e| e.to_string())
            })
            .collect(),
        Some(_) => return Err("candidates debe ser un array".to_string()),
    };
    // Contrato pre-vNext (statement + record_id sin candidates): se responde
    // con un descarte explícito para que el agente reenvíe candidatos, en
    // vez de convertir la llamada en una propuesta pendiente.
    let legacy_statement = if args.get("candidates").is_none() {
        optional_str("statement")
    } else {
        None
    };
    let (project_root, repo_path) = resolve_roots(args)?;

    let outcome = pipeline::finalize(
        pipeline::FinalizeRequest {
            project_root,
            repo_path,
            operation_id: optional_str("operation_id"),
            summary: optional_str("summary"),
            target_spec: optional_str("target"),
            base_revision: optional_str("base_revision"),
            candidates,
            actor: session.actor.clone(),
            legacy_statement,
        },
        provider,
        &session.recorder,
    )?;
    serde_json::to_value(&outcome).map_err(|e| format!("no se pudo serializar la respuesta: {e}"))
}

fn call_resolve_conflict(args: &Value, session: &Session) -> Result<Value, String> {
    let conflict_id = args
        .get("conflict_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "falta el argumento requerido 'conflict_id'".to_string())?;
    let decision = args
        .get("decision")
        .and_then(Value::as_str)
        .and_then(canon::ConflictDecision::parse)
        .ok_or_else(|| "decision debe ser 'keep_pinned' o 'adopt_new'".to_string())?;
    let human_answer = args
        .get("human_answer")
        .and_then(Value::as_str)
        .map(canon::sanitize_control_chars)
        .filter(|answer| !answer.trim().is_empty())
        .ok_or_else(|| {
            "falta 'human_answer': resolve_conflict solo aplica una decisión que el humano ya \
             tomó — pregúntale y transcribe su respuesta"
                .to_string()
        })?;
    let (project_root, repo_path) = resolve_roots(args)?;
    let config = configuration::load(&project_root).map_err(|e| e.to_string())?;
    let actor = configuration::git_actor(&config.project_root);
    let declared = config.authority_for_actor(&actor).declared;
    let local_dir = configuration::find_rationale_local(&config.project_root);
    let ctx = canon::CanonContext {
        rationale_dir: &config.rationale_dir,
        project_id: &config.project_id,
        repo_path: &repo_path,
        local_dir: &local_dir,
        head_revision: crate::revision::snapshot(&repo_path).head,
        uncommitted_paths: Default::default(),
        actor: session.actor.clone(),
    };
    let resolution = canon::resolve_conflict(
        &ctx,
        conflict_id,
        decision,
        canon::HumanDecision {
            actor,
            declared,
            relayed_by: Some(session.actor.client.clone()),
            human_answer: Some(human_answer),
        },
    )?;
    let scope = activity::Scope::new(
        &local_dir,
        &config.project_root,
        &config.project_id,
        &session.actor,
    );
    session.recorder.emit(
        &scope,
        "conflict.resolved",
        activity::payload::conflict_resolved(&resolution),
    );
    for committed in &resolution.outcome.committed {
        session.recorder.emit(
            &scope,
            "record.committed",
            activity::payload::record_committed(committed),
        );
    }
    for superseded in &resolution.outcome.superseded {
        session.recorder.emit(
            &scope,
            "record.superseded",
            activity::payload::record_superseded(superseded),
        );
    }
    serde_json::to_value(&resolution)
        .map_err(|e| format!("no se pudo serializar la respuesta: {e}"))
}
