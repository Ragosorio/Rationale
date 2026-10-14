//! `rationale doctor` — Fase 1.5: detecta defectos de integridad en el
//! canon que el productor roto (Fase 1.1-1.4 los corrigió en el código,
//! pero no en los datos ya escritos) dejó en `.rationale/`. Read-only por
//! defecto — reparar exige `--repair` y confirmación humana por hallazgo,
//! nunca automática (Proceso §21, mismo principio que `rationale review`).
//!
//! Los cuatro repos que vieron el productor roto (este, Monorepo,
//! BoostAPI, y el proyecto del dogfood real) ya tienen canon escrito con
//! severidades fuera de enum, Records sin bindings, y Subjects colgantes
//! — `doctor` es la herramienta para encontrarlos, no una migración de
//! versión (eso es `.rationale/migrations/`).

use crate::{binding_match, lifecycle, review, storage, subjects};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Finding {
    /// Defecto real: el MCP defaulteaba `severity` a `"normal"`, fuera del
    /// enum del schema — invisible en retrieval sin ningún error.
    InvalidSeverity {
        record_id: String,
        path: PathBuf,
        current: String,
    },
    /// El defecto real que rompió CI (`e2320f0`): `finalize_change`
    /// defaulteaba `kind` a `"constraint"` siempre que el caller no lo
    /// declaraba, así que un Record `id: decision.*` podía escribirse con
    /// `kind: constraint` en silencio. `expected_kind` es lo que
    /// `storage::infer_kind_from_id` deriva del prefijo del propio `id` —
    /// nunca inventado, solo la convención que el mismo `id` ya declara.
    KindMismatch {
        record_id: String,
        path: PathBuf,
        current_kind: String,
        expected_kind: String,
    },
    /// Un Record sin ningún binding no puede gobernar ningún target —
    /// `linkage: unresolved` por diseño (Fase 1.4). Solo se reporta: un
    /// binding inventado sería peor que ninguno.
    RecordWithoutBindings {
        record_id: String,
        path: PathBuf,
    },
    /// Un `path_hint` que ya no existe en el working tree — `linkage:
    /// stale` para ese Record.
    BindingPathMissing {
        record_id: String,
        path: PathBuf,
        binding_id: String,
        path_hint: String,
    },
    /// `record.subject.id` no resuelve contra ningún Subject real en
    /// `.rationale/subjects/` — el defecto original del dogfood, previo a
    /// que `finalize_change` materializara el Subject (Fase 1.3).
    DanglingSubject {
        record_id: String,
        path: PathBuf,
        subject_id: String,
    },
    /// `authority` con un valor que ni `normal` ni `pinned`: se lee como
    /// `normal`, pero quien lo escribió a mano probablemente quiso otra
    /// cosa. Nunca se repara solo — fijar es un acto humano explícito.
    ///
    /// Reemplaza al antiguo "Record sin aprobación": en vNext el canon se
    /// mantiene solo y un Record sin `approvals` es el caso normal.
    InvalidRecordAuthority {
        record_id: String,
        path: PathBuf,
        current: String,
    },
    /// Propuestas de la era de aprobaciones todavía en `.rationale/proposals/`.
    /// vNext no las revisa a mano: `rationale migrate` las pasa por el gate.
    PendingLegacyProposals {
        count: usize,
        path: PathBuf,
    },
    /// `.rationale/bindings/` ya no lo crea `cmd_init` (nada lo lee ni
    /// escribe) — si existe y está vacío, es seguro borrarlo a mano.
    EmptyBindingsDirectory {
        path: PathBuf,
    },
    /// `schema_version` se escribe (`pipeline.rs`, `subjects.rs`) pero
    /// nunca se lee ni se compara — un Record de una versión futura se
    /// parsearía hoy en silencio como si fuera la actual. No hay
    /// migraciones implementadas (`.rationale/migrations/` es una
    /// afordancia vacía), así que esto no se repara: es la puerta visible
    /// que faltaba antes de que exista una migración real.
    UnknownSchemaVersion {
        record_id: String,
        path: PathBuf,
        found_version: String,
    },
    /// `review::approve`/`reject` reclaman una propuesta moviéndola a
    /// `.rationale/proposals/.in-review/{id}.yaml.{pid}-{seq}` antes de
    /// promoverla. Si el proceso muere entre el claim y la promoción, ese
    /// archivo queda ahí para siempre: su extensión ya no es `.yaml` y vive
    /// en una subcarpeta que `list_pending_detailed` no recorre, así que
    /// ninguna vía normal (`rationale review`, `prepare_change`) vuelve a
    /// verlo. Los comentarios que prometían "queda recuperable" no tenían
    /// ningún camino real de recuperación hasta este finding.
    OrphanedClaimedProposal {
        path: PathBuf,
        record_id: Option<String>,
    },
    /// Un archivo de `.rationale/records/` que no se puede leer: no gobierna
    /// nada y nadie lo ve. Antes solo aparecía como `skipped` interno.
    UnreadableRecord {
        path: PathBuf,
        error: String,
    },
    /// Una carpeta (o un archivo) que ya no existe con Records vigentes atados
    /// a ella: un módulo retirado cuyo canon nadie actualizó. Agrupa los
    /// `binding-path-missing` de ese alcance en un solo hallazgo.
    RetiredScope {
        path: String,
        record_ids: Vec<String>,
        /// Fijados por una persona: `rationale retire` no los toca.
        pinned: Vec<String>,
        /// También atados a código vigente: se desatan, no se retiran.
        partially_bound: Vec<String>,
    },
    /// Reemplazado por el `supersedes` de otro Record vigente, pero el propio
    /// Record no lo dice: un binario anterior a 1.2 lo sigue sirviendo como
    /// vigente.
    InconsistentSupersession {
        record_id: String,
        path: PathBuf,
        superseded_by: String,
    },
    SupersessionCycle {
        record_ids: Vec<String>,
    },
    /// Excepción vigente cuya regla ya no gobierna.
    OrphanedException {
        record_id: String,
        path: PathBuf,
        note: String,
    },
    /// Excepción que no dice a qué regla exceptúa (ni `exception_to` ni un id
    /// citado): nadie puede saber cuándo deja de aplicar.
    UnlinkedException {
        record_id: String,
        path: PathBuf,
    },
    /// Binding léxico (`symbol`, sin identidad estructural) a un símbolo que
    /// el archivo ya no define.
    UnconfirmedSymbolMissing {
        record_id: String,
        path: PathBuf,
        symbol: String,
        path_hint: String,
    },
}

/// Gravedad de un hallazgo. `error` rompe la integridad del contexto que se
/// sirve; `warning` lo degrada; `info` es una observación.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FindingSeverity {
    Info,
    Warning,
    Error,
}

impl FindingSeverity {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "info" => Some(Self::Info),
            "warning" => Some(Self::Warning),
            "error" => Some(Self::Error),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

impl Finding {
    pub fn is_repairable(&self) -> bool {
        matches!(
            self,
            Finding::InvalidSeverity { .. }
                | Finding::KindMismatch { .. }
                | Finding::DanglingSubject { .. }
                | Finding::OrphanedClaimedProposal { .. }
                | Finding::InconsistentSupersession { .. }
        )
    }

