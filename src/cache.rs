//! Capa derivada local (ADR-0004: SQLite: ADR-0005: cache root e identidad).
//!
//! "La base derivada nunca puede ser la única copia de una decisión"
//! (Arquitectura §11.7). Todo lo que este módulo guarda es 100%
//! reconstruible desde `.rationale/` — borrar el archivo `.sqlite3` nunca
//! pierde una decisión, solo obliga a recalcular.
//!
//! Invalidación por revisión (Rationale_v0.5.md §26.2): un Assessment
//! cacheado solo es válido para la revisión exacta en la que se calculó.
//! Esto se aplica como condición de la propia consulta SQL (`WHERE
//! assessed_revision = ?`), no como lógica de aplicación que podría
//! olvidarse de revisar.

use crate::assessment::Assessment;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum CacheError {
    Io(std::io::Error),
    Sqlite(rusqlite::Error),
    NoHomeDir,
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheError::Io(e) => write!(f, "error de I/O en la capa derivada: {e}"),
            CacheError::Sqlite(e) => write!(f, "error de SQLite en la capa derivada: {e}"),
            CacheError::NoHomeDir => write!(f, "no se pudo determinar el directorio HOME"),
        }
    }
}

impl From<rusqlite::Error> for CacheError {
    fn from(e: rusqlite::Error) -> Self {
        CacheError::Sqlite(e)
    }
}

/// ADR-0005: `~/.cache/rationale/projects/<ruta-sanitizada>/`. Uniforme en
/// macOS y Linux (no usa `~/Library/Caches/` nativo de macOS) — mismo
/// precedente medido en Codebase Memory (`docs/research/codebase-memory/07`).
/// Windows queda como gap documentado (`docs/dependencies/inventory.yaml`).
pub fn cache_root(project_root: &Path) -> Result<PathBuf, CacheError> {
    let home = std::env::var("HOME").map_err(|_| CacheError::NoHomeDir)?;
    let canonical = project_root
        .canonicalize()
        .unwrap_or_else(|_| project_root.to_path_buf());
    let sanitized = canonical
        .to_string_lossy()
        .trim_start_matches('/')
        .replace('/', "-");
    Ok(PathBuf::from(home)
        .join(".cache")
        .join("rationale")
        .join("projects")
        .join(sanitized))
}

/// Abre (creando si hace falta) la base derivada y aplica el schema.
/// Idempotente: puede llamarse sobre una base ya existente sin efecto.
pub fn open(cache_dir: &Path) -> Result<Connection, CacheError> {
    std::fs::create_dir_all(cache_dir).map_err(CacheError::Io)?;
    let db_path = cache_dir.join("derived.sqlite3");
    // Determinar esto ANTES de `Connection::open` (que crea un archivo
    // vacío si no existía) — es lo que evita que N conexiones concurrentes
    // contra un cache YA inicializado vuelvan a correr el DDL completo
    // innecesariamente (ver nota de busy_timeout abajo: esta era la fuente
    // real de contención, no solo la falta de timeout).
    let needs_schema = !db_path.exists();

    let conn = Connection::open(&db_path)?;
    // Sin esto, SQLite devuelve `SQLITE_BUSY` ("database is locked")
    // inmediatamente ante cualquier contención real entre conexiones
    // concurrentes (default: 0ms de espera) — descubierto por
    // `cache::tests::concurrent_reads_do_not_corrupt_cache` fallando de
    // forma intermitente bajo carga real de `cargo test` en paralelo.
    conn.busy_timeout(std::time::Duration::from_millis(15_000))?;

    if needs_schema {
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;

            CREATE TABLE IF NOT EXISTS assessments_cache (
                record_id           TEXT NOT NULL,
                assessed_revision   TEXT NOT NULL,
                revision_consistency TEXT NOT NULL,
                epistemic           TEXT NOT NULL,
                authority           TEXT NOT NULL,
                applicability        TEXT NOT NULL,
                linkage             TEXT NOT NULL,
                assessment_reason   TEXT NOT NULL,
                computed_at         TEXT NOT NULL,
                PRIMARY KEY (record_id, assessed_revision)
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS records_fts USING fts5(
                record_id UNINDEXED,
                statement,
                title
            );
            ",
        )?;
    }
    Ok(conn)
}

