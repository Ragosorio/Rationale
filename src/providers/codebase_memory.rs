//! Cliente MCP persistente hacia Codebase Memory (ADR-0002).
//!
//! Sesión de larga duración: un solo `initialize` por vida del proceso,
//! cada operación subsecuente reutiliza la misma sesión (~15-30ms medido en
//! docs/research/codebase-memory/11-performance-observations.md, frente a
//! ~6.8s de arrancar un proceso nuevo por llamada).
//!
//! Reglas de `docs/research/codebase-memory/10-failure-modes.md` y
//! `12-integration-recommendation.md` aplicadas aquí:
//!   - Resultado vacío del proveedor -> coverage: Unknown, nunca "no existe".
//!   - Error del proveedor -> normalizado a Degraded, nunca se reenvía el
//!     string crudo del proveedor al agente.
//!   - Timeout -> fail open: se mata el proceso, se reporta Unavailable,
//!     nunca se bloquea la operación que lo invocó
//!     (Arquitectura §13.5 "Provider unavailable").

use super::{CodeIntelligenceProvider, Coverage, ProviderResult, ProviderStatus, ResolvedTarget};
use serde_json::{json, Value};
use std::io::{BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::Duration;

const INITIALIZE_DEADLINE: Duration = Duration::from_secs(15); // ver 11-performance-observations.md: ~6.8s medido, margen generoso
const CALL_DEADLINE: Duration = Duration::from_secs(5); // muy por encima de los ~15-30ms medidos en sesión cálida

pub struct CodebaseMemoryClient {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<Value>,
    next_id: u64,
    binary: String,
}

impl CodebaseMemoryClient {
    /// Arranca el binario de Codebase Memory como sesión MCP persistente.
    /// Busca `codebase-memory-mcp` en PATH (el binario que un usuario real
    /// tendría instalado) — no asume una versión ni una ruta específica.
    pub fn spawn() -> std::io::Result<Self> {
        let binary = "codebase-memory-mcp".to_string();
        let mut child = Command::new(&binary)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let stdin = child.stdin.take().expect("stdin piped");
        let stdout = child.stdout.take().expect("stdout piped");

        // Hilo lector: única fuente de mensajes entrantes durante toda la
        // vida de la sesión. Como esta vertical slice hace llamadas
        // estrictamente secuenciales (nunca concurrentes), cualquier
        // mensaje que llegue es la respuesta a la última petición enviada.
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            while let Some(msg) = read_mcp_message(&mut reader) {
                if tx.send(msg).is_err() {
                    break;
                }
            }
        });

        let mut client = CodebaseMemoryClient {
            child,
            stdin,
            rx,
            next_id: 1,
            binary,
        };

        client.initialize()?;
        Ok(client)
    }

    fn initialize(&mut self) -> std::io::Result<()> {
        let id = self.next_id();
        self.send(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "rationale", "version": env!("CARGO_PKG_VERSION")}
            }
        }))?;
        self.recv_with_deadline(INITIALIZE_DEADLINE);
        self.send(json!({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}))?;
        Ok(())
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn send(&mut self, value: Value) -> std::io::Result<()> {
        let body = serde_json::to_string(&value).expect("serialize request");
        write!(self.stdin, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
        self.stdin.flush()
    }

    /// Espera una respuesta con deadline. Si expira, mata el proceso
    /// (fail open — nunca deja la sesión en un estado ambiguo) y devuelve
    /// `None`. La llamada siguiente detectará el proceso muerto al escribir.
    fn recv_with_deadline(&mut self, deadline: Duration) -> Option<Value> {
        match self.rx.recv_timeout(deadline) {
            Ok(v) => Some(v),
            Err(RecvTimeoutError::Timeout) => {
                let _ = self.child.kill();
                let _ = self.child.wait();
                None
            }
            Err(RecvTimeoutError::Disconnected) => None,
        }
    }

    fn call_tool(&mut self, name: &str, arguments: Value) -> Option<Value> {
        let id = self.next_id();
        if self
            .send(json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "tools/call",
                "params": {"name": name, "arguments": arguments}
            }))
            .is_err()
        {
            return None;
        }
        self.recv_with_deadline(CALL_DEADLINE)
    }

    /// Extrae el contenido de texto de una respuesta `tools/call` y lo
    /// reparsea como JSON (así es como CBM envuelve sus respuestas —
    /// verificado empíricamente en docs/research/codebase-memory/03).
    fn extract_tool_json(response: &Value) -> Option<Value> {
        let text = response
            .get("result")?
            .get("content")?
            .as_array()?
            .first()?
            .get("text")?
            .as_str()?;
        serde_json::from_str(text).ok()
    }

    fn project_name_for(repo_path: &str) -> String {
        // Misma convención observada empíricamente en list_projects
        // (docs/research/codebase-memory/07-storage-and-cache.md): la ruta
        // absoluta con separadores reemplazados por guiones.
        repo_path.trim_start_matches('/').replace('/', "-")
    }
}