    pub fn severity(&self) -> FindingSeverity {
        match self {
            Finding::InvalidSeverity { .. }
            | Finding::InvalidRecordAuthority { .. }
            | Finding::BindingPathMissing { .. }
            | Finding::UnreadableRecord { .. }
            | Finding::RetiredScope { .. }
            | Finding::SupersessionCycle { .. }
            | Finding::OrphanedException { .. } => FindingSeverity::Error,
            Finding::KindMismatch { .. }
            | Finding::RecordWithoutBindings { .. }
            | Finding::DanglingSubject { .. }
            | Finding::PendingLegacyProposals { .. }
            | Finding::UnknownSchemaVersion { .. }
            | Finding::OrphanedClaimedProposal { .. }
            | Finding::InconsistentSupersession { .. }
            | Finding::UnconfirmedSymbolMissing { .. } => FindingSeverity::Warning,
            Finding::EmptyBindingsDirectory { .. } | Finding::UnlinkedException { .. } => {
                FindingSeverity::Info
            }
        }
    }

    /// Identidad estable del hallazgo para la línea base de `--check`: el
    /// tipo más lo que lo localiza, nunca el texto (que puede cambiar).
    pub fn fingerprint(&self) -> String {
        let kind = self.kind();
        let key = match self {
            Finding::InvalidSeverity { record_id, .. }
            | Finding::KindMismatch { record_id, .. }
            | Finding::RecordWithoutBindings { record_id, .. }
            | Finding::DanglingSubject { record_id, .. }
            | Finding::InvalidRecordAuthority { record_id, .. }
            | Finding::UnknownSchemaVersion { record_id, .. }
            | Finding::InconsistentSupersession { record_id, .. }
            | Finding::OrphanedException { record_id, .. }
            | Finding::UnlinkedException { record_id, .. } => record_id.clone(),
            Finding::BindingPathMissing {
                record_id,
                path_hint,
                ..
            } => format!("{record_id}:{path_hint}"),
            Finding::UnconfirmedSymbolMissing {
                record_id, symbol, ..
            } => format!("{record_id}:{symbol}"),
            Finding::PendingLegacyProposals { .. } | Finding::EmptyBindingsDirectory { .. } => {
                String::new()
            }
            Finding::OrphanedClaimedProposal { path, .. }
            | Finding::UnreadableRecord { path, .. } => path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            Finding::RetiredScope { path, .. } => path.clone(),
            Finding::SupersessionCycle { record_ids } => record_ids.join(","),
        };
        if key.is_empty() {
            kind
        } else {
            format!("{kind}:{key}")
        }
    }

    pub fn kind(&self) -> String {
        serde_json::to_value(self)
            .ok()
            .and_then(|value| {
                value
                    .get("kind")
                    .and_then(|k| k.as_str())
                    .map(str::to_string)
            })
            .unwrap_or_default()
    }

