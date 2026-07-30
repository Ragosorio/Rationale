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
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

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
        // vida de la sesión. Las llamadas son secuenciales, pero MCP también
        // permite notificaciones y mensajes de progreso entre petición y
        // respuesta: el caller correlaciona cada respuesta por `id`.
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            // EOF real o un mensaje que Codebase Memory no pudo formar
            // correctamente terminan el hilo por igual — el `recv_timeout`
            // del caller ya trata un canal desconectado como `Unavailable`
            // (fail open, Arquitectura §13.5).
            while let framing::Frame::Message(msg) = framing::read_content_length(&mut reader) {
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
        if self.recv_response_for(id, self.init_deadline).is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Codebase Memory no respondió initialize con el id esperado",
            ));
        }
        self.send(json!({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}))?;
        Ok(())
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn send(&mut self, value: Value) -> std::io::Result<()> {
        framing::write_content_length(&mut self.stdin, &value)
    }

    /// Espera la respuesta de una petición concreta. Notificaciones y
    /// respuestas atrasadas con otro `id` no pueden hacerse pasar por ella.
    /// Si el deadline total expira, mata el proceso (fail open) para no dejar
    /// una sesión cuya correlación ya no es confiable.
    fn recv_response_for(&mut self, expected_id: u64, deadline: Duration) -> Option<Value> {
        let started = Instant::now();
        loop {
            let Some(remaining) = deadline.checked_sub(started.elapsed()) else {
                let _ = self.child.kill();
                let _ = self.child.wait();
                return None;
            };
            match self.rx.recv_timeout(remaining) {
                Ok(value) => {
                    if value.get("id").and_then(Value::as_u64) == Some(expected_id) {
                        return Some(value);
                    }
                    // Mensaje sin id = notificación. Un id distinto puede ser
                    // una respuesta tardía. Ninguno satisface esta llamada.
                }
                Err(RecvTimeoutError::Timeout) => {
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                    return None;
                }
                Err(RecvTimeoutError::Disconnected) => return None,
            }
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
        self.recv_response_for(id, self.call_deadline)
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

    /// Resuelve la identidad derivada del proveedor únicamente mediante su
    /// contrato público. Rationale no replica el algoritmo de nombres de
    /// Codebase Memory: ese algoritmo ya cambió (p. ej. colapsa guiones) y
    /// dos implementaciones inevitablemente divergen.
    fn project_for_repo(&mut self, repo_path: &str) -> Result<Option<String>, ()> {
        if let Some(project) = Self::load_project_mapping(repo_path) {
            return Ok(Some(project));
        }
        let response = self.call_tool("list_projects", json!({})).ok_or(())?;
        let Some(payload) = Self::extract_tool_json(&response) else {
            return Ok(None);
        };
        let project = Self::project_from_list(&payload, repo_path);
        if let Some(project) = &project {
            let _ = Self::save_project_mapping(repo_path, project);
        }
        Ok(project)
    }

    fn project_from_list(payload: &Value, repo_path: &str) -> Option<String> {
        let requested = canonical_or_original(Path::new(repo_path));
        payload
            .get("projects")?
            .as_array()?
            .iter()
            .find(|project| {
                project
                    .get("root_path")
                    .and_then(Value::as_str)
                    .map(|root| canonical_or_original(Path::new(root)) == requested)
                    .unwrap_or(false)
            })
            .and_then(|project| project.get("name"))
            .and_then(Value::as_str)
            .map(str::to_owned)
    }

    fn project_from_index(payload: &Value) -> Option<String> {
        payload.get("project")?.as_str().map(str::to_owned)
    }

    fn index_project(&mut self, repo_path: &str) -> Result<Option<String>, ()> {
        let response = self
            .call_tool(
                "index_repository",
                json!({"repo_path": repo_path, "mode": "fast"}),
            )
            .ok_or(())?;
        let project = Self::extract_tool_json(&response)
            .as_ref()
            .and_then(Self::project_from_index);
        if let Some(project) = &project {
            let _ = Self::save_project_mapping(repo_path, project);
        }
        Ok(project)
    }

    fn mapping_path(repo_path: &str) -> std::path::PathBuf {
        Path::new(repo_path)
            .join(".rationale-local")
            .join("codebase-memory-project.json")
    }

    fn load_project_mapping(repo_path: &str) -> Option<String> {
        let content = std::fs::read_to_string(Self::mapping_path(repo_path)).ok()?;
        let value: Value = serde_json::from_str(&content).ok()?;
        let stored_root = value.get("root_path")?.as_str()?;
        if canonical_or_original(Path::new(stored_root))
            != canonical_or_original(Path::new(repo_path))
        {
            return None;
        }
        value.get("project")?.as_str().map(str::to_owned)
    }

    fn save_project_mapping(repo_path: &str, project: &str) -> Result<(), String> {
        if !Path::new(repo_path).join(".rationale").is_dir() {
            return Err("el repositorio no está inicializado con Rationale".to_string());
        }
        let path = Self::mapping_path(repo_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("no se pudo crear {}: {error}", parent.display()))?;
        }
        let payload = json!({
            "provider": "codebase-memory-mcp",
            "project": project,
            "root_path": canonical_or_original(Path::new(repo_path)),
        });
        let mut bytes = serde_json::to_vec_pretty(&payload)
            .map_err(|error| format!("no se pudo serializar el vínculo: {error}"))?;
        bytes.push(b'\n');
        crate::storage::atomic_write_bytes(&path, &bytes)
            .map_err(|error| format!("no se pudo guardar {}: {error}", path.display()))
    }
}