impl Drop for CodebaseMemoryClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl CodeIntelligenceProvider for CodebaseMemoryClient {
    fn health(&mut self, repo_path: &str) -> ProviderResult<()> {
        let project = Self::project_name_for(repo_path);
        match self.call_tool("index_status", json!({"project": project})) {
            None => ProviderResult {
                data: None,
                provider_name: self.binary.clone(),
                status: ProviderStatus::Unavailable,
                coverage: Coverage::Unknown,
                warnings: vec!["provider no respondió dentro del deadline".to_string()],
            },
            Some(resp) => match Self::extract_tool_json(&resp) {
                Some(v) if v.get("status").and_then(|s| s.as_str()) == Some("ready") => {
                    ProviderResult {
                        data: Some(()),
                        provider_name: self.binary.clone(),
                        status: ProviderStatus::Successful,
                        coverage: Coverage::Complete,
                        warnings: vec![],
                    }
                }
                _ => ProviderResult {
                    data: None,
                    provider_name: self.binary.clone(),
                    status: ProviderStatus::Degraded,
                    coverage: Coverage::Unknown,
                    warnings: vec!["proyecto no indexado o respuesta inesperada".to_string()],
                },
            },
        }
    }

    fn resolve_target(
        &mut self,
        repo_path: &str,
        symbol_name: &str,
    ) -> ProviderResult<ResolvedTarget> {
        let project = Self::project_name_for(repo_path);

        // Asegurar indexación antes de buscar (mode="fast": suficiente para
        // resolver un símbolo, sin pagar similarity/semantic innecesarios
        // en esta vertical slice — Rationale_v0.5.md §20.5.1 "evitar... lo
        // que no sea estrictamente necesario para la operación").
        let _ = self.call_tool(
            "index_repository",
            json!({"repo_path": repo_path, "mode": "fast"}),
        );

        let search = self.call_tool(
            "search_graph",
            json!({"project": project, "name_pattern": symbol_name, "limit": 5}),
        );

        match search {
            None => ProviderResult {
                data: None,
                provider_name: self.binary.clone(),
                status: ProviderStatus::Unavailable,
                coverage: Coverage::Unknown,
                warnings: vec!["provider no respondió dentro del deadline".to_string()],
            },
            Some(resp) => {
                let parsed = Self::extract_tool_json(&resp);
                let results = parsed
                    .as_ref()
                    .and_then(|v| v.get("results"))
                    .and_then(|r| r.as_array());

                match results.and_then(|r| r.first()) {
                    Some(node) => {
                        let qualified_name = node
                            .get("qualified_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or(symbol_name)
                            .to_string();
                        let file_path = node
                            .get("file_path")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        ProviderResult {
                            data: Some(ResolvedTarget {
                                qualified_name,
                                file_path,
                            }),
                            provider_name: self.binary.clone(),
                            status: ProviderStatus::Successful,
                            coverage: Coverage::Complete,
                            warnings: vec![],
                        }
                    }
                    // Ausencia de resultado != "el símbolo no existe"
                    // (Rationale_v0.5.md §19.2, confirmado empíricamente en
                    // docs/research/codebase-memory/08-workspaces-and-monorepos.md).
                    None => ProviderResult {
                        data: None,
                        provider_name: self.binary.clone(),
                        status: ProviderStatus::Successful,
                        coverage: Coverage::Unknown,
                        warnings: vec![
                            "no se encontró el símbolo dentro de la cobertura disponible; no implica que no exista".to_string(),
                        ],
                    },
                }
            }
        }
    }
}

/// Mismo framing Content-Length verificado empíricamente contra Codebase
/// Memory en el spike de lenguaje (src/mcp/mcp.c, confirmado en
/// docs/research/codebase-memory/11-performance-observations.md).
fn read_mcp_message(reader: &mut dyn Read) -> Option<Value> {
    let mut header = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        if reader.read(&mut byte).ok()? == 0 {
            return None;
        }
        header.push(byte[0]);
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let header_str = String::from_utf8_lossy(&header);
    let length: usize = header_str
        .lines()
        .find(|l| l.to_lowercase().starts_with("content-length:"))?
        .split(':')
        .nth(1)?
        .trim()
        .parse()
        .ok()?;
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}