    /// El comando o la acción que lo resuelve.
    pub fn fix(&self) -> Option<String> {
        match self {
            Finding::InvalidSeverity { .. }
            | Finding::KindMismatch { .. }
            | Finding::DanglingSubject { .. }
            | Finding::OrphanedClaimedProposal { .. }
            | Finding::InconsistentSupersession { .. } => {
                Some("rationale doctor --repair".to_string())
            }
            Finding::RetiredScope { path, .. } => Some(format!(
                "rationale retire {path} --reason \"…\" (plans first; --apply writes)"
            )),
            Finding::PendingLegacyProposals { .. } => Some("rationale migrate".to_string()),
            Finding::InvalidRecordAuthority { record_id, .. } => Some(format!(
                "rationale pin {record_id} (or set authority: normal)"
            )),
            Finding::OrphanedException { record_id, .. } => Some(format!(
                "ask a person: keep it for the new rule (add exception_to), or `rationale \
                 review-record {record_id}` to revoke it"
            )),
            Finding::BindingPathMissing { record_id, .. } => Some(format!(
                "rebind or revoke it: `rationale review-record {record_id}`"
            )),
            _ => None,
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Finding::InvalidSeverity {
                record_id, current, ..
            } => format!(
                "'{record_id}' has severity '{current}', outside the enum ({}); it never decides \
                 visibility, but ordering in retrieval is only real with a valid value",
                storage::Severity::ALL.join(", ")
            ),
            Finding::KindMismatch {
                record_id,
                current_kind,
                expected_kind,
                ..
            } => format!(
                "'{record_id}' has kind '{current_kind}' but its id says '{expected_kind}'; a \
                 Record with the wrong kind can be served as a governing constraint when it is \
                 something else"
            ),
            Finding::RecordWithoutBindings { record_id, .. } => format!(
                "'{record_id}' has no binding, so it can govern no target; it is not repaired \
                 automatically because an invented binding is worse than none"
            ),
            Finding::BindingPathMissing {
                record_id,
                binding_id,
                path_hint,
                ..
            } => format!(
                "'{record_id}': binding '{binding_id}' points at '{path_hint}', which no longer \
                 exists in the working tree"
            ),
            Finding::DanglingSubject {
                record_id,
                subject_id,
                ..
            } => format!(
                "'{record_id}' references Subject '{subject_id}', which does not exist in \
                 .rationale/subjects/; a stub with review.status: unreviewed can be created"
            ),
            Finding::InvalidRecordAuthority {
                record_id, current, ..
            } => format!(
                "'{record_id}' declares authority '{current}', which is neither 'normal' nor \
                 'pinned'; it is read as 'normal'"
            ),
            Finding::PendingLegacyProposals { count, path } => {
                format!("{count} pre-1.0 proposal(s) pending in {}", path.display())
            }
            Finding::EmptyBindingsDirectory { path } => format!(
                "{} exists and is empty; nothing reads or writes it, so it is safe to delete",
                path.display()
            ),
            Finding::UnknownSchemaVersion {
                record_id,
                found_version,
                ..
            } => format!(
                "'{record_id}' declares schema_version '{found_version}', not the version this \
                 binary writes ('{CURRENT_SCHEMA_VERSION}'); review it by hand before trusting \
                 how its fields are read"
            ),
            Finding::OrphanedClaimedProposal { path, record_id } => {
                let label = record_id.as_deref().unwrap_or("unknown");
                format!(
                    "a proposal claimed for review was never promoted or rejected: {} (Record \
                     '{label}'); a process died between the claim and the promotion",
                    path.display()
                )
            }
            Finding::UnreadableRecord { path, error } => {
                format!("{} cannot be read as a Record: {error}", path.display())
            }
            Finding::RetiredScope {
                path,
                record_ids,
                pinned,
                partially_bound,
            } => format!(
                "'{path}' no longer exists, but {} active Record(s) are still bound to it \
                 ({} pinned, {} also bound to current code): they describe retired code as \
                 current",
                record_ids.len(),
                pinned.len(),
                partially_bound.len()
            ),
            Finding::InconsistentSupersession {
                record_id,
                superseded_by,
                ..
            } => format!(
                "'{record_id}' is replaced by '{superseded_by}' (its `supersedes`), but its own \
                 file does not say so; binaries before 1.2 still serve it as current"
            ),
            Finding::SupersessionCycle { record_ids } => format!(
                "these Records supersede each other in a cycle: {}",
                record_ids.join(" → ")
            ),
            Finding::OrphanedException {
                record_id, note, ..
            } => format!("'{record_id}': {note}"),
            Finding::UnlinkedException { record_id, .. } => format!(
                "exception '{record_id}' does not say which rule it excepts (no exception_to, no \
                 cited Record id); nobody can tell when it stops applying"
            ),
            Finding::UnconfirmedSymbolMissing {
                record_id,
                symbol,
                path_hint,
                ..
            } => format!(
                "'{record_id}' is bound to '{symbol}' in '{path_hint}' (lexically, never \
                 confirmed by the index), and the file no longer defines it"
            ),
        }
    }
}

/// La única versión de schema que este binario escribe hoy
/// (`pipeline::finalize`, `subjects::materialize_proposed`). No existe
/// lógica de migración (`.rationale/migrations/` es una afordancia vacía) —
/// esta constante es la puerta de detección, no una promesa de que otras
/// versiones sean incompatibles de verdad.
const CURRENT_SCHEMA_VERSION: &str = "rationale/0.1";

fn schema_version_of(extra: &yaml_serde::Mapping) -> Option<String> {
    extra
        .get(yaml_serde::Value::String("schema_version".to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub findings: Vec<Finding>,
    /// Observaciones sobre el canon que no son defectos (no cuentan para
    /// `--check`): p. ej. que ninguna regla la fijó o la declaró una persona.
    pub observations: Vec<String>,
}

/// Solo lectura — nunca escribe nada. `project_root` es contra qué se
/// verifica la existencia de cada `path_hint` (mismo criterio que
/// `assessment::linkage`, Fase 1.4).
pub fn check(rationale_dir: &Path, project_root: &Path) -> DoctorReport {
    let mut findings = Vec::new();

    let records_dir = rationale_dir.join("records");
    let records_result = storage::list_records_detailed(&records_dir).unwrap_or_else(|_| {
        storage::RecordListResult {
            records: vec![],
            skipped: vec![],
        }
    });

    let subjects_dir = rationale_dir.join("subjects");
    let known_subjects = subjects::list_subjects(&subjects_dir).unwrap_or_default();
    for (path, error) in &records_result.skipped {
        findings.push(Finding::UnreadableRecord {
            path: path.clone(),
            error: error.to_string(),
        });
    }
    let status = lifecycle::StatusIndex::build(&records_result.records);
    // Rutas faltantes de Records vigentes, agrupadas después por alcance
    // retirado. Un Record reemplazado o revocado puede apuntar a código que ya
    // no existe: es historia, no un defecto.
    let mut missing: Vec<(String, String, String, PathBuf)> = Vec::new();

    for record in &records_result.records {
        let path = records_dir.join(format!("{}.yaml", record.id));

        if storage::severity_of(record).is_none() {
            findings.push(Finding::InvalidSeverity {
                record_id: record.id.clone(),
                path: path.clone(),
                current: record.severity.clone(),
            });
        }

        if let Some(expected_kind) = storage::infer_kind_from_id(&record.id) {
            if record.kind != expected_kind {
                findings.push(Finding::KindMismatch {
                    record_id: record.id.clone(),
                    path: path.clone(),
                    current_kind: record.kind.clone(),
                    expected_kind: expected_kind.to_string(),
                });
            }
        }

        if record.binding_declarations.is_empty() {
            findings.push(Finding::RecordWithoutBindings {
                record_id: record.id.clone(),
                path: path.clone(),
            });
        } else {
            for binding in &record.binding_declarations {
                if let Some(hint) = &binding.path_hint {
                    if !status.is_active(record) {
                        continue;
                    }
                    if !project_root.join(hint).exists() {
                        missing.push((
                            record.id.clone(),
                            binding.id.clone(),
                            hint.clone(),
                            path.clone(),
                        ));
                    } else if let Some(symbol) = &binding.symbol {
                        let source =
                            std::fs::read_to_string(project_root.join(hint)).unwrap_or_default();
                        if crate::symbols::find_definition(&source, symbol).is_none() {
                            findings.push(Finding::UnconfirmedSymbolMissing {
                                record_id: record.id.clone(),
                                path: path.clone(),
                                symbol: symbol.clone(),
                                path_hint: hint.clone(),
                            });
                        }
                    }
                }
            }
        }

        if let Some(subject_ref) = &record.subject {
            if subjects::resolve_by_id_or_alias(&known_subjects, &subject_ref.id).is_none() {
                findings.push(Finding::DanglingSubject {
                    record_id: record.id.clone(),
                    path: path.clone(),
                    subject_id: subject_ref.id.clone(),
                });
            }
        }

        if let Some(authority) = &record.authority {
            if !matches!(authority.as_str(), "normal" | "pinned") {
                findings.push(Finding::InvalidRecordAuthority {
                    record_id: record.id.clone(),
                    path: path.clone(),
                    current: authority.clone(),
                });
            }
        }

        if let Some(found_version) = schema_version_of(&record.extra) {
            if found_version != CURRENT_SCHEMA_VERSION {
                findings.push(Finding::UnknownSchemaVersion {
                    record_id: record.id.clone(),
                    path,
                    found_version,
                });
            }
        }
    }

    findings.extend(retired_scopes(
        project_root,
        &records_result.records,
        &status,
        missing,
    ));
    findings.extend(lifecycle_findings(
        &records_dir,
        &records_result.records,
        &status,
    ));

    let bindings_dir = rationale_dir.join("bindings");
    if bindings_dir.is_dir() {
        let is_empty = std::fs::read_dir(&bindings_dir)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false);
        if is_empty {
            findings.push(Finding::EmptyBindingsDirectory { path: bindings_dir });
        }
    }

    let proposals_dir = rationale_dir.join("proposals");
    let pending_count = std::fs::read_dir(&proposals_dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("yaml"))
                .count()
        })
        .unwrap_or(0);
    if pending_count > 0 {
        findings.push(Finding::PendingLegacyProposals {
            count: pending_count,
            path: proposals_dir,
        });
    }

    let in_review_dir = rationale_dir.join("proposals").join(".in-review");
    if let Ok(entries) = std::fs::read_dir(&in_review_dir) {
        for entry in entries.flatten() {
            let claimed_path = entry.path();
            if !claimed_path.is_file() {
                continue;
            }
            let record_id = storage::read_record(&claimed_path)
                .ok()
                .map(|r| r.id.clone());
            findings.push(Finding::OrphanedClaimedProposal {
                path: claimed_path,
                record_id,
            });
        }
    }

    let observations = human_authority_observation(&records_result.records, &status)
        .into_iter()
        .collect();
    DoctorReport {
        findings,
        observations,
    }
}

/// El ancestro más alto que tampoco existe: `src/comunicaciones/inbox.ts`
/// borrado junto con su carpeta da `src/comunicaciones`.
fn retired_root(project_root: &Path, missing: &str) -> String {
    let mut root = missing.trim_end_matches('/').to_string();
    while let Some((parent, _)) = root.rsplit_once('/') {
        if parent.is_empty() || project_root.join(parent).exists() {
            break;
        }
        root = parent.to_string();
    }
    root
}