/// Guarda un Assessment computado, indexado por (record_id, revisión). Un
/// Assessment de una revisión distinta simplemente no sobrescribe al
/// anterior — ambos coexisten, y solo el de la revisión actual se lee.
pub fn cache_assessment(conn: &Connection, assessment: &Assessment) -> Result<(), CacheError> {
    let revision = assessment.assessed_revision.clone().unwrap_or_default();
    conn.execute(
        "INSERT OR REPLACE INTO assessments_cache
         (record_id, assessed_revision, revision_consistency, epistemic, authority, applicability, linkage, assessment_reason, computed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            assessment.record_id,
            revision,
            assessment.revision_consistency.to_string(),
            assessment.state.epistemic.to_string(),
            assessment.state.authority.to_string(),
            assessment.state.applicability.to_string(),
            assessment.state.linkage.to_string(),
            assessment.assessment_reason,
            crate::evaluation::now_iso8601(),
        ],
    )?;
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub struct CachedAssessmentReason {
    pub assessment_reason: String,
}

/// Lee un Assessment cacheado — SOLO si coincide exactamente con
/// `current_revision`. La condición de igualdad de revisión está en el
/// `WHERE` de la propia consulta: no hay ninguna ruta de código que pueda
/// "olvidarse" de invalidar por revisión, porque la fila simplemente no
/// existe para una revisión distinta (ADR-0006).
pub fn get_cached_assessment(
    conn: &Connection,
    record_id: &str,
    current_revision: &str,
) -> Result<Option<CachedAssessmentReason>, CacheError> {
    let result = conn.query_row(
        "SELECT assessment_reason FROM assessments_cache WHERE record_id = ?1 AND assessed_revision = ?2",
        rusqlite::params![record_id, current_revision],
        |row| Ok(CachedAssessmentReason { assessment_reason: row.get(0)? }),
    );
    match result {
        Ok(row) => Ok(Some(row)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Reconstruye el índice FTS desde cero a partir de Records reales — usado
/// tanto en la carga inicial como en la verificación de "cache rebuild
/// desde cero" (Fase E6). Nunca lee `records_fts` como fuente de verdad;
/// siempre re-deriva de `.rationale/records/`.
pub fn rebuild_fts(
    conn: &Connection,
    records: &[crate::storage::Record],
) -> Result<(), CacheError> {
    conn.execute("DELETE FROM records_fts", [])?;
    for record in records {
        conn.execute(
            "INSERT INTO records_fts (record_id, statement, title) VALUES (?1, ?2, ?3)",
            rusqlite::params![record.id, record.statement, record.id],
        )?;
    }
    Ok(())
}

/// Candidatos por búsqueda de texto — orden determinista antes que
/// semántica (Rationale_v0.5.md §19.1): esto es un paso de FTS, no de
/// embeddings, y solo produce candidatos, nunca decide autoridad o bloqueo.
pub fn search_candidates(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<String>, CacheError> {
    let mut stmt = conn.prepare(
        "SELECT record_id FROM records_fts WHERE records_fts MATCH ?1 ORDER BY rank LIMIT ?2",
    )?;
    let rows = stmt.query_map(rusqlite::params![query, limit as i64], |row| row.get(0))?;
    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assessment::{Applicability, AssessmentState, AuthorityStatus, Linkage};
    use crate::revision::Consistency;
    use crate::storage::EpistemicStatus;

    fn temp_cache_dir() -> PathBuf {
        std::env::temp_dir().join(format!("rationale-cache-test-{}", uuid_like()))
    }

    /// PID + nanos + un contador atómico: bajo carga extrema (muchos tests
    /// en paralelo), la resolución real del reloj puede no ser tan fina como
    /// promete `as_nanos()` — dos hilos pueden generar el mismo timestamp y
    /// colisionar en el mismo directorio. El contador lo hace imposible.
    fn uuid_like() -> String {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        format!(
            "{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        )
    }

    fn fixed_assessment(revision: &str) -> Assessment {
        Assessment {
            record_id: "constraint.cache-test".to_string(),
            assessed_revision: Some(revision.to_string()),
            revision_consistency: Consistency::Exact,
            state: AssessmentState {
                epistemic: EpistemicStatus::Stated,
                authority: AuthorityStatus::Approved,
                applicability: Applicability::Active,
                linkage: Linkage::Current,
            },
            assessment_reason: "test reason".to_string(),
            binding_resolution: vec![],
        }
    }

    #[test]
    fn cache_root_is_stable_and_under_home_cache() {
        let root = cache_root(Path::new(".")).unwrap();
        assert!(root
            .to_string_lossy()
            .contains(".cache/rationale/projects/"));
    }

    #[test]
    fn assessment_roundtrip_and_revision_invalidation() {
        let dir = temp_cache_dir();
        let conn = open(&dir).unwrap();

        let assessment_v1 = fixed_assessment("rev-aaa");
        cache_assessment(&conn, &assessment_v1).unwrap();

        // Se lee correctamente para la revisión exacta.
        let found = get_cached_assessment(&conn, "constraint.cache-test", "rev-aaa").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().assessment_reason, "test reason");

        // Para una revisión distinta, es un cache MISS — nunca se sirve
        // el Assessment de una revisión distinta como si fuera actual.
        let stale = get_cached_assessment(&conn, "constraint.cache-test", "rev-bbb").unwrap();
        assert!(stale.is_none());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cache_rebuild_from_scratch_never_loses_canonical_data() {
        // El propio Record real del repo es la fuente de verdad; el cache
        // se destruye y se reconstruye sin perder ninguna decisión, porque
        // nunca fue la única copia.
        let records_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(".rationale/records");
        let records = crate::storage::list_records(&records_dir).unwrap();
        assert!(!records.is_empty(), "debe existir al menos un Record real");

        let dir = temp_cache_dir();
        let conn = open(&dir).unwrap();
        rebuild_fts(&conn, &records).unwrap();

        let candidates = search_candidates(&conn, "provider", 10).unwrap();
        assert!(
            candidates.contains(&"constraint.no-provider-internal-access".to_string()),
            "FTS debe encontrar el Record real por palabra clave de su statement"
        );

        // Destruir el archivo de cache por completo y reconstruir desde
        // .rationale/ — verifica que ninguna decisión se pierde.
        drop(conn);
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(!dir.exists());

        let conn2 = open(&dir).unwrap();
        rebuild_fts(&conn2, &records).unwrap();
        let candidates2 = search_candidates(&conn2, "provider", 10).unwrap();
        assert_eq!(
            candidates, candidates2,
            "la reconstrucción debe ser determinista"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// E6 — lecturas concurrentes: dos conexiones independientes al mismo
    /// archivo `.sqlite3` (el escenario real: dos invocaciones de la CLI o
    /// dos llamadas del servidor MCP en paralelo) deben poder leer el mismo
    /// assessment cacheado sin error ni corrupción. WAL (`PRAGMA
    /// journal_mode = WAL`, ya activado en `open()`) es precisamente lo que
    /// permite lectores concurrentes sin bloquearse entre sí.
    #[test]
    fn concurrent_reads_do_not_corrupt_cache() {
        let dir = temp_cache_dir();
        let writer = open(&dir).unwrap();
        cache_assessment(&writer, &fixed_assessment("rev-concurrent")).unwrap();
        drop(writer);

        let dir_for_threads = std::sync::Arc::new(dir.clone());
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let dir = std::sync::Arc::clone(&dir_for_threads);
                std::thread::spawn(move || {
                    let conn = open(&dir).expect("abrir conexión de lectura concurrente");
                    get_cached_assessment(&conn, "constraint.cache-test", "rev-concurrent")
                        .expect("lectura concurrente no debe fallar")
                })
            })
            .collect();

        for handle in handles {
            let result = handle.join().expect("el hilo lector no debe panicar");
            assert_eq!(
                result.map(|r| r.assessment_reason),
                Some("test reason".to_string()),
                "cada lector concurrente debe ver el mismo dato, sin corrupción"
            );
        }

        std::fs::remove_dir_all(&dir).ok();
    }
}
