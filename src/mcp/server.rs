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
use crate::{configuration, pipeline, retrieval};
use serde_json::{json, Value};
use std::io::{self, BufReader};
use std::path::PathBuf;

const PROTOCOL_VERSION: &str = "2024-11-05";

pub fn run() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();

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
                let resp = handle_tools_call(&msg, id, &mut provider);
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
        return json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": -32602, "message": format!("prompt desconocido: '{name}'")}
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

fn tool_definitions() -> Value {
    json!([
        {
            "name": "prepare_change",
            "description": "Snapshot de consistencia, constraints críticas aprobadas, conflictos con la intención, riesgos y cobertura para un target antes de cambiarlo.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "target": {"type": "string", "description": "path::symbol dentro del proyecto, ej. src/main.rs::cmd_prepare"},
                    "intent": {"type": "string", "description": "Intención declarada opcional — si viene, activa detección de conflictos automáticamente (v0.5 §4.18)"},
                    "mode": {"type": "string", "enum": ["baseline", "intent-aware"], "description": "Normalmente no hace falta: el modo se infiere de si viene 'intent'. Pasa 'baseline' explícito para forzar retrieval puro aunque venga 'intent'."},
                    "project_root": {"type": "string", "description": "Por defecto: el proyecto Rationale que contiene el cwd del servidor"},
                    "repo_path": {"type": "string"},
                    "max_tokens": {"type": "integer"},
                    "max_critical_constraints": {"type": "integer"},
                    "max_risks": {"type": "integer"}
                },
                "required": ["target"]
            }
        },
        {
            "name": "explain_target",
            "description": "Por qué existe un target, qué Records lo gobiernan por binding exacto, y qué es conocido vs desconocido.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "target": {"type": "string"},
                    "project_root": {"type": "string"},
                    "repo_path": {"type": "string"}
                },
                "required": ["target"]
            }
        },
        {
            "name": "health",
            "description": "Revisión Git, working tree, proveedor estructural y cobertura — y qué no sabe.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_root": {"type": "string"}
                }
            }
        },
        {
            "name": "finalize_change",
            "description": "Captura mecánica del diff + señales de alto valor + resolución de Subject, y escribe una propuesta PENDIENTE en .rationale/proposals/ (nunca en records/ directamente — solo `rationale review` aprueba). Si el cambio es puramente mecánico (Nivel 0, v0.5 §16), no escribe nada.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "target": {"type": "string", "description": "path::symbol del target principal del cambio"},
                    "base_revision": {"type": "string", "description": "Revisión Git desde la que capturar el diff — normalmente la que un prepare_change anterior reportó"},
                    "intent": {"type": "string", "description": "Por qué se hizo el cambio — nunca inferido, debe venir de quien hizo el cambio"},
                    "statement": {"type": "string", "description": "La afirmación normativa propuesta (solo se usa si el nivel resultante supera Git-only)"},
                    "severity": {"type": "string", "enum": ["critical", "high", "medium", "low"], "description": "Peso de la afirmación normativa — sin default: inventar uno oculta al Record en retrieval sin avisar"},
                    "kind": {"type": "string", "enum": ["constraint", "decision", "risk", "exception"], "description": "Naturaleza de la afirmación — por defecto 'constraint' si se omite"},
                    "record_id": {"type": "string", "description": "Slug único para la propuesta, ej. constraint.no-double-charge"},
                    "subject_id": {"type": "string", "description": "Subject candidato — el Subject Resolver contrasta contra el canon existente"},
                    "subject_title": {"type": "string"},
                    "subject_type": {"type": "string", "description": "Clasificación libre del Subject (ver subject.schema.json); si se omite, se materializa como 'unclassified'"},
                    "novelty_reason": {
                        "type": "object",
                        "description": "Contraste estructurado requerido al no reutilizar un candidato fuerte (v0.5 §9.2)",
                        "required": ["contrasted_subject", "difference_kind", "difference", "evidence"],
                        "properties": {
                            "contrasted_subject": {"type": "string"},
                            "difference_kind": {"type": "string", "enum": ["behavior", "scope", "lifecycle", "authority", "invariant"]},
                            "difference": {"type": "string"},
                            "evidence": {"type": "string"}
                        }
                    },
                    "risks": {"type": "array", "items": {"type": "string"}},
                    "project_root": {"type": "string"},
                    "repo_path": {"type": "string"}
                },
                "required": ["target", "base_revision", "intent", "statement", "severity", "record_id", "subject_id", "subject_title"]
            }
        }
    ])
}