/// Agrupa las rutas faltantes por alcance retirado. Una carpeta retirada, o
/// un archivo con varios Records, es un solo hallazgo con su lista; un
/// archivo suelto con un solo Record sigue siendo `binding-path-missing`.
fn retired_scopes(
    project_root: &Path,
    records: &[storage::Record],
    status: &lifecycle::StatusIndex,
    missing: Vec<(String, String, String, PathBuf)>,
) -> Vec<Finding> {
    let mut groups: BTreeMap<String, Vec<(String, String, String, PathBuf)>> = BTreeMap::new();
    for entry in missing {
        groups
            .entry(retired_root(project_root, &entry.2))
            .or_default()
            .push(entry);
    }
    let by_id: HashMap<&str, &storage::Record> =
        records.iter().map(|r| (r.id.as_str(), r)).collect();
    let mut findings = Vec::new();
    for (root, entries) in groups {
        let mut record_ids: Vec<String> = entries.iter().map(|e| e.0.clone()).collect();
        record_ids.sort();
        record_ids.dedup();
        // Un solo Record conserva su hallazgo puntual, más preciso.
        if record_ids.len() < 2 {
            for (record_id, binding_id, path_hint, path) in entries {
                findings.push(Finding::BindingPathMissing {
                    record_id,
                    path,
                    binding_id,
                    path_hint,
                });
            }
            continue;
        }
        let mut pinned = Vec::new();
        let mut partially_bound = Vec::new();
        for id in &record_ids {
            let Some(record) = by_id.get(id.as_str()) else {
                continue;
            };
            if !status.is_active(record) {
                continue;
            }
            if storage::record_authority(record) == storage::RecordAuthority::Pinned {
                pinned.push(id.clone());
            }
            let outside = record.binding_declarations.iter().any(|b| {
                b.path_hint.as_deref().is_some_and(|hint| {
                    let hint = hint.trim_end_matches('/');
                    hint != root && !binding_match::is_under(hint, &root)
                })
            });
            if outside {
                partially_bound.push(id.clone());
            }
        }
        findings.push(Finding::RetiredScope {
            path: root,
            record_ids,
            pinned,
            partially_bound,
        });
    }
    findings
}

/// Supersesiones de un solo lado, ciclos y excepciones sin regla vigente.
fn lifecycle_findings(
    records_dir: &Path,
    records: &[storage::Record],
    status: &lifecycle::StatusIndex,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let path_of = |id: &str| records_dir.join(format!("{id}.yaml"));
    for record in records {
        if status.status(record) == lifecycle::RecordStatus::Superseded
            && storage::superseded_by(record).is_none()
            && storage::lifecycle_status(record) != Some("superseded")
        {
            if let Some(by) = status.replaced_by(&record.id) {
                findings.push(Finding::InconsistentSupersession {
                    record_id: record.id.clone(),
                    path: path_of(&record.id),
                    superseded_by: by.to_string(),
                });
            }
        }
        if let Some(note) = lifecycle::orphaned_exception(record, status) {
            findings.push(Finding::OrphanedException {
                record_id: record.id.clone(),
                path: path_of(&record.id),
                note,
            });
        } else if record.kind == "exception"
            && status.is_active(record)
            && lifecycle::exception_targets(record).0.is_empty()
        {
            findings.push(Finding::UnlinkedException {
                record_id: record.id.clone(),
                path: path_of(&record.id),
            });
        }
    }
    // Ciclos en `supersedes`: a → b → … → a.
    let edges: HashMap<&str, Vec<&str>> = records
        .iter()
        .map(|r| {
            (
                r.id.as_str(),
                r.supersedes.iter().map(String::as_str).collect(),
            )
        })
        .collect();
    let mut reported: HashSet<Vec<String>> = HashSet::new();
    for start in edges.keys() {
        let mut path = vec![*start];
        let mut current = *start;
        let mut seen = HashSet::new();
        while let Some(next) = edges.get(current).and_then(|targets| targets.first()) {
            if *next == *start {
                let mut cycle: Vec<String> = path.iter().map(|s| s.to_string()).collect();
                cycle.sort();
                if reported.insert(cycle.clone()) {
                    findings.push(Finding::SupersessionCycle { record_ids: cycle });
                }
                break;
            }
            if !seen.insert(*next) {
                break;
            }
            path.push(next);
            current = next;
        }
    }
    findings
}

/// Ninguna regla vigente la fijó ni la declaró una persona: toda la memoria
/// es lo que afirmaron agentes. No es un defecto, pero conviene saberlo.
fn human_authority_observation(
    records: &[storage::Record],
    status: &lifecycle::StatusIndex,
) -> Option<String> {
    let active: Vec<&storage::Record> = records.iter().filter(|r| status.is_active(r)).collect();
    let human = active.iter().any(|r| {
        storage::record_authority(r) == storage::RecordAuthority::Pinned
            || storage::provenance_kind(r) == storage::ProvenanceKind::HumanStated
    });
    (!active.is_empty() && !human).then(|| {
        format!(
            "{} active Record(s), none pinned or stated by a person: every rule is what an \
             agent asserted. People state the rules they own with `rationale rule add` or fix \
             one with `rationale pin <id>`.",
            active.len()
        )
    })
}

// ---------------------------------------------------------------------------
// Línea base para `--check`
// ---------------------------------------------------------------------------

const BASELINE_SCHEMA: &str = "rationale/doctor-baseline/1";
pub const BASELINE_FILE: &str = "doctor-baseline.json";

/// Deuda conocida y aceptada: los hallazgos que existían cuando una persona
/// fijó la línea base. `--check` falla solo por lo nuevo; lo conocido se
/// sigue reportando, nunca se oculta.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Baseline {
    pub schema_version: String,
    pub created_at: String,
    pub findings: Vec<BaselineEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineEntry {
    pub fingerprint: String,
    pub severity: FindingSeverity,
    pub summary: String,
}

pub fn load_baseline(rationale_dir: &Path) -> Result<Option<Baseline>, String> {
    let path = rationale_dir.join(BASELINE_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|e| format!("{} is not a valid baseline: {e}", path.display()))
}

pub fn write_baseline(rationale_dir: &Path, report: &DoctorReport) -> Result<PathBuf, String> {
    let mut entries: Vec<BaselineEntry> = report
        .findings
        .iter()
        .map(|finding| BaselineEntry {
            fingerprint: finding.fingerprint(),
            severity: finding.severity(),
            summary: crate::activity::bounded(&finding.describe(), 160),
        })
        .collect();
    entries.sort_by(|a, b| a.fingerprint.cmp(&b.fingerprint));
    entries.dedup_by(|a, b| a.fingerprint == b.fingerprint);
    let baseline = Baseline {
        schema_version: BASELINE_SCHEMA.to_string(),
        created_at: crate::evaluation::now_iso8601(),
        findings: entries,
    };
    let path = rationale_dir.join(BASELINE_FILE);
    let mut bytes = serde_json::to_vec_pretty(&baseline).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    storage::atomic_write_bytes(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Cómo queda cada hallazgo frente a la línea base.
#[derive(Debug, Serialize)]
pub struct CheckedFinding<'a> {
    #[serde(flatten)]
    pub finding: &'a Finding,
    pub severity: FindingSeverity,
    pub fingerprint: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
    /// `new` (no estaba en la línea base) o `known`.
    pub baseline: &'static str,
}

#[derive(Debug, Serialize)]
pub struct CheckSummary {
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
    /// Hallazgos que no estaban en la línea base, con gravedad ≥ el umbral.
    pub new_failing: usize,
    pub known: usize,
    /// En la línea base y ya resueltos: la línea base puede actualizarse.
    pub fixed: Vec<String>,
    pub baseline_present: bool,
}

pub fn classify<'a>(
    report: &'a DoctorReport,
    baseline: Option<&Baseline>,
    fail_on: FindingSeverity,
) -> (Vec<CheckedFinding<'a>>, CheckSummary) {
    let known: HashSet<&str> = baseline
        .map(|b| b.findings.iter().map(|e| e.fingerprint.as_str()).collect())
        .unwrap_or_default();
    let current: HashSet<String> = report.findings.iter().map(Finding::fingerprint).collect();
    let checked: Vec<CheckedFinding> = report
        .findings
        .iter()
        .map(|finding| {
            let fingerprint = finding.fingerprint();
            CheckedFinding {
                severity: finding.severity(),
                baseline: if known.contains(fingerprint.as_str()) {
                    "known"
                } else {
                    "new"
                },
                fingerprint,
                message: finding.describe(),
                fix: finding.fix(),
                finding,
            }
        })
        .collect();
    let count = |severity| checked.iter().filter(|c| c.severity == severity).count();
    let summary = CheckSummary {
        errors: count(FindingSeverity::Error),
        warnings: count(FindingSeverity::Warning),
        info: count(FindingSeverity::Info),
        new_failing: checked
            .iter()
            .filter(|c| c.baseline == "new" && c.severity >= fail_on)
            .count(),
        known: checked.iter().filter(|c| c.baseline == "known").count(),
        fixed: baseline
            .map(|b| {
                b.findings
                    .iter()
                    .filter(|e| !current.contains(&e.fingerprint))
                    .map(|e| e.fingerprint.clone())
                    .collect()
            })
            .unwrap_or_default(),
        baseline_present: baseline.is_some(),
    };
    (checked, summary)
}