fn canonical_or_original(path: &Path) -> std::path::PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

impl Drop for CodebaseMemoryClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl CodeIntelligenceProvider for CodebaseMemoryClient {
    fn health(&mut self, repo_path: &str) -> ProviderResult<()> {
        let project = match self.project_for_repo(repo_path) {
            Err(()) => {
                return ProviderResult {
                    data: None,
                    provider_name: self.binary.clone(),
                    status: ProviderStatus::Unavailable,
                    coverage: Coverage::Unknown,
                    warnings: vec!["provider no respondió dentro del deadline".to_string()],
                };
            }
            Ok(Some(project)) => project,
            Ok(None) => match self.index_project(repo_path) {
                Err(()) => {
                    return ProviderResult {
                        data: None,
                        provider_name: self.binary.clone(),
                        status: ProviderStatus::Unavailable,
                        coverage: Coverage::Unknown,
                        warnings: vec![
                            "provider no respondió durante la vinculación inicial".to_string()
                        ],
                    };
                }
                Ok(Some(project)) => project,
                Ok(None) => {
                    return ProviderResult {
                        data: None,
                        provider_name: self.binary.clone(),
                        status: ProviderStatus::Degraded,
                        coverage: Coverage::Unknown,
                        warnings: vec![
                            "Codebase Memory no devolvió una identidad tras la vinculación inicial"
                                .to_string(),
                        ],
                    };
                }
            },
        };
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
        file_path: &str,
        symbol_name: &str,
    ) -> ProviderResult<ResolvedTarget> {
        // Consultar primero. La implementación anterior reindexaba en cada
        // prepare/finalize aunque el proyecto ya estuviera listo, reemplazando
        // innecesariamente la generación derivada del proveedor. Solo se
        // indexa cuando list_projects confirma que aún no existe.
        let project = match self.project_for_repo(repo_path) {
            Err(()) => {
                return ProviderResult {
                    data: None,
                    provider_name: self.binary.clone(),
                    status: ProviderStatus::Unavailable,
                    coverage: Coverage::Unknown,
                    warnings: vec!["provider no respondió dentro del deadline".to_string()],
                };
            }
            Ok(Some(project)) => project,
            Ok(None) => match self.index_project(repo_path) {
                Err(()) => {
                    return ProviderResult {
                        data: None,
                        provider_name: self.binary.clone(),
                        status: ProviderStatus::Unavailable,
                        coverage: Coverage::Unknown,
                        warnings: vec![
                            "provider no respondió durante la indexación inicial".to_string()
                        ],
                    }
                }
                Ok(Some(project)) => project,
                Ok(None) => {
                    return ProviderResult {
                        data: None,
                        provider_name: self.binary.clone(),
                        status: ProviderStatus::Degraded,
                        coverage: Coverage::Unknown,
                        warnings: vec![
                            "Codebase Memory no devolvió una identidad de proyecto tras indexar"
                                .to_string(),
                        ],
                    }
                }
            },
        };

        let search = self.call_tool(
            "search_graph",
            json!({
                "project": project,
                "name_pattern": symbol_name,
                "file_pattern": file_path,
                "limit": 20
            }),
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

                let selected = results.and_then(|results| {
                    results
                        .iter()
                        .find(|node| {
                            node.get("name").and_then(Value::as_str) == Some(symbol_name)
                                && node.get("file_path").and_then(Value::as_str) == Some(file_path)
                        })
                        .or_else(|| results.first())
                });

                match selected {
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
    #[cfg(not(windows))]
    use std::path::Path;

    #[cfg(not(windows))]
    fn mock_slow_server_path() -> String {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/mock-mcp/slow_server.sh")
            .to_string_lossy()
            .to_string()
    }

    #[cfg(not(windows))]
    fn mock_interleaved_server_path() -> String {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/mock-mcp/interleaved_server.sh")
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

    #[test]
    fn project_identity_comes_from_public_root_path_not_reimplemented_name() {
        let payload = json!({
            "projects": [{
                "name": "private-tmp-owner-project",
                "root_path": "/private/tmp/-owner/project"
            }]
        });

        assert_eq!(
            CodebaseMemoryClient::project_from_list(&payload, "/private/tmp/-owner/project"),
            Some("private-tmp-owner-project".to_string())
        );
    }

    #[test]
    fn initial_index_identity_comes_from_the_public_response() {
        let payload = json!({
            "project": "provider-owned-project-name",
            "status": "indexed"
        });
        assert_eq!(
            CodebaseMemoryClient::project_from_index(&payload),
            Some("provider-owned-project-name".to_string())
        );
    }

    #[test]
    fn provider_project_mapping_roundtrips_and_is_root_scoped() {
        let repo = std::env::temp_dir().join(format!(
            "rationale-provider-map-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::create_dir_all(repo.join(".rationale")).unwrap();
        let root = repo.to_string_lossy();
        CodebaseMemoryClient::save_project_mapping(&root, "provider-project").unwrap();
        assert_eq!(
            CodebaseMemoryClient::load_project_mapping(&root),
            Some("provider-project".to_string())
        );
        assert_eq!(
            CodebaseMemoryClient::load_project_mapping(&repo.join("different").to_string_lossy()),
            None
        );
        std::fs::remove_dir_all(repo).ok();
    }

    #[test]
    #[cfg(not(windows))]
    fn notifications_cannot_be_mistaken_for_tool_responses() {
        let mut client = CodebaseMemoryClient::spawn_with(
            &mock_interleaved_server_path(),
            Duration::from_secs(2),
            Duration::from_secs(2),
        )
        .expect("el mock intercalado debe inicializar");
        let result = client.health("/tmp/repo");
        assert!(matches!(result.status, ProviderStatus::Successful));
        assert!(matches!(result.coverage, Coverage::Complete));
    }

    /// D5 — "provider timeout": un proveedor que responde initialize pero
    /// nunca responde a una llamada posterior debe reportarse Unavailable
    /// dentro del deadline configurado, y el proceso debe quedar matado
    /// (fail open, Arquitectura §13.5) — nunca colgar la operación llamante.
    ///
    /// Se salta en Windows: el mock es un script bash (`dd`, `BASH_REMATCH`,
    /// framing byte-exacto) — Windows no puede `CreateProcess` un `.sh`
    /// directamente ("%1 is not a valid Win32 application"). Esto es una
    /// limitación del fixture de prueba, no del código bajo prueba:
    /// `spawn_with` en producción siempre lanza un binario real
    /// (`codebase-memory-mcp` o su `.exe`), nunca un script — el
    /// comportamiento de timeout/kill que este test verifica no tiene
    /// ninguna rama específica de plataforma en `spawn_with` ni en
    /// `health()`. Reescribir el mock en un lenguaje que Windows pueda
    /// ejecutar nativamente (PowerShell) es trabajo real si hace falta
    /// cobertura Windows de este camino específico.
    #[test]
    #[cfg(not(windows))]
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
