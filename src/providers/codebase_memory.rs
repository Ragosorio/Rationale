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

use super::{
    ArchitectureCluster, ArchitectureSummary, CodeIntelligenceProvider, CodeSnippet, Coverage,
    Direction, IndexStatus, NeighborQuery, NodeBinding, ProviderCapabilities, ProviderResult,
    ProviderStatus, RelationKind, ResolvedTarget, StructuralNode, StructuralPath,
    StructuralRelationship, StructuralSubgraph,
};
use crate::mcp::framing;
use serde_json::{json, Value};
use std::io::BufReader;
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

const INITIALIZE_DEADLINE: Duration = Duration::from_secs(15); // ver 11-performance-observations.md: ~6.8s medido, margen generoso
const CALL_DEADLINE: Duration = Duration::from_secs(5); // muy por encima de los ~15-30ms medidos en sesión cálida
/// Indexar un repositorio real no es una llamada de 15-30ms, y la
/// investigación no midió su duración: con el deadline de una llamada normal
/// la indexación de primer uso de un repo mediano expiraría, mataría la sesión
/// y nunca terminaría. Solo `index_repository` usa este techo.
const INDEX_DEADLINE: Duration = Duration::from_secs(180);

pub struct CodebaseMemoryClient {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<Value>,
    next_id: u64,
    binary: String,
    init_deadline: Duration,
    call_deadline: Duration,
    index_deadline: Duration,
    /// Proyectos ya confirmados en esta sesión (repo → nombre del proveedor).
    validated_projects: std::collections::HashMap<String, String>,
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
            index_deadline: INDEX_DEADLINE.max(call_deadline),
            validated_projects: std::collections::HashMap::new(),
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
        let deadline = self.call_deadline;
        self.call_tool_within(name, arguments, deadline)
    }

    fn call_tool_within(
        &mut self,
        name: &str,
        arguments: Value,
        deadline: Duration,
    ) -> Option<Value> {
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
        self.recv_response_for(id, deadline)
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
    ///
    /// `Ok(None)` solo cuando la lista se leyó y el repo no está: una lista
    /// ilegible degrada en vez de invitar a reindexar un proyecto existente.
    fn project_from_listing(&mut self, repo_path: &str) -> Result<Option<String>, CallFailure> {
        let response = self.call_tool("list_projects", json!({})).ok_or_else(|| {
            CallFailure::Unavailable("provider no respondió dentro del deadline".to_string())
        })?;
        let payload = Self::extract_tool_json(&response).ok_or_else(|| {
            CallFailure::Degraded(
                "respuesta inesperada de Codebase Memory en list_projects".to_string(),
            )
        })?;
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
                    // Un `root_path` relativo depende del cwd con el que se
                    // indexó (visto: `fixtures/vertical-slice/repo`);
                    // resolverlo contra el cwd de Rationale podría atar este
                    // repo al índice de otro.
                    .filter(|root| Path::new(root).is_absolute())
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

    /// Pide la indexación con la ruta absoluta y devuelve la identidad que el
    /// proveedor declara. No la recuerda: `resolve_project` la confirma antes.
    fn index_project(&mut self, repo_path: &str) -> Result<Option<String>, ()> {
        let deadline = self.index_deadline;
        let response = self
            .call_tool_within(
                "index_repository",
                json!({"repo_path": provider_repo_path(repo_path), "mode": "fast"}),
                deadline,
            )
            .ok_or(())?;
        Ok(Self::extract_tool_json(&response)
            .as_ref()
            .and_then(Self::project_from_index))
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

    fn forget_project_mapping(repo_path: &str) {
        let _ = std::fs::remove_file(Self::mapping_path(repo_path));
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

/// La ruta del repo tal como la recibe el proveedor: absoluta y canónica. Una
/// ruta relativa (`--project-root .`) se interpreta en el cwd del proceso del
/// proveedor, y Codebase Memory nombra el proyecto por su texto.
fn provider_repo_path(repo_path: &str) -> String {
    canonical_or_original(Path::new(repo_path))
        .to_string_lossy()
        .to_string()
}

impl Drop for CodebaseMemoryClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Falla de una llamada al proveedor, antes de convertirse en
/// `ProviderResult`: el tipo de dato de la respuesta final varía por método.
enum CallFailure {
    Unavailable(String),
    Degraded(String),
}

impl CallFailure {
    fn into_result<T>(self, provider: &str) -> ProviderResult<T> {
        match self {
            CallFailure::Unavailable(warning) => ProviderResult::unavailable(provider, warning),
            CallFailure::Degraded(warning) => ProviderResult::degraded(provider, warning),
        }
    }
}

const PROVIDER_NAME: &str = "codebase-memory";
const NEIGHBOR_FETCH_LIMIT: usize = 200;
const SNIPPET_MAX_CHARS: usize = 4000;

/// Literal de string Cypher seguro, o `None`. Los valores vienen de targets
/// declarados por agentes: en vez de apostar a una regla de escape que el
/// parser de Codebase Memory no documenta, se rechaza todo lo que podría
/// cerrar el literal o partir la consulta.
fn cypher_literal(value: &str) -> Option<String> {
    let unsafe_char = |c: char| c == '\'' || c == '\\' || c == '"' || c.is_control();
    (!value.is_empty() && !value.chars().any(unsafe_char)).then(|| format!("'{value}'"))
}

/// Tipo de arista de Codebase Memory → modelo de Rationale.
fn relation_kind_from_provider(label: &str) -> RelationKind {
    match label {
        "CALLS" => RelationKind::Calls,
        "USAGE" => RelationKind::Uses,
        "WRITES" => RelationKind::Writes,
        "IMPORTS" => RelationKind::Imports,
        "DEFINES" | "DEFINES_METHOD" => RelationKind::Defines,
        "IMPLEMENTS" => RelationKind::Implements,
        "TESTS" | "TESTS_FILE" => RelationKind::Tests,
        "CONFIGURES" => RelationKind::Configures,
        "DEPENDS_ON" => RelationKind::DependsOn,
        "HTTP_CALLS" => RelationKind::HttpCalls,
        "CONTAINS_FOLDER" | "CONTAINS_FILE" => RelationKind::Contains,
        "DECORATES" => RelationKind::Decorates,
        "SEMANTICALLY_RELATED" => RelationKind::SemanticallyRelated,
        "SIMILAR_TO" => RelationKind::SimilarTo,
        _ => RelationKind::Other,
    }
}

/// Modelo de Rationale → tipo de arista principal de Codebase Memory, para
/// consultas de caminos tipados.
fn provider_edge_label(kind: RelationKind) -> Option<&'static str> {
    Some(match kind {
        RelationKind::Calls => "CALLS",
        RelationKind::Uses => "USAGE",
        RelationKind::Writes => "WRITES",
        RelationKind::Imports => "IMPORTS",
        RelationKind::Defines => "DEFINES",
        RelationKind::Implements => "IMPLEMENTS",
        RelationKind::Tests => "TESTS",
        RelationKind::Configures => "CONFIGURES",
        RelationKind::DependsOn => "DEPENDS_ON",
        RelationKind::HttpCalls => "HTTP_CALLS",
        RelationKind::Decorates => "DECORATES",
        RelationKind::Contains
        | RelationKind::SemanticallyRelated
        | RelationKind::SimilarTo
        | RelationKind::Other => return None,
    })
}

/// Quita el prefijo de proyecto que Codebase Memory antepone a cada nombre
/// calificado. El prefijo es el identificador de proyecto que el propio
/// proveedor devolvió (`list_projects`), no una reimplementación de su
/// algoritmo de nombres.
fn portable_qualified_name(project: &str, qualified_name: &str) -> String {
    qualified_name
        .strip_prefix(project)
        .and_then(|rest| rest.strip_prefix('.'))
        .unwrap_or(qualified_name)
        .to_string()
}

fn provider_qualified_name(project: &str, portable: &str) -> String {
    if portable.starts_with(&format!("{project}.")) {
        portable.to_string()
    } else {
        format!("{project}.{portable}")
    }
}

fn cell_str(row: &[Value], index: usize) -> Option<String> {
    match row.get(index)? {
        Value::String(text) if !text.is_empty() => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

fn cell_u32(row: &[Value], index: usize) -> Option<u32> {
    cell_str(row, index)?.parse().ok().filter(|line| *line > 0)
}

/// Columnas estándar de nodo en las consultas de este adaptador:
/// name, qualified_name, label, file_path, start_line, end_line, is_test.
const NODE_COLUMNS: &str =
    "n.name, n.qualified_name, n.label, n.file_path, n.start_line, n.end_line, n.is_test";

fn node_columns(alias: &str) -> String {
    NODE_COLUMNS.replace("n.", &format!("{alias}."))
}

fn node_from_row(project: &str, row: &[Value], offset: usize) -> Option<StructuralNode> {
    let qualified_name = cell_str(row, offset + 1)?;
    let file_path = cell_str(row, offset + 3)?;
    let label = cell_str(row, offset + 2)
        .unwrap_or_else(|| "unknown".to_string())
        .to_ascii_lowercase();
    let binding = NodeBinding {
        file_path: file_path.replace('\\', "/"),
        qualified_name: portable_qualified_name(project, &qualified_name),
        symbol_kind: Some(label.clone()),
    };
    Some(StructuralNode {
        key: binding.key(project_key()),
        name: cell_str(row, offset).unwrap_or_else(|| binding.short_name().to_string()),
        label,
        start_line: cell_u32(row, offset + 4),
        end_line: cell_u32(row, offset + 5),
        signature: None,
        is_test: cell_str(row, offset + 6).as_deref() == Some("true"),
        binding,
    })
}

/// Las claves de nodo son independientes del proveedor y de la máquina:
/// se derivan de la identidad portable, nunca del nombre de proyecto de CBM.
fn project_key() -> &'static str {
    "rationale"
}

impl CodebaseMemoryClient {
    fn project_or_failure(&mut self, repo_path: &str) -> Result<String, CallFailure> {
        if let Some(project) = self.validated_projects.get(repo_path) {
            return Ok(project.clone());
        }
        let project = match Self::load_project_mapping(repo_path) {
            Some(project) if self.project_exists(&project)? => project,
            // El vínculo en disco puede sobrevivir al índice del proveedor
            // (visto en vivo: Codebase Memory perdió el proyecto de este repo
            // y cada consulta quedaba degradada para siempre). Se olvida y la
            // identidad se resuelve como la primera vez.
            Some(_) => {
                Self::forget_project_mapping(repo_path);
                self.resolve_project(repo_path)?
            }
            None => self.resolve_project(repo_path)?,
        };
        self.validated_projects
            .insert(repo_path.to_string(), project.clone());
        Ok(project)
    }

    /// Primera resolución: `list_projects` por raíz y, solo si el proveedor
    /// confirma que el repo no está, `index_repository`. Nunca se reindexa un
    /// proyecto existente (`decision.provider-owned-project-identity`).
    fn resolve_project(&mut self, repo_path: &str) -> Result<String, CallFailure> {
        if let Some(project) = self.project_from_listing(repo_path)? {
            return Ok(project);
        }
        let project = match self.index_project(repo_path) {
            Err(()) => {
                return Err(CallFailure::Unavailable(
                    "provider no respondió durante la indexación inicial".to_string(),
                ))
            }
            Ok(Some(project)) => project,
            Ok(None) => {
                return Err(CallFailure::Degraded(
                    "Codebase Memory no devolvió una identidad de proyecto tras indexar"
                        .to_string(),
                ))
            }
        };
        // Visto en vivo: `index_repository` con una ruta relativa devolvió el
        // proyecto `root`, que nunca pudo consultarse. Una identidad se
        // confirma antes de recordarla.
        if !self.project_exists(&project)? {
            return Err(CallFailure::Degraded(format!(
                "Codebase Memory devolvió el proyecto '{project}' tras indexar, pero todavía no \
                 puede consultarlo"
            )));
        }
        let _ = Self::save_project_mapping(repo_path, &project);
        Ok(project)
    }

    /// ¿Sigue existiendo un proyecto recordado? Solo un «not found» explícito
    /// cuenta como ausencia: un error ambiguo nunca borra el vínculo.
    fn project_exists(&mut self, project: &str) -> Result<bool, CallFailure> {
        let response = self
            .call_tool("index_status", json!({"project": project}))
            .ok_or_else(|| {
                CallFailure::Unavailable("provider no respondió dentro del deadline".to_string())
            })?;
        Ok(!Self::is_missing_project_response(&response))
    }

    /// Codebase Memory responde `isError: true` con un JSON de error que llega
    /// truncado (arrastra la lista de proyectos disponibles), así que se
    /// reconoce por su mensaje en vez de parsearlo.
    fn is_missing_project_response(response: &Value) -> bool {
        let Some(result) = response.get("result") else {
            return false;
        };
        let is_error = result.get("isError").and_then(Value::as_bool) == Some(true);
        let text = result
            .get("content")
            .and_then(Value::as_array)
            .and_then(|content| content.first())
            .and_then(|item| item.get("text"))
            .and_then(Value::as_str)
            .unwrap_or("");
        is_error && text.contains("project not found")
    }

    fn tool_json(&mut self, name: &str, arguments: Value) -> Result<Value, CallFailure> {
        let response = self.call_tool(name, arguments).ok_or_else(|| {
            CallFailure::Unavailable("provider no respondió dentro del deadline".to_string())
        })?;
        Self::extract_tool_json(&response).ok_or_else(|| {
            CallFailure::Degraded(format!("respuesta inesperada de Codebase Memory en {name}"))
        })
    }

    fn cypher_rows(
        &mut self,
        project: &str,
        query: String,
    ) -> Result<Vec<Vec<Value>>, CallFailure> {
        let payload = self.tool_json("query_graph", json!({"project": project, "query": query}))?;
        if payload.get("error").is_some() {
            return Err(CallFailure::Degraded(
                "consulta estructural rechazada por el proveedor".to_string(),
            ));
        }
        Ok(payload
            .get("rows")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(|row| row.as_array().cloned())
                    .collect()
            })
            .unwrap_or_default())
    }

    fn literal_for(project: &str, node: &NodeBinding) -> Result<String, CallFailure> {
        cypher_literal(&provider_qualified_name(project, &node.qualified_name)).ok_or_else(|| {
            CallFailure::Degraded(format!(
                "'{}' contiene caracteres que no se consultan de forma segura",
                node.qualified_name
            ))
        })
    }

    fn neighbors_in_direction(
        &mut self,
        project: &str,
        node: &NodeBinding,
        outbound: bool,
        query: &NeighborQuery,
        subgraph: &mut StructuralSubgraph,
    ) -> Result<(), CallFailure> {
        let literal = Self::literal_for(project, node)?;
        let pattern = if outbound {
            "MATCH (a)-[r]->(n)"
        } else {
            "MATCH (n)-[r]->(a)"
        };
        let rows = self.cypher_rows(
            project,
            format!(
                "{pattern} WHERE a.qualified_name = {literal} RETURN type(r), {NODE_COLUMNS} LIMIT {NEIGHBOR_FETCH_LIMIT}"
            ),
        )?;
        if rows.len() >= NEIGHBOR_FETCH_LIMIT {
            subgraph.truncated = true;
        }
        let mut accepted = 0usize;
        for row in rows {
            let Some(kind) =
                cell_str(&row, 0).map(|label| (relation_kind_from_provider(&label), label))
            else {
                continue;
            };
            if !query.accepts(kind.0) {
                continue;
            }
            let Some(other) = node_from_row(project, &row, 1) else {
                continue;
            };
            if crate::providers::LOW_SIGNAL_LABELS.contains(&other.label.as_str()) {
                continue;
            }
            // Una arista `DEFINES` saliente de un archivo trae todo su
            // contenido: solo aporta contexto cuando el nodo consultado es
            // el definido, no el contenedor.
            if kind.0 == RelationKind::Defines && outbound {
                continue;
            }
            accepted += 1;
            if accepted > query.limit {
                subgraph.truncated = true;
                continue;
            }
            let (source, target) = if outbound {
                (node.clone(), other.binding.clone())
            } else {
                (other.binding.clone(), node.clone())
            };
            let relationship = StructuralRelationship {
                key: StructuralRelationship::relationship_key(
                    project_key(),
                    &source,
                    kind.0,
                    &target,
                ),
                source,
                kind: kind.0,
                target,
                provider_kind: kind.1,
            };
            subgraph.merge(StructuralSubgraph {
                nodes: vec![other],
                relationships: vec![relationship],
                truncated: false,
            });
        }
        Ok(())
    }
}

impl CodeIntelligenceProvider for CodebaseMemoryClient {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            name: PROVIDER_NAME.to_string(),
            neighbors: true,
            bounded_paths: true,
            max_path_hops: 3,
            code_snippets: true,
            file_outline: true,
            architecture: true,
            changed_nodes: true,
        }
    }

    fn health(&mut self, repo_path: &str) -> ProviderResult<()> {
        let project = match self.project_or_failure(repo_path) {
            Ok(project) => project,
            Err(failure) => return failure.into_result(&self.binary),
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

    fn index_status(&mut self, repo_path: &str) -> ProviderResult<IndexStatus> {
        let project = match self.project_or_failure(repo_path) {
            Ok(project) => project,
            Err(failure) => return failure.into_result(PROVIDER_NAME),
        };
        match self.tool_json("index_status", json!({"project": project})) {
            Err(failure) => failure.into_result(PROVIDER_NAME),
            Ok(payload) => ProviderResult::ok(
                PROVIDER_NAME,
                IndexStatus {
                    project: Some(project),
                    state: payload
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                        .to_string(),
                    nodes: payload.get("nodes").and_then(Value::as_u64),
                    edges: payload.get("edges").and_then(Value::as_u64),
                },
                Coverage::Complete,
            ),
        }
    }

    /// Delegado en `resolve_node`: resolución exacta dentro del archivo
    /// declarado (`decision.provider-owned-project-identity`). La versión
    /// anterior buscaba por patrón y, sin coincidencia exacta entre los
    /// primeros 20 resultados, devolvía el primero: en el índice real de este
    /// repo, `src/pipeline.rs::prepare` resolvía a una sección Markdown del
    /// sitio, con el prefijo de proyecto de la máquina incluido.
    fn resolve_target(
        &mut self,
        repo_path: &str,
        file_path: &str,
        symbol_name: &str,
    ) -> ProviderResult<ResolvedTarget> {
        let result = self.resolve_node(repo_path, file_path, symbol_name);
        ProviderResult {
            data: result.data.map(|node| ResolvedTarget {
                qualified_name: node.binding.qualified_name,
                file_path: node.binding.file_path,
            }),
            provider_name: result.provider_name,
            status: result.status,
            coverage: result.coverage,
            warnings: result.warnings,
        }
    }

    /// Resolución exacta dentro del archivo declarado
    /// (`decision.provider-owned-project-identity`): por nombre y ruta, nunca
    /// "el primer resultado" de una búsqueda por patrón. `Tipo::metodo` se
    /// desambigua por el sufijo del nombre calificado.
    fn resolve_node(
        &mut self,
        repo_path: &str,
        file_path: &str,
        symbol_name: &str,
    ) -> ProviderResult<StructuralNode> {
        let project = match self.project_or_failure(repo_path) {
            Ok(project) => project,
            Err(failure) => return failure.into_result(PROVIDER_NAME),
        };
        let segments: Vec<&str> = symbol_name.split("::").filter(|s| !s.is_empty()).collect();
        let Some(name) = segments.last() else {
            return ProviderResult::not_found(PROVIDER_NAME, "símbolo vacío");
        };
        let (Some(file_literal), Some(name_literal)) =
            (cypher_literal(file_path), cypher_literal(name))
        else {
            return ProviderResult::degraded(
                PROVIDER_NAME,
                "el target contiene caracteres que no se consultan de forma segura",
            );
        };
        let rows = match self.cypher_rows(
            &project,
            format!(
                "MATCH (n) WHERE n.file_path = {file_literal} AND n.name = {name_literal} RETURN {NODE_COLUMNS} LIMIT 20"
            ),
        ) {
            Ok(rows) => rows,
            Err(failure) => return failure.into_result(PROVIDER_NAME),
        };
        let candidates: Vec<StructuralNode> = rows
            .iter()
            .filter_map(|row| node_from_row(&project, row, 0))
            .collect();
        let dotted_suffix = format!(".{}", segments.join("."));
        let exact: Vec<&StructuralNode> = candidates
            .iter()
            .filter(|node| {
                node.binding.qualified_name.ends_with(&dotted_suffix)
                    || node.binding.qualified_name == segments.join(".")
            })
            .collect();
        match (exact.as_slice(), candidates.as_slice()) {
            ([single], _) => ProviderResult::ok(PROVIDER_NAME, (*single).clone(), Coverage::Complete),
            ([], [single]) => ProviderResult::ok(PROVIDER_NAME, single.clone(), Coverage::Complete),
            ([], []) => ProviderResult::not_found(
                PROVIDER_NAME,
                "no se encontró el símbolo dentro de la cobertura disponible; no implica que no exista",
            ),
            _ => ProviderResult::not_found(
                PROVIDER_NAME,
                format!(
                    "'{symbol_name}' es ambiguo en '{file_path}' ({} candidatos) — usa Tipo::metodo",
                    candidates.len()
                ),
            ),
        }
    }

    fn get_node(&mut self, repo_path: &str, node: &NodeBinding) -> ProviderResult<StructuralNode> {
        let project = match self.project_or_failure(repo_path) {
            Ok(project) => project,
            Err(failure) => return failure.into_result(PROVIDER_NAME),
        };
        let literal = match Self::literal_for(&project, node) {
            Ok(literal) => literal,
            Err(failure) => return failure.into_result(PROVIDER_NAME),
        };
        match self.cypher_rows(
            &project,
            format!("MATCH (n) WHERE n.qualified_name = {literal} RETURN {NODE_COLUMNS} LIMIT 1"),
        ) {
            Err(failure) => failure.into_result(PROVIDER_NAME),
            Ok(rows) => match rows.first().and_then(|row| node_from_row(&project, row, 0)) {
                Some(found) => ProviderResult::ok(PROVIDER_NAME, found, Coverage::Complete),
                None => ProviderResult::not_found(
                    PROVIDER_NAME,
                    "el nodo no está en el índice actual; no implica que no exista",
                ),
            },
        }
    }

    fn get_neighbors(
        &mut self,
        repo_path: &str,
        node: &NodeBinding,
        query: &NeighborQuery,
    ) -> ProviderResult<StructuralSubgraph> {
        let project = match self.project_or_failure(repo_path) {
            Ok(project) => project,
            Err(failure) => return failure.into_result(PROVIDER_NAME),
        };
        let seed = self.get_node(repo_path, node);
        let mut subgraph = StructuralSubgraph {
            nodes: seed.data.into_iter().collect(),
            ..Default::default()
        };
        if subgraph.nodes.is_empty() {
            return ProviderResult::not_found(
                PROVIDER_NAME,
                format!("'{}' no está en el índice actual", node.qualified_name),
            );
        }
        let directions: &[bool] = match query.direction {
            Direction::Outbound => &[true],
            Direction::Inbound => &[false],
            Direction::Both => &[true, false],
        };
        for outbound in directions {
            if let Err(failure) =
                self.neighbors_in_direction(&project, node, *outbound, query, &mut subgraph)
            {
                return failure.into_result(PROVIDER_NAME);
            }
        }
        ProviderResult::ok(PROVIDER_NAME, subgraph, Coverage::Complete)
    }

    fn get_relationships(
        &mut self,
        repo_path: &str,
        source: &NodeBinding,
        target: &NodeBinding,
    ) -> ProviderResult<Vec<StructuralRelationship>> {
        let project = match self.project_or_failure(repo_path) {
            Ok(project) => project,
            Err(failure) => return failure.into_result(PROVIDER_NAME),
        };
        let (source_literal, target_literal) = match (
            Self::literal_for(&project, source),
            Self::literal_for(&project, target),
        ) {
            (Ok(s), Ok(t)) => (s, t),
            (Err(failure), _) | (_, Err(failure)) => return failure.into_result(PROVIDER_NAME),
        };
        let mut relationships = Vec::new();
        for (from, to, from_binding, to_binding) in [
            (&source_literal, &target_literal, source, target),
            (&target_literal, &source_literal, target, source),
        ] {
            let rows = match self.cypher_rows(
                &project,
                format!(
                    "MATCH (a)-[r]->(b) WHERE a.qualified_name = {from} AND b.qualified_name = {to} RETURN type(r) LIMIT 20"
                ),
            ) {
                Ok(rows) => rows,
                Err(failure) => return failure.into_result(PROVIDER_NAME),
            };
            for row in rows {
                if let Some(label) = cell_str(&row, 0) {
                    let kind = relation_kind_from_provider(&label);
                    relationships.push(StructuralRelationship {
                        key: StructuralRelationship::relationship_key(
                            project_key(),
                            from_binding,
                            kind,
                            to_binding,
                        ),
                        source: from_binding.clone(),
                        kind,
                        target: to_binding.clone(),
                        provider_kind: label,
                    });
                }
            }
        }
        ProviderResult::ok(PROVIDER_NAME, relationships, Coverage::Complete)
    }

    /// Caminos acotados vía patrones tipados de longitud fija: `calls`
    /// intermedios y el tipo pedido al final. Se prueba de 1 a `max_hops` y
    /// se devuelven los más cortos — verificado contra CBM 0.8.1 que los
    /// patrones `(a)-[r1:CALLS]->(m1)-[r2:USAGE]->(b)` se ejecutan.
    fn find_paths(
        &mut self,
        repo_path: &str,
        source: &NodeBinding,
        target: &NodeBinding,
        kind: RelationKind,
        max_hops: usize,
    ) -> ProviderResult<Vec<StructuralPath>> {
        let Some(final_label) = provider_edge_label(kind) else {
            return ProviderResult::unsupported(
                PROVIDER_NAME,
                "find_paths para este tipo de relación",
            );
        };
        let project = match self.project_or_failure(repo_path) {
            Ok(project) => project,
            Err(failure) => return failure.into_result(PROVIDER_NAME),
        };
        let (source_literal, target_literal) = match (
            Self::literal_for(&project, source),
            Self::literal_for(&project, target),
        ) {
            (Ok(s), Ok(t)) => (s, t),
            (Err(failure), _) | (_, Err(failure)) => return failure.into_result(PROVIDER_NAME),
        };
        for hops in 1..=max_hops.min(self.capabilities().max_path_hops) {
            let mut pattern = String::from("MATCH (a)");
            for hop in 1..=hops {
                let label = if hop == hops { final_label } else { "CALLS" };
                let next = if hop == hops {
                    "(b)".to_string()
                } else {
                    format!("(m{hop})")
                };
                pattern.push_str(&format!("-[r{hop}:{label}]->{next}"));
            }
            let returns: Vec<String> = (1..hops).map(|m| node_columns(&format!("m{m}"))).collect();
            let return_clause = if returns.is_empty() {
                "type(r1)".to_string()
            } else {
                returns.join(", ")
            };
            let rows = match self.cypher_rows(
                &project,
                format!(
                    "{pattern} WHERE a.qualified_name = {source_literal} AND b.qualified_name = {target_literal} RETURN {return_clause} LIMIT 5"
                ),
            ) {
                Ok(rows) => rows,
                Err(failure) => return failure.into_result(PROVIDER_NAME),
            };
            if rows.is_empty() {
                continue;
            }
            let paths = rows
                .iter()
                .filter_map(|row| {
                    let mut nodes = vec![source.clone()];
                    for m in 0..hops.saturating_sub(1) {
                        nodes.push(node_from_row(&project, row, m * 7)?.binding);
                    }
                    nodes.push(target.clone());
                    let mut kinds = vec![RelationKind::Calls; hops - 1];
                    kinds.push(kind);
                    Some(StructuralPath { nodes, kinds })
                })
                .collect();
            return ProviderResult::ok(PROVIDER_NAME, paths, Coverage::Complete);
        }
        ProviderResult::ok(PROVIDER_NAME, vec![], Coverage::Complete)
    }

    fn get_code_snippet(
        &mut self,
        repo_path: &str,
        node: &NodeBinding,
    ) -> ProviderResult<CodeSnippet> {
        let project = match self.project_or_failure(repo_path) {
            Ok(project) => project,
            Err(failure) => return failure.into_result(PROVIDER_NAME),
        };
        let qualified_name = provider_qualified_name(&project, &node.qualified_name);
        match self.tool_json(
            "get_code_snippet",
            json!({"project": project, "qualified_name": qualified_name}),
        ) {
            Err(failure) => failure.into_result(PROVIDER_NAME),
            Ok(payload) => match payload.get("source").and_then(Value::as_str) {
                None => ProviderResult::not_found(
                    PROVIDER_NAME,
                    "el proveedor no devolvió source para este nodo",
                ),
                Some(source) => {
                    let truncated = source.chars().count() > SNIPPET_MAX_CHARS;
                    ProviderResult::ok(
                        PROVIDER_NAME,
                        CodeSnippet {
                            binding: node.clone(),
                            start_line: payload
                                .get("start_line")
                                .and_then(Value::as_u64)
                                .map(|l| l as u32),
                            end_line: payload
                                .get("end_line")
                                .and_then(Value::as_u64)
                                .map(|l| l as u32),
                            source: source.chars().take(SNIPPET_MAX_CHARS).collect(),
                            truncated,
                        },
                        Coverage::Complete,
                    )
                }
            },
        }
    }

    fn get_file_outline(
        &mut self,
        repo_path: &str,
        file_path: &str,
    ) -> ProviderResult<Vec<StructuralNode>> {
        let project = match self.project_or_failure(repo_path) {
            Ok(project) => project,
            Err(failure) => return failure.into_result(PROVIDER_NAME),
        };
        let Some(file_literal) = cypher_literal(file_path) else {
            return ProviderResult::degraded(PROVIDER_NAME, "ruta con caracteres no consultables");
        };
        match self.cypher_rows(
            &project,
            format!("MATCH (n) WHERE n.file_path = {file_literal} RETURN {NODE_COLUMNS} LIMIT 500"),
        ) {
            Err(failure) => failure.into_result(PROVIDER_NAME),
            Ok(rows) => {
                let mut nodes: Vec<StructuralNode> = rows
                    .iter()
                    .filter_map(|row| node_from_row(&project, row, 0))
                    .filter(|node| {
                        !crate::providers::LOW_SIGNAL_LABELS.contains(&node.label.as_str())
                            && node.label != "file"
                    })
                    .collect();
                nodes.sort_by_key(|node| node.start_line.unwrap_or(0));
                ProviderResult::ok(PROVIDER_NAME, nodes, Coverage::Complete)
            }
        }
    }

    fn get_architecture(&mut self, repo_path: &str) -> ProviderResult<ArchitectureSummary> {
        let project = match self.project_or_failure(repo_path) {
            Ok(project) => project,
            Err(failure) => return failure.into_result(PROVIDER_NAME),
        };
        match self.tool_json(
            "get_architecture",
            json!({"project": project, "aspects": ["clusters"]}),
        ) {
            Err(failure) => failure.into_result(PROVIDER_NAME),
            Ok(payload) => ProviderResult::ok(
                PROVIDER_NAME,
                ArchitectureSummary {
                    total_nodes: payload.get("total_nodes").and_then(Value::as_u64),
                    total_edges: payload.get("total_edges").and_then(Value::as_u64),
                    clusters: payload
                        .get("clusters")
                        .and_then(Value::as_array)
                        .map(|clusters| {
                            clusters
                                .iter()
                                .map(|cluster| ArchitectureCluster {
                                    label: cluster
                                        .get("label")
                                        .and_then(Value::as_str)
                                        .unwrap_or("")
                                        .to_string(),
                                    members: cluster
                                        .get("members")
                                        .and_then(Value::as_u64)
                                        .unwrap_or(0),
                                    top_nodes: cluster
                                        .get("top_nodes")
                                        .and_then(Value::as_array)
                                        .map(|nodes| {
                                            nodes
                                                .iter()
                                                .filter_map(Value::as_str)
                                                .map(str::to_string)
                                                .collect()
                                        })
                                        .unwrap_or_default(),
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                },
                Coverage::Complete,
            ),
        }
    }

    /// `detect_changes` de CBM informa archivos cambiados y símbolos sin
    /// nombre calificado; los nodos se obtienen del outline de cada archivo
    /// (acotado) para conservar identidad normalizada.
    fn changed_nodes(
        &mut self,
        repo_path: &str,
        since: &str,
    ) -> ProviderResult<Vec<StructuralNode>> {
        let project = match self.project_or_failure(repo_path) {
            Ok(project) => project,
            Err(failure) => return failure.into_result(PROVIDER_NAME),
        };
        let payload = match self.tool_json(
            "detect_changes",
            json!({"project": project, "since": since, "depth": 1}),
        ) {
            Ok(payload) => payload,
            Err(failure) => return failure.into_result(PROVIDER_NAME),
        };
        let files: Vec<String> = payload
            .get("changed_files")
            .and_then(Value::as_array)
            .map(|files| {
                files
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        let mut nodes = Vec::new();
        let mut warnings = Vec::new();
        for file in files.iter().take(20) {
            let outline = self.get_file_outline(repo_path, file);
            warnings.extend(outline.warnings);
            nodes.extend(outline.data.unwrap_or_default());
        }
        if files.len() > 20 {
            warnings.push(format!(
                "{} archivos cambiados; solo se resolvieron 20",
                files.len()
            ));
        }
        let mut result = ProviderResult::ok(PROVIDER_NAME, nodes, Coverage::Complete);
        result.warnings = warnings;
        result
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
    #[test]
    fn cypher_literals_reject_anything_that_could_escape_the_string() {
        assert_eq!(
            cypher_literal("src.pipeline.prepare"),
            Some("'src.pipeline.prepare'".to_string())
        );
        assert_eq!(
            cypher_literal("Type::method<T>"),
            Some("'Type::method<T>'".to_string())
        );
        for hostile in ["x' OR 1=1 //", "a\\b", "multi\nline", "", "quote\"d"] {
            assert_eq!(cypher_literal(hostile), None, "{hostile:?}");
        }
    }

    #[test]
    fn qualified_names_become_portable_and_back() {
        let project = "Users-someone-Desktop-Rationale";
        let provider = "Users-someone-Desktop-Rationale.src.storage.is_revoked";
        let portable = portable_qualified_name(project, provider);
        assert_eq!(portable, "src.storage.is_revoked");
        assert_eq!(provider_qualified_name(project, &portable), provider);
        assert_eq!(
            provider_qualified_name(project, provider),
            provider,
            "idempotente"
        );
        assert_eq!(
            portable_qualified_name(project, "OtherProject.src.x"),
            "OtherProject.src.x",
            "un prefijo ajeno nunca se recorta"
        );
    }

    #[test]
    fn provider_edge_labels_map_both_ways() {
        for kind in RelationKind::STRUCTURAL {
            let label = provider_edge_label(kind).unwrap();
            assert_eq!(relation_kind_from_provider(label), kind);
        }
        assert_eq!(
            relation_kind_from_provider("DEFINES_METHOD"),
            RelationKind::Defines
        );
        assert_eq!(
            relation_kind_from_provider("SOMETHING_NEW"),
            RelationKind::Other
        );
        assert!(provider_edge_label(RelationKind::SimilarTo).is_none());
    }

    #[test]
    fn node_rows_normalize_identity_and_types() {
        let row = vec![
            json!("is_revoked"),
            json!("Users-x-Rationale.src.storage.is_revoked"),
            json!("Function"),
            json!("src/storage.rs"),
            json!("631"),
            json!("633"),
            json!("false"),
        ];
        let node = node_from_row("Users-x-Rationale", &row, 0).unwrap();
        assert_eq!(node.binding.qualified_name, "src.storage.is_revoked");
        assert_eq!(node.label, "function");
        assert_eq!(node.start_line, Some(631));
        assert!(!node.is_test);
        assert_eq!(
            node.key,
            NodeBinding {
                file_path: "src/storage.rs".to_string(),
                qualified_name: "src.storage.is_revoked".to_string(),
                symbol_kind: None,
            }
            .key("rationale"),
            "la clave no depende del nombre de proyecto del proveedor"
        );
    }

    const MOCK_PRELUDE: &str = r##"#!/usr/bin/env bash
set -euo pipefail
read_message() {
  local content_length=0
  local line
  while IFS= read -r line; do
    line="${line%$'\r'}"
    [ -z "$line" ] && break
    if [[ "$line" =~ ^Content-Length:\ *([0-9]+)$ ]]; then
      content_length="${BASH_REMATCH[1]}"
    fi
  done
  if [ "$content_length" -gt 0 ]; then
    dd bs=1 count="$content_length" 2>/dev/null
  fi
}
write_message() {
  local body="$1"
  printf 'Content-Length: %d\r\n\r\n%s' "${#body}" "$body"
}
message_id() {
  echo "$1" | grep -o '"id"[[:space:]]*:[[:space:]]*[0-9]*' | grep -o '[0-9]*$'
}
not_found() {
  write_message '{"jsonrpc":"2.0","id":'"$1"',"result":{"content":[{"type":"text","text":"{\"error\":\"project not found or not indexed\",\"available_projects\":[\"a\",\"b"}],"isError":true}}'
}
tool_text() {
  write_message '{"jsonrpc":"2.0","id":'"$1"',"result":{"content":[{"type":"text","text":"'"$2"'"}]}}'
}
id="$(message_id "$(read_message)")"
write_message '{"jsonrpc":"2.0","id":'"$id"',"result":{"protocolVersion":"2024-11-05","capabilities":{},"serverInfo":{"name":"mock","version":"0.0.0"}}}'
read_message >/dev/null
"##;

    /// index_status(recordado) → not found; list_projects → el repo con otro
    /// nombre; index_status(nuevo) → ready.
    const STALE_PROJECT_BODY: &str = r##"id="$(message_id "$(read_message)")"; not_found "$id"
id="$(message_id "$(read_message)")"; tool_text "$id" '{\"projects\":[{\"name\":\"renamed-project\",\"root_path\":\"__ROOT__\"}]}'
id="$(message_id "$(read_message)")"; tool_text "$id" '{\"status\":\"ready\"}'
"##;

    /// index_status(recordado) → not found; list_projects → vacío;
    /// index_repository → `root` solo si llegó una ruta absoluta;
    /// index_status(root) → not found (la identidad no es consultable).
    const UNCONFIRMED_INDEX_BODY: &str = r##"id="$(message_id "$(read_message)")"; not_found "$id"
id="$(message_id "$(read_message)")"; tool_text "$id" '{\"projects\":[]}'
msg="$(read_message)"; id="$(message_id "$msg")"
case "$msg" in
  *'"repo_path":"/'*) project=root ;;
  *) project=relative-path-sent ;;
esac
tool_text "$id" "{\\\"project\\\":\\\"$project\\\"}"
id="$(message_id "$(read_message)")"; not_found "$id"
"##;

    #[cfg(unix)]
    fn mock_repo(label: &str) -> std::path::PathBuf {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!(
                "rationale-mock-repo-{label}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        std::fs::create_dir_all(repo.join(".rationale")).unwrap();
        repo
    }

    #[cfg(unix)]
    fn spawn_mock(dir: &Path, body: &str) -> CodebaseMemoryClient {
        use std::os::unix::fs::PermissionsExt;
        let script = dir.join("mock-cbm.sh");
        std::fs::write(&script, format!("{MOCK_PRELUDE}{body}")).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        CodebaseMemoryClient::spawn_with(
            &script.to_string_lossy(),
            Duration::from_secs(5),
            Duration::from_secs(5),
        )
        .expect("el mock debe inicializar")
    }

    /// Defecto real (dogfood de vNext): Codebase Memory perdió el proyecto de
    /// este repo y el vínculo guardado seguía nombrándolo, así que cada
    /// consulta quedaba degradada para siempre. El vínculo se valida una vez
    /// por sesión, se olvida ante un «not found» explícito (cuyo JSON llega
    /// truncado) y la identidad se resuelve de nuevo por `list_projects`.
    #[cfg(unix)]
    #[test]
    fn a_stale_project_mapping_is_forgotten_and_resolved_again() {
        let repo = mock_repo("stale");
        let repo_path = repo.to_string_lossy().to_string();
        CodebaseMemoryClient::save_project_mapping(&repo_path, "gone-project").unwrap();
        let root = canonical_or_original(&repo).to_string_lossy().to_string();
        let mut client = spawn_mock(&repo, &STALE_PROJECT_BODY.replace("__ROOT__", &root));
        let result = client.health(&repo_path);
        assert!(
            matches!(result.status, ProviderStatus::Successful),
            "{:?}",
            result.warnings
        );
        assert_eq!(
            CodebaseMemoryClient::load_project_mapping(&repo_path).as_deref(),
            Some("renamed-project"),
            "el vínculo nuevo reemplaza al obsoleto"
        );
        std::fs::remove_dir_all(&repo).ok();
    }

    /// Defecto real del mismo dogfood: con `--project-root .` la indexación de
    /// primer uso envió `.` a `index_repository`; Codebase Memory devolvió el
    /// proyecto `root`, que nunca pudo consultarse, y quedó recordado.
    #[cfg(unix)]
    #[test]
    fn indexing_sends_an_absolute_path_and_never_remembers_an_unqueryable_project() {
        let repo = mock_repo("unconfirmed");
        let relative = repo
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap()
            .to_string_lossy()
            .to_string();
        CodebaseMemoryClient::save_project_mapping(&relative, "gone-project").unwrap();
        let mut client = spawn_mock(&repo, UNCONFIRMED_INDEX_BODY);
        let result = client.health(&relative);
        assert!(
            matches!(result.status, ProviderStatus::Degraded),
            "{:?}",
            result.warnings
        );
        assert!(
            result.warnings.iter().any(|w| w.contains("'root'")),
            "index_repository debe recibir una ruta absoluta: {:?}",
            result.warnings
        );
        assert_eq!(
            CodebaseMemoryClient::load_project_mapping(&relative),
            None,
            "nunca se recuerda un proyecto que no se puede consultar"
        );
        std::fs::remove_dir_all(&repo).ok();
    }

    #[test]
    fn relative_provider_root_paths_never_identify_a_repo() {
        let here = env!("CARGO_MANIFEST_DIR");
        let absolute = canonical_or_original(Path::new(here));
        let mixed = json!({"projects": [
            {"name": "root", "root_path": "."},
            {"name": "real", "root_path": absolute}
        ]});
        assert_eq!(
            CodebaseMemoryClient::project_from_list(&mixed, here).as_deref(),
            Some("real")
        );
        let only_relative = json!({"projects": [{"name": "root", "root_path": "."}]});
        assert_eq!(
            CodebaseMemoryClient::project_from_list(&only_relative, here),
            None
        );
        assert!(Path::new(&provider_repo_path(".")).is_absolute());
    }

    #[test]
    fn only_an_explicit_not_found_marks_a_project_missing() {
        let truncated = json!({"result": {"isError": true, "content": [{"type": "text",
            "text": "{\"error\":\"project not found or not indexed\",\"available_projects\":[\"a\",\"b"}]}});
        assert!(CodebaseMemoryClient::is_missing_project_response(
            &truncated
        ));
        let locked = json!({"result": {"isError": true, "content": [{"type": "text",
            "text": "{\"error\":\"database is locked\"}"}]}});
        assert!(!CodebaseMemoryClient::is_missing_project_response(&locked));
        let ready =
            json!({"result": {"content": [{"type": "text", "text": "{\"status\":\"ready\"}"}]}});
        assert!(!CodebaseMemoryClient::is_missing_project_response(&ready));
    }
}