/// Repara UN hallazgo — siempre con la palabra de confirmación ya validada
/// por el caller (misma disciplina que `rationale review`: nunca se
/// autoaprueba, nunca se repara en lote sin que un humano vea cada uno).
/// Las mutaciones de Records pasan por `review::mutate_record` — nunca un
/// side-channel que esquive el audit trail de lifecycle (`Record.extra.
/// lifecycle.events`).
pub fn repair(
    rationale_dir: &Path,
    finding: &Finding,
    reviewer_actor: &str,
) -> Result<PathBuf, String> {
    match finding {
        Finding::InvalidSeverity {
            record_id, current, ..
        } => Err(format!(
            "InvalidSeverity para '{record_id}' (actual: '{current}') no se repara sin que el \
             humano elija el nuevo valor — usar repair_severity()"
        )),
        Finding::KindMismatch {
            record_id,
            current_kind,
            ..
        } => Err(format!(
            "KindMismatch para '{record_id}' (actual: '{current_kind}') no se repara sin \
             confirmación humana — usar repair_kind()"
        )),
        Finding::DanglingSubject {
            record_id,
            subject_id,
            ..
        } => {
            let subjects_dir = rationale_dir.join("subjects");
            let records_dir = rationale_dir.join("records");
            let record_path = records_dir.join(format!("{record_id}.yaml"));
            let record = storage::read_record(&record_path).map_err(|e| e.to_string())?;
            let title = record
                .subject
                .as_ref()
                .and_then(|s| {
                    let title_key = yaml_serde::Value::String("title".to_string());
                    s.extra.get(&title_key).and_then(|v| v.as_str())
                })
                .unwrap_or(subject_id)
                .to_string();
            match subjects::materialize_proposed(
                &subjects_dir,
                subject_id,
                &title,
                "",
                "project",
                &[],
                reviewer_actor,
                &crate::evaluation::now_iso8601(),
            )
            .map_err(|e| e.to_string())?
            {
                subjects::MaterializeOutcome::Created(path) => Ok(path),
                subjects::MaterializeOutcome::AlreadyExists(path) => Ok(path),
            }
        }
        Finding::OrphanedClaimedProposal { path, record_id } => {
            let record_id = record_id.as_ref().ok_or_else(|| {
                format!(
                    "{} no se pudo leer como Record — no hay id seguro para devolverlo a \
                     proposals/; revisar a mano",
                    path.display()
                )
            })?;
            storage::validate_safe_id(record_id).map_err(|e| e.to_string())?;
            let proposals_dir = rationale_dir.join("proposals");
            let restored = proposals_dir.join(format!("{record_id}.yaml"));
            if restored.exists() {
                return Err(format!(
                    "ya existe {} — una propuesta más nueva con el mismo id llegó después del \
                     crash; resolver el conflicto a mano antes de devolver la reclamada",
                    restored.display()
                ));
            }
            std::fs::rename(path, &restored).map_err(|e| e.to_string())?;
            Ok(restored)
        }
        Finding::InconsistentSupersession {
            record_id,
            superseded_by,
            path,
        } => {
            crate::canon::mark_superseded_in(
                &rationale_dir.join("records"),
                record_id,
                superseded_by,
                &format!("user:{reviewer_actor}"),
                "both sides of the supersession recorded by rationale doctor --repair",
            )?;
            Ok(path.clone())
        }
        _ => Err("this finding is not repaired automatically; see its fix".into()),
    }
}

/// Repara `InvalidSeverity` con el valor que el humano eligió — separado
/// de `repair()` porque exige un input adicional (`new_severity`) que solo
/// tiene sentido pedido interactivamente, nunca inferido.
pub fn repair_severity(
    rationale_dir: &Path,
    record_id: &str,
    new_severity: &str,
    reason: &str,
    reviewer_actor: &str,
    reviewer_role: storage::AuthorityRole,
    reviewer_declared: bool,
) -> Result<PathBuf, String> {
    review::mutate_record(
        rationale_dir,
        record_id,
        review::RecordMutation::CorrectSeverity {
            new_severity: new_severity.to_string(),
            reason: reason.to_string(),
        },
        reviewer_actor,
        reviewer_role,
        reviewer_declared,
    )
    .map_err(|e| e.to_string())
}

/// Repara `KindMismatch` con el `kind` que el humano confirmó — separado de
/// `repair()` por la misma razón que `repair_severity`: aunque
/// `storage::infer_kind_from_id` ya sugiere el valor correcto, escribirlo
/// exige confirmación humana explícita por Record, nunca una corrección
/// silenciosa en lote.
pub fn repair_kind(
    rationale_dir: &Path,
    record_id: &str,
    new_kind: &str,
    reason: &str,
    reviewer_actor: &str,
    reviewer_role: storage::AuthorityRole,
    reviewer_declared: bool,
) -> Result<PathBuf, String> {
    review::mutate_record(
        rationale_dir,
        record_id,
        review::RecordMutation::CorrectKind {
            new_kind: new_kind.to_string(),
            reason: reason.to_string(),
        },
        reviewer_actor,
        reviewer_role,
        reviewer_declared,
    )
    .map_err(|e| e.to_string())
}

