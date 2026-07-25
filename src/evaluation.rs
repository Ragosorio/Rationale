//! Evaluation and Telemetry — instrumentación desde la primera vertical,
//! no al final (Arquitectura §20, §11.14).
//!
//! Local únicamente: nunca se envía automáticamente a ningún servicio
//! (Arquitectura §11.14 "No enviar datos automáticamente").

use serde::Serialize;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Serialize)]
pub struct RunLog {
    pub event: String,
    pub timestamp: String,
    pub latency_ms: u128,
    pub git_revision: Option<String>,
    pub consistency: String,
    pub provider_status: String,
    pub provider_coverage: String,
    pub packet_bytes: usize,
}

/// Escribe un evento NDJSON en `.rationale-local/runs/` (ignorado por Git,
/// Rationale_Proceso_Construccion_Agentes_v0.1.md §11).
pub fn record_run(rationale_local_dir: &Path, log: &RunLog) -> std::io::Result<()> {
    let runs_dir = rationale_local_dir.join("runs");
    std::fs::create_dir_all(&runs_dir)?;
    let log_path = runs_dir.join("vertical-slice.ndjson");

    let line = serde_json::to_string(log).expect("serialize run log");
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    writeln!(file, "{line}")
}

pub fn now_iso8601() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    // Formato simple sin dependencia de chrono/time — suficiente para logs
    // locales de diagnóstico; no se usa para lógica de negocio.
    format!("epoch:{}", duration.as_secs())
}
