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
use crate::mcp::framing;
use serde_json::{json, Value};
use std::io::BufReader;
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
    init_deadline: Duration,
    call_deadline: Duration,
}

impl CodebaseMemoryClient {
    /// Arranca el binario de Codebase Memory como sesión MCP persistente.
    /// Busca `codebase-memory-mcp` en PATH (el binario que un usuario real
    /// tendría instalado) — no asume una versión ni una ruta específica.
    pub fn spawn() -> std::io::Result<Self> {
        Self::spawn_with("codebase-memory-mcp", INITIALIZE_DEADLINE, CALL_DEADLINE)
    }

    /// Variante con binario y deadlines inyectables — usada en producción
    /// solo indirectamente vía `spawn()`; existe para poder probar
    /// `Unavailable` (binario inexistente) y timeout de llamada (deadline
    /// corto contra un mock) sin depender de un binario real ni esperar
    /// segundos reales en cada corrida de tests (`docs/rust/testing-guide.md`).
    pub fn spawn_with(
        binary: &str,
        init_deadline: Duration,
        call_deadline: Duration,
    ) -> std::io::Result<Self> {
        let binary = binary.to_string();
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
            // EOF real o un mensaje que Codebase Memory no pudo formar
            // correctamente terminan el hilo por igual — el `recv_timeout`
            // del caller ya trata un canal desconectado como `Unavailable`
            // (fail open, Arquitectura §13.5).
            while let framing::Frame::Message(msg) = framing::read_message(&mut reader) {
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
            init_deadline,
            call_deadline,
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
        self.recv_with_deadline(self.init_deadline);
        self.send(json!({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}))?;
        Ok(())
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn send(&mut self, value: Value) -> std::io::Result<()> {
        framing::write_message(&mut self.stdin, &value)
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
        self.recv_with_deadline(self.call_deadline)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn mock_slow_server_path() -> String {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/mock-mcp/slow_server.sh")
            .to_string_lossy()
            .to_string()
    }

    /// D5 — "provider unavailable": un binario inexistente debe fallar al
    /// spawnearse, nunca colgar ni entrar en un estado ambiguo.
    #[test]
    fn provider_unavailable_when_binary_does_not_exist() {
        let result = CodebaseMemoryClient::spawn_with(
            "this-binary-definitely-does-not-exist-rationale-test",
            Duration::from_secs(1),
            Duration::from_secs(1),
        );
        assert!(
            result.is_err(),
            "spawnear un binario inexistente debe fallar"
        );
    }

    /// D5 — "provider timeout": un proveedor que responde initialize pero
    /// nunca responde a una llamada posterior debe reportarse Unavailable
    /// dentro del deadline configurado, y el proceso debe quedar matado
    /// (fail open, Arquitectura §13.5) — nunca colgar la operación llamante.
    #[test]
    fn provider_timeout_reports_unavailable_and_kills_process() {
        let mut client = CodebaseMemoryClient::spawn_with(
            &mock_slow_server_path(),
            Duration::from_secs(2),     // initialize: el mock responde rápido
            Duration::from_millis(300), // tools/call: el mock nunca responde
        )
        .expect("el mock server debe arrancar e inicializar correctamente");

        let started = std::time::Instant::now();
        let result = client.health("/tmp/does-not-matter");
        let elapsed = started.elapsed();

        assert!(matches!(result.status, ProviderStatus::Unavailable));
        assert!(
            elapsed < Duration::from_secs(2),
            "debe respetar el deadline corto (300ms), no colgarse: tardó {elapsed:?}"
        );

        // Fail open verificado: el proceso fue matado, no abandonado vivo.
        let wait_result = client.child.try_wait();
        assert!(
            matches!(wait_result, Ok(Some(_))),
            "el proceso del proveedor debe estar terminado tras el timeout"
        );
    }
}