/// Repara `RecordWithoutBindings` con la ruta (y símbolo opcional) que el
/// HUMANO aporta — nunca `repair()` genérico, por la misma razón que
/// `repair_severity`: exige un input que solo tiene sentido pedido
/// interactivamente. `RecordWithoutBindings::is_repairable() == false` a
/// propósito: sigue sin haber reparación automática, solo un camino para
/// que un humano cierre el hueco que dejó el productor roto sin que
/// Rationale invente el dato. Rechaza una ruta que no exista en
/// `project_root` — un binding humano sigue debiendo señalar algo real.
#[allow(clippy::too_many_arguments)]
pub fn repair_binding(
    rationale_dir: &Path,
    project_root: &Path,
    record_id: &str,
    path_hint: &str,
    symbol: Option<&str>,
    reason: &str,
    reviewer_actor: &str,
    reviewer_role: storage::AuthorityRole,
    reviewer_declared: bool,
) -> Result<PathBuf, String> {
    if !project_root.join(path_hint).is_file() {
        return Err(format!(
            "'{path_hint}' no existe en {} — un binding humano debe señalar un archivo real, \
             no inventado",
            project_root.display()
        ));
    }
    review::mutate_record(
        rationale_dir,
        record_id,
        review::RecordMutation::AddHumanConfirmedBinding {
            path_hint: path_hint.to_string(),
            symbol: symbol.map(|s| s.to_string()),
            reason: reason.to_string(),
        },
        reviewer_actor,
        reviewer_role,
        reviewer_declared,
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_dir(label: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "rationale-doctor-test-{label}-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_record_yaml(records_dir: &Path, id: &str, content: &str) {
        std::fs::create_dir_all(records_dir).unwrap();
        std::fs::write(records_dir.join(format!("{id}.yaml")), content).unwrap();
    }

    #[test]
    fn detects_invalid_severity() {
        let project = unique_dir("severity");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "constraint.legacy",
            "id: constraint.legacy\nkind: constraint\nseverity: normal\nstatement: \"x\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\n",
        );

        let report = check(&rationale_dir, &project);
        assert!(report.findings.iter().any(
            |f| matches!(f, Finding::InvalidSeverity { record_id, current, .. } if record_id == "constraint.legacy" && current == "normal")
        ));

        std::fs::remove_dir_all(&project).ok();
    }

    /// El defecto real que rompió CI: un Record `id: decision.*` escrito con
    /// `kind: constraint` (porque `finalize_change` defaulteaba siempre a
    /// `"constraint"` sin `kind` explícito) debe ser detectable por `doctor`.
    #[test]
    fn detects_kind_mismatch_with_id_prefix() {
        let project = unique_dir("kind-mismatch");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "decision.mislabeled",
            "id: decision.mislabeled\nkind: constraint\nseverity: high\nstatement: \"x\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\n",
        );

        let report = check(&rationale_dir, &project);
        assert!(report.findings.iter().any(|f| matches!(
            f,
            Finding::KindMismatch { record_id, current_kind, expected_kind, .. }
                if record_id == "decision.mislabeled" && current_kind == "constraint" && expected_kind == "decision"
        )));

        std::fs::remove_dir_all(&project).ok();
    }

    /// Un `kind` que ya coincide con el prefijo de su id nunca debe
    /// reportarse — el hallazgo es exclusivamente sobre la discrepancia.
    #[test]
    fn kind_matching_id_prefix_is_not_flagged() {
        let project = unique_dir("kind-match");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "decision.correctly-labeled",
            "id: decision.correctly-labeled\nkind: decision\nseverity: high\nstatement: \"x\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\n",
        );

        let report = check(&rationale_dir, &project);
        assert!(!report
            .findings
            .iter()
            .any(|f| matches!(f, Finding::KindMismatch { .. })));

        std::fs::remove_dir_all(&project).ok();
    }

    /// `schema_version` se escribe pero nunca se leía ni comparaba —
    /// `doctor` es la puerta de detección visible hasta que exista una
    /// migración real.
    #[test]
    fn detects_unknown_schema_version() {
        let project = unique_dir("schema-version");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "constraint.future-schema",
            "id: constraint.future-schema\nkind: constraint\nseverity: high\nstatement: \"x\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\nschema_version: rationale/0.2\n",
        );

        let report = check(&rationale_dir, &project);
        assert!(report.findings.iter().any(|f| matches!(
            f,
            Finding::UnknownSchemaVersion { record_id, found_version, .. }
                if record_id == "constraint.future-schema" && found_version == "rationale/0.2"
        )));

        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn current_schema_version_is_not_flagged() {
        let project = unique_dir("schema-version-current");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "constraint.current-schema",
            "id: constraint.current-schema\nkind: constraint\nseverity: high\nstatement: \"x\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\nschema_version: rationale/0.1\n",
        );

        let report = check(&rationale_dir, &project);
        assert!(!report
            .findings
            .iter()
            .any(|f| matches!(f, Finding::UnknownSchemaVersion { .. })));

        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn detects_record_without_bindings() {
        let project = unique_dir("no-bindings");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "constraint.no-bindings",
            "id: constraint.no-bindings\nkind: constraint\nseverity: high\nstatement: \"x\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\n",
        );

        let report = check(&rationale_dir, &project);
        assert!(report
            .findings
            .iter()
            .any(|f| matches!(f, Finding::RecordWithoutBindings { record_id, .. } if record_id == "constraint.no-bindings")));

        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn detects_missing_binding_path() {
        let project = unique_dir("missing-path");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "constraint.missing-path",
            "id: constraint.missing-path\nkind: constraint\nseverity: high\nstatement: \"x\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\nbinding_declarations:\n  - id: binding.a\n    type: file\n    path_hint: does/not/exist.rs\n",
        );

        let report = check(&rationale_dir, &project);
        assert!(report.findings.iter().any(|f| matches!(
            f,
            Finding::BindingPathMissing { record_id, path_hint, .. }
                if record_id == "constraint.missing-path" && path_hint == "does/not/exist.rs"
        )));

        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn detects_dangling_subject() {
        let project = unique_dir("dangling-subject");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "constraint.dangling",
            "id: constraint.dangling\nkind: constraint\nseverity: high\nstatement: \"x\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\nbinding_declarations:\n  - id: binding.a\n    type: file\n    path_hint: README.md\nsubject:\n  id: some.missing-subject\n",
        );
        std::fs::write(project.join("README.md"), "x\n").unwrap();

        let report = check(&rationale_dir, &project);
        assert!(report.findings.iter().any(|f| matches!(
            f,
            Finding::DanglingSubject { record_id, subject_id, .. }
                if record_id == "constraint.dangling" && subject_id == "some.missing-subject"
        )));

        std::fs::remove_dir_all(&project).ok();
    }

    /// vNext: un Record sin aprobaciones es el caso normal del canon
    /// autónomo, nunca un hallazgo.
    #[test]
    fn record_without_approval_is_not_a_finding() {
        let project = unique_dir("no-approval");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "constraint.no-approval",
            "id: constraint.no-approval\nkind: constraint\nseverity: high\nstatement: \"x\"\napprovals: []\nbinding_declarations:\n  - id: binding.a\n    type: file\n    path_hint: README.md\n",
        );
        std::fs::write(project.join("README.md"), "x\n").unwrap();

        let report = check(&rationale_dir, &project);
        assert!(report.findings.is_empty(), "{:?}", report.findings);

        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn detects_invalid_authority_and_pending_legacy_proposals() {
        let project = unique_dir("vnext-findings");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "constraint.odd-authority",
            "id: constraint.odd-authority\nkind: constraint\nseverity: high\nstatement: \"x\"\nauthority: approved\nbinding_declarations:\n  - id: binding.a\n    type: file\n    path_hint: README.md\n",
        );
        std::fs::write(project.join("README.md"), "x\n").unwrap();
        write_record_yaml(
            &rationale_dir.join("proposals"),
            "constraint.legacy",
            "id: constraint.legacy\nkind: constraint\nseverity: high\nstatement: \"x\"\nstatus: pending\n",
        );

        let report = check(&rationale_dir, &project);
        assert!(report.findings.iter().any(|f| matches!(
            f,
            Finding::InvalidRecordAuthority { current, .. } if current == "approved"
        )));
        assert!(report
            .findings
            .iter()
            .any(|f| matches!(f, Finding::PendingLegacyProposals { count: 1, .. })));

        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn detects_empty_bindings_directory() {
        let project = unique_dir("empty-bindings-dir");
        let rationale_dir = project.join(".rationale");
        std::fs::create_dir_all(rationale_dir.join("bindings")).unwrap();

        let report = check(&rationale_dir, &project);
        assert!(report
            .findings
            .iter()
            .any(|f| matches!(f, Finding::EmptyBindingsDirectory { .. })));

        std::fs::remove_dir_all(&project).ok();
    }

    fn bound(id: &str, kind: &str, paths: &[&str], extra: &str) -> String {
        let bindings: String = paths
            .iter()
            .enumerate()
            .map(|(i, p)| format!("  - id: b.{i}\n    type: file\n    path_hint: {p}\n"))
            .collect();
        format!(
            "id: {id}\nkind: {kind}\nseverity: high\nstatement: \"s {id}\"\nrationale: \"r\"\nbinding_declarations:\n{bindings}{extra}"
        )
    }

    #[test]
    fn a_retired_folder_is_one_finding_and_inactive_records_are_history() {
        let project = unique_dir("retired");
        let rationale_dir = project.join(".rationale");
        let records = rationale_dir.join("records");
        std::fs::create_dir_all(project.join("src/notifications")).unwrap();
        std::fs::write(project.join("src/notifications/email.ts"), "x\n").unwrap();
        for (id, paths, extra) in [
            ("decision.a", vec!["src/comms/inbox.ts"], ""),
            (
                "decision.b",
                vec!["src/comms/outbox.ts", "src/notifications/email.ts"],
                "",
            ),
            (
                "constraint.c",
                vec!["src/comms/templates.ts"],
                "authority: pinned\n",
            ),
            (
                "decision.old",
                vec!["src/comms/inbox.ts"],
                "lifecycle:\n  status: revoked\n",
            ),
        ] {
            let kind = id.split('.').next().unwrap();
            write_record_yaml(&records, id, &bound(id, kind, &paths, extra));
        }
        let report = check(&rationale_dir, &project);
        let retired: Vec<_> = report
            .findings
            .iter()
            .filter_map(|f| match f {
                Finding::RetiredScope {
                    path,
                    record_ids,
                    pinned,
                    partially_bound,
                } => Some((path, record_ids, pinned, partially_bound)),
                _ => None,
            })
            .collect();
        assert_eq!(retired.len(), 1, "{:?}", report.findings);
        let (path, ids, pinned, partial) = retired[0];
        assert_eq!(path, "src/comms");
        assert_eq!(ids, &["constraint.c", "decision.a", "decision.b"]);
        assert_eq!(pinned, &["constraint.c"]);
        assert_eq!(partial, &["decision.b"]);
        assert!(!report
            .findings
            .iter()
            .any(|f| matches!(f, Finding::BindingPathMissing { .. })));
        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn one_sided_supersession_is_found_and_repaired_on_the_old_record() {
        let project = unique_dir("one-sided");
        let rationale_dir = project.join(".rationale");
        let records = rationale_dir.join("records");
        std::fs::write(project.join("README.md"), "x\n").unwrap();
        write_record_yaml(
            &records,
            "decision.old",
            &bound("decision.old", "decision", &["README.md"], ""),
        );
        write_record_yaml(
            &records,
            "decision.new",
            &bound(
                "decision.new",
                "decision",
                &["README.md"],
                "supersedes: [decision.old]\n",
            ),
        );
        let report = check(&rationale_dir, &project);
        let finding = report
            .findings
            .iter()
            .find(|f| matches!(f, Finding::InconsistentSupersession { .. }))
            .expect("one-sided supersession");
        assert!(finding.is_repairable());
        repair(&rationale_dir, finding, "owner").unwrap();
        let old = storage::read_record(&records.join("decision.old.yaml")).unwrap();
        assert_eq!(storage::superseded_by(&old), Some("decision.new"));
        assert_eq!(storage::lifecycle_status(&old), Some("superseded"));
        assert!(!check(&rationale_dir, &project)
            .findings
            .iter()
            .any(|f| matches!(f, Finding::InconsistentSupersession { .. })));
        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn exceptions_without_a_current_rule_are_errors_and_unlinked_ones_info() {
        let project = unique_dir("exceptions");
        let rationale_dir = project.join(".rationale");
        let records = rationale_dir.join("records");
        std::fs::write(project.join("README.md"), "x\n").unwrap();
        write_record_yaml(
            &records,
            "constraint.rule",
            &bound(
                "constraint.rule",
                "constraint",
                &["README.md"],
                "lifecycle:\n  status: revoked\n",
            ),
        );
        write_record_yaml(
            &records,
            "exception.orphan",
            &bound(
                "exception.orphan",
                "exception",
                &["README.md"],
                "exception_to: [constraint.rule]\n",
            ),
        );
        write_record_yaml(
            &records,
            "exception.unlinked",
            &bound("exception.unlinked", "exception", &["README.md"], ""),
        );
        let report = check(&rationale_dir, &project);
        let orphan = report
            .findings
            .iter()
            .find(|f| matches!(f, Finding::OrphanedException { .. }))
            .unwrap();
        assert_eq!(orphan.severity(), FindingSeverity::Error);
        let unlinked = report
            .findings
            .iter()
            .find(|f| matches!(f, Finding::UnlinkedException { .. }))
            .unwrap();
        assert_eq!(unlinked.severity(), FindingSeverity::Info);
        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn the_baseline_fails_only_on_new_findings_and_never_hides_known_ones() {
        let project = unique_dir("baseline");
        let rationale_dir = project.join(".rationale");
        let records = rationale_dir.join("records");
        write_record_yaml(
            &records,
            "constraint.legacy",
            &bound("constraint.legacy", "constraint", &["gone.rs"], ""),
        );
        let report = check(&rationale_dir, &project);
        let (_, before) = classify(&report, None, FindingSeverity::Error);
        assert_eq!(
            before.new_failing, 1,
            "without a baseline everything is new"
        );

        write_baseline(&rationale_dir, &report).unwrap();
        let baseline = load_baseline(&rationale_dir).unwrap().unwrap();
        let (checked, known) = classify(&report, Some(&baseline), FindingSeverity::Error);
        assert_eq!(known.new_failing, 0);
        assert_eq!(checked[0].baseline, "known", "still reported");

        write_record_yaml(
            &records,
            "constraint.fresh",
            &bound("constraint.fresh", "constraint", &["also-gone.rs"], ""),
        );
        let report = check(&rationale_dir, &project);
        let (_, after) = classify(&report, Some(&baseline), FindingSeverity::Error);
        assert_eq!(after.new_failing, 1, "a new regression fails");
        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn clean_project_has_no_findings() {
        let project = unique_dir("clean");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "constraint.clean",
            "id: constraint.clean\nkind: constraint\nseverity: high\nstatement: \"x\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\nbinding_declarations:\n  - id: binding.a\n    type: file\n    path_hint: README.md\n",
        );
        std::fs::write(project.join("README.md"), "x\n").unwrap();

        let report = check(&rationale_dir, &project);
        assert!(
            report.findings.is_empty(),
            "no debía haber hallazgos: {:?}",
            report.findings
        );

        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn repair_severity_writes_through_review_mutate_record_with_lifecycle_event() {
        let project = unique_dir("repair-severity");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "constraint.legacy",
            "id: constraint.legacy\nkind: constraint\nseverity: normal\nstatement: \"x\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\n",
        );

        let path = repair_severity(
            &rationale_dir,
            "constraint.legacy",
            "high",
            "corregido por rationale doctor",
            "user:doctor-test",
            storage::AuthorityRole::Contributor,
            true,
        )
        .unwrap();

        let record = storage::read_record(&path).unwrap();
        assert_eq!(record.severity, "high");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("severity-corrected"));

        std::fs::remove_dir_all(&project).ok();
    }

    /// Fase C del plan beta: `RecordWithoutBindings` no es reparable
    /// automáticamente (`is_repairable() == false`), pero `repair_binding`
    /// da el único camino de migración para el canon legado — con la ruta
    /// que aporta el humano, nunca inventada.
    #[test]
    fn repair_binding_adds_human_confirmed_binding_and_clears_the_finding() {
        let project = unique_dir("repair-binding");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "constraint.no-bindings",
            "id: constraint.no-bindings\nkind: constraint\nseverity: high\nstatement: \"x\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\n",
        );
        std::fs::create_dir_all(project.join("app")).unwrap();
        std::fs::write(
            project.join("app/upload.ts"),
            "export function submit() {}\n",
        )
        .unwrap();

        let report_before = check(&rationale_dir, &project);
        assert!(report_before
            .findings
            .iter()
            .any(|f| matches!(f, Finding::RecordWithoutBindings { .. })));

        let path = repair_binding(
            &rationale_dir,
            &project,
            "constraint.no-bindings",
            "app/upload.ts",
            Some("submit"),
            "confirmado a mano tras revisar el codigo real",
            "user:doctor-test",
            storage::AuthorityRole::Contributor,
            true,
        )
        .unwrap();
        assert!(path.exists());

        let report_after = check(&rationale_dir, &project);
        assert!(!report_after
            .findings
            .iter()
            .any(|f| matches!(f, Finding::RecordWithoutBindings { .. })));

        std::fs::remove_dir_all(&project).ok();
    }

    /// Un binding humano sigue debiendo señalar algo real — nunca debe
    /// poder "confirmarse" una ruta que no existe.
    #[test]
    fn repair_binding_rejects_a_path_that_does_not_exist() {
        let project = unique_dir("repair-binding-missing-path");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "constraint.no-bindings",
            "id: constraint.no-bindings\nkind: constraint\nseverity: high\nstatement: \"x\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\n",
        );

        let result = repair_binding(
            &rationale_dir,
            &project,
            "constraint.no-bindings",
            "does/not/exist.ts",
            None,
            "intento invalido",
            "user:doctor-test",
            storage::AuthorityRole::Contributor,
            true,
        );
        assert!(result.is_err());

        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn repair_dangling_subject_materializes_stub() {
        let project = unique_dir("repair-subject");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "constraint.dangling",
            "id: constraint.dangling\nkind: constraint\nseverity: high\nstatement: \"x\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\nbinding_declarations:\n  - id: binding.a\n    type: file\n    path_hint: README.md\nsubject:\n  id: some.missing-subject\n  title: Missing subject\n",
        );
        std::fs::write(project.join("README.md"), "x\n").unwrap();

        let report = check(&rationale_dir, &project);
        let finding = report
            .findings
            .iter()
            .find(|f| matches!(f, Finding::DanglingSubject { .. }))
            .unwrap();

        let path = repair(&rationale_dir, finding, "user:doctor-test").unwrap();
        assert!(path.exists());

        let report_after = check(&rationale_dir, &project);
        assert!(!report_after
            .findings
            .iter()
            .any(|f| matches!(f, Finding::DanglingSubject { .. })));

        std::fs::remove_dir_all(&project).ok();
    }

    fn write_claimed_proposal(rationale_dir: &Path, record_id: &str, suffix: &str) -> PathBuf {
        let in_review = rationale_dir.join("proposals").join(".in-review");
        std::fs::create_dir_all(&in_review).unwrap();
        let path = in_review.join(format!("{record_id}.yaml.{suffix}"));
        std::fs::write(
            &path,
            format!(
                "id: {record_id}\nkind: constraint\nseverity: high\nstatement: \"x\"\napprovals: []\n"
            ),
        )
        .unwrap();
        path
    }

    /// Defecto real: un proceso que muere entre `claim_proposal` y la
    /// promoción/rechazo deja el archivo en `.in-review/` para siempre —
    /// invisible para `list_pending_detailed` (extensión ya no es `.yaml`)
    /// y sin ningún finding hasta este.
    #[test]
    fn detects_orphaned_claimed_proposal() {
        let project = unique_dir("orphaned-claim");
        let rationale_dir = project.join(".rationale");
        write_claimed_proposal(&rationale_dir, "constraint.orphaned", "12345-0");

        let report = check(&rationale_dir, &project);
        assert!(report.findings.iter().any(|f| matches!(
            f,
            Finding::OrphanedClaimedProposal { record_id, .. }
                if record_id.as_deref() == Some("constraint.orphaned")
        )));

        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn repair_orphaned_claimed_proposal_restores_it_to_proposals() {
        let project = unique_dir("repair-orphaned-claim");
        let rationale_dir = project.join(".rationale");
        write_claimed_proposal(&rationale_dir, "constraint.orphaned", "12345-0");

        let report = check(&rationale_dir, &project);
        let finding = report
            .findings
            .iter()
            .find(|f| matches!(f, Finding::OrphanedClaimedProposal { .. }))
            .unwrap();

        let restored = repair(&rationale_dir, finding, "user:doctor-test").unwrap();
        assert_eq!(
            restored,
            rationale_dir.join("proposals/constraint.orphaned.yaml")
        );
        assert!(restored.exists());

        // Ahora la ve `rationale review` — comprobado con la misma función
        // de listado que usa la interfaz humana.
        let pending = review::list_pending_detailed(&rationale_dir);
        assert!(pending
            .pending
            .iter()
            .any(|p| p.record.id == "constraint.orphaned"));

        std::fs::remove_dir_all(&project).ok();
    }

    /// Nunca sobrescribir en silencio: si una propuesta más nueva con el
    /// mismo id ya llegó después del crash, `repair` debe rechazar en vez
    /// de decidir cuál de las dos gana.
    #[test]
    fn repair_orphaned_claimed_proposal_refuses_to_overwrite_a_newer_one() {
        let project = unique_dir("repair-orphaned-conflict");
        let rationale_dir = project.join(".rationale");
        write_claimed_proposal(&rationale_dir, "constraint.orphaned", "12345-0");
        write_record_yaml(
            &rationale_dir.join("proposals"),
            "constraint.orphaned",
            "id: constraint.orphaned\nkind: constraint\nseverity: high\nstatement: \"y (mas nueva)\"\napprovals: []\n",
        );

        let report = check(&rationale_dir, &project);
        let finding = report
            .findings
            .iter()
            .find(|f| matches!(f, Finding::OrphanedClaimedProposal { .. }))
            .unwrap();

        assert!(repair(&rationale_dir, finding, "user:doctor-test").is_err());
        assert!(rationale_dir
            .join("proposals/constraint.orphaned.yaml")
            .exists());

        std::fs::remove_dir_all(&project).ok();
    }
}