fn handle_tools_call(
    msg: &Value,
    id: Option<Value>,
    provider: &mut Option<ProviderHandle>,
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
        "prepare_change" => call_prepare_change(&arguments, provider_mut(provider)),
        "explain_target" => call_explain_target(&arguments),
        "health" => call_health(&arguments, provider_mut(provider)),
        "finalize_change" => call_finalize_change(&arguments, provider_mut(provider)),
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

fn call_prepare_change(args: &Value, provider: &mut ProviderHandle) -> Result<Value, String> {
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

    let outcome = pipeline::prepare(
        &pipeline::PrepareRequest {
            target_spec: target,
            intent: effective_intent,
            project_root,
            repo_path,
            budget,
        },
        provider,
    )?;

    Ok(json!({
        "packet": outcome.packet,
        "assessment": outcome.assessment,
        "diagnostics": outcome.diagnostics,
        "latency_ms": outcome.latency_ms,
    }))
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

fn call_finalize_change(args: &Value, provider: &mut ProviderHandle) -> Result<Value, String> {
    let require_str = |field: &str| -> Result<String, String> {
        args.get(field)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| format!("falta el argumento requerido '{field}'"))
    };

    let target_spec = require_str("target")?;
    let base_revision = require_str("base_revision")?;
    let intent = require_str("intent")?;
    let statement = require_str("statement")?;
    let record_id = require_str("record_id")?;
    let subject_id = require_str("subject_id")?;
    let subject_title = require_str("subject_title")?;
    // Antes defaulteaba a "normal" — un valor fuera del propio enum del
    // schema (`critical|high|medium|low`), que hacía invisible el Record
    // en retrieval sin ningún error (el defecto real de un dogfood). Un
    // default de severidad es Rationale inventando el peso de una
    // afirmación normativa ajena; ahora se exige explícita y se valida.
    let severity = require_str("severity")?;
    if crate::storage::Severity::parse(&severity).is_none() {
        return Err(format!(
            "severity '{severity}' inválida — valores válidos: {}",
            crate::storage::Severity::ALL.join(", ")
        ));
    }
    // `kind: "exception"` está en el enum del schema desde el principio
    // (junto a constraint/decision/risk), pero nada lo escribía: el
    // productor hardcodeaba "constraint" siempre. `None` sigue
    // defaulteando a "constraint" — el comportamiento de antes, no un
    // valor nuevo inventado — pero un caller ya puede declarar que una
    // afirmación es una excepción real.
    let kind = args.get("kind").and_then(|v| v.as_str());
    if let Some(kind) = kind {
        const VALID_KINDS: [&str; 4] = ["constraint", "decision", "risk", "exception"];
        if !VALID_KINDS.contains(&kind) {
            return Err(format!(
                "kind '{kind}' inválido — valores válidos: {}",
                VALID_KINDS.join(", ")
            ));
        }
    }
    let kind = kind.unwrap_or("constraint").to_string();
    let subject_type = args
        .get("subject_type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let novelty_reason = args
        .get("novelty_reason")
        .map(|value| {
            serde_json::from_value::<crate::subjects::NoveltyReason>(value.clone())
                .map_err(|e| format!("novelty_reason inválida: {e}"))
        })
        .transpose()?;
    let risks: Vec<String> = args
        .get("risks")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let (project_root, repo_path) = resolve_roots(args)?;

    let outcome = pipeline::finalize(
        &pipeline::FinalizeRequest {
            target_spec,
            project_root,
            repo_path,
            base_revision,
            intent,
            statement,
            severity,
            kind,
            record_id,
            subject_id,
            subject_title,
            subject_type,
            novelty_reason,
            risks,
        },
        provider,
    )?;

    Ok(json!({
        "level": outcome.level,
        "signals": outcome.signals,
        "capture": outcome.capture,
        "subject_resolution": outcome.subject_resolution,
        "proposal_written": outcome.proposal_written,
        "proposal_path": outcome.proposal_path,
        "proposal_id": outcome.proposal_id,
        "blocked_reason": outcome.blocked_reason,
        "diagnostics": outcome.diagnostics,
    }))
}
