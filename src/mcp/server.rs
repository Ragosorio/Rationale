//! Servidor MCP de Rationale — Fase E5 (ADR-0007).
//!
//! Bucle síncrono sobre stdin/stdout con el framing de `mcp::framing`, sin
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

    // Sesión persistente para toda la vida del servidor — no una por
    // llamada (ADR-0002 / ADR-0007).
    let mut provider = ProviderHandle::spawn();

    loop {
        let msg = match framing::read_message(&mut reader) {
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
                let _ = framing::write_message(&mut writer, &resp);
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
                        "capabilities": {"tools": {}},
                        "serverInfo": {"name": "rationale", "version": env!("CARGO_PKG_VERSION")}
                    }
                });
                let _ = framing::write_message(&mut writer, &resp);
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
                let _ = framing::write_message(&mut writer, &resp);
            }
            "tools/call" => {
                let resp = handle_tools_call(&msg, id, &mut provider);
                let _ = framing::write_message(&mut writer, &resp);
            }
            _ => {
                if let Some(id) = id {
                    let resp = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {"code": -32601, "message": "method not found"}
                    });
                    let _ = framing::write_message(&mut writer, &resp);
                }
            }
        }
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
                    "intent": {"type": "string", "description": "Intención declarada opcional — solo se usa en modo intent-aware"},
                    "mode": {"type": "string", "enum": ["baseline", "intent-aware"], "default": "baseline"},
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
        }
    ])
}

fn handle_tools_call(msg: &Value, id: Option<Value>, provider: &mut ProviderHandle) -> Value {
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
        "prepare_change" => call_prepare_change(&arguments, provider),
        "explain_target" => call_explain_target(&arguments),
        "health" => call_health(&arguments, provider),
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
    let mode = args
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("baseline");
    let (project_root, repo_path) = resolve_roots(args)?;

    // Modo baseline (v0.5 §4.18/§24.1): ignora la intención aunque venga —
    // solo intent-aware activa la detección de conflictos.
    let effective_intent = if mode == "intent-aware" { intent } else { None };

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
    );

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

    let outcome = pipeline::explain(target, &project_root, &repo_path);

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

    let outcome = pipeline::health(&project_root, provider);

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
