//! Subject — Rationale_v0.5.md §5.1, Arquitectura §11.8 (Subject Resolver).
//!
//! El Subject es la identidad conceptual estable de un comportamiento
//! gobernado. Los 7 Subjects fundacionales de Fase A y los 2 Subjects de
//! gobernanza de F8/Fase G viven en `.rationale/subjects/`.
//!
//! Fase F4 completa la resolución determinista de `v0.5 §9.1` (orden
//! obligatorio):
//!
//! ```text
//! 1. ID o alias exacto.                          -> resolve_by_id_or_alias
//! 2. Nombre normalizado dentro del mismo scope.   -> resolve (candidatos)
//! 3. Overlap de bindings estructurales.           -> resolve (candidatos)
//! 4. Relación parent/child y compatibilidad de scope. -> resolve (candidatos)
//! 5. Búsqueda FTS por título/descripción.         -> resolve (candidatos, léxico)
//! 6. Similitud semántica local.                   -> diferido, §28.3 (embeddings)
//! 7. Revisión de candidatos y decisión explícita. -> rationale review, Fase F6
//! ```
//!
//! `resolve()` nunca decide sola por un agente: produce candidatos con
//! señales explícitas y una acción *sugerida*, siempre revisable por un
//! humano. Solo el paso 1 (coincidencia exacta) es lo bastante inequívoco
//! para no necesitar candidatos.

use serde::Deserialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Clone)]
pub struct Subject {
    pub id: String,
    #[serde(rename = "type")]
    pub subject_type: String,
    pub title: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub applies_to: Vec<String>,
}

#[derive(Debug)]
pub enum SubjectError {
    Io(std::io::Error),
    Parse(String),
    MissingRequiredField(&'static str),
}

impl std::fmt::Display for SubjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubjectError::Io(e) => write!(f, "error de I/O leyendo Subject: {e}"),
            SubjectError::Parse(e) => write!(f, "Subject YAML inválido: {e}"),
            SubjectError::MissingRequiredField(field) => {
                write!(f, "Subject inválido: falta campo obligatorio '{field}'")
            }
        }
    }
}

pub fn read_subject(path: &Path) -> Result<Subject, SubjectError> {
    let content = std::fs::read_to_string(path).map_err(SubjectError::Io)?;
    let subject: Subject =
        yaml_serde::from_str(&content).map_err(|e| SubjectError::Parse(e.to_string()))?;

    if subject.id.is_empty() {
        return Err(SubjectError::MissingRequiredField("id"));
    }
    if subject.title.is_empty() {
        return Err(SubjectError::MissingRequiredField("title"));
    }
    Ok(subject)
}

/// Resultado detallado de listar Subjects: los que sí parsearon, y los que
/// no (con el path y el error) — nunca se descartan en silencio. Ver
/// `list_subjects_detailed`.
pub struct SubjectListResult {
    pub subjects: Vec<Subject>,
    pub skipped: Vec<(PathBuf, SubjectError)>,
}

/// Lista todos los Subjects en `.rationale/subjects/`, reportando qué
/// archivos no pudieron leerse (E7/revisión adversarial de Fase F, hallazgo
/// 1: la versión anterior abortaba la lectura COMPLETA ante un solo archivo
/// corrupto — apagando el Subject Resolver entero para todo el canon
/// existente, en silencio, para cualquier caller que usara
/// `.unwrap_or_default()`). Un archivo que no parsea o le falta un campo
/// obligatorio se salta — nunca descarta los demás Subjects ya leídos.
pub fn list_subjects_detailed(subjects_dir: &Path) -> Result<SubjectListResult, SubjectError> {
    let mut subjects = Vec::new();
    let mut skipped = Vec::new();
    if !subjects_dir.is_dir() {
        return Ok(SubjectListResult { subjects, skipped });
    }
    let entries = std::fs::read_dir(subjects_dir).map_err(SubjectError::Io)?;
    for entry in entries {
        let entry = entry.map_err(SubjectError::Io)?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
            match read_subject(&path) {
                Ok(subject) => subjects.push(subject),
                Err(e) => skipped.push((path, e)),
            }
        }
    }
    Ok(SubjectListResult { subjects, skipped })
}

/// Variante simple para callers que no necesitan saber qué se saltó (la
/// mayoría). Nunca fallar por UN archivo corrupto es la garantía real —
/// solo falla si el directorio mismo no pudo leerse (I/O real).
pub fn list_subjects(subjects_dir: &Path) -> Result<Vec<Subject>, SubjectError> {
    Ok(list_subjects_detailed(subjects_dir)?.subjects)
}

/// Resuelve un Subject por ID exacto o alias — paso 1 de
/// `Rationale_v0.5.md §9.1` ("Subject Resolver").
pub fn resolve_by_id_or_alias<'a>(
    subjects: &'a [Subject],
    id_or_alias: &str,
) -> Option<&'a Subject> {
    subjects
        .iter()
        .find(|s| s.id == id_or_alias || s.aliases.iter().any(|a| a == id_or_alias))
}

/// Acción sugerida por el Subject Resolver (v0.5 §9.1, ejemplo de
/// `resolution:`). Siempre una sugerencia — la decisión final es de
/// `rationale review` (Fase F6), nunca automática (paso 7 del orden).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionAction {
    Reuse,
    Create,
    Alias,
    MergeCandidate,
    // SplitCandidate nunca lo sugiere `resolve()` automáticamente — detectar
    // que un Subject existente es demasiado amplio y debería dividirse
    // requiere juicio humano, no una señal determinista disponible aquí.
    // Se deja modelado para que `rationale review` (F6) pueda asignarlo.
    #[allow(dead_code)]
    SplitCandidate,
}

/// Señales de un candidato — mismos nombres que el ejemplo de
/// `Rationale_v0.5.md §9.1` (`binding_overlap`, `lexical_similarity`,
/// `scope_compatible`). `semantic_similarity` no se calcula (§28.3 difiere
/// embeddings); el campo no existe aquí a propósito, no como `None` — no
/// pretender una señal que no se produce.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CandidateSignals {
    pub binding_overlap: f64,
    pub lexical_similarity: f64,
    pub scope_compatible: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Candidate {
    pub id: String,
    pub signals: CandidateSignals,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DifferenceKind {
    Behavior,
    Scope,
    Lifecycle,
    Authority,
    Invariant,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct NoveltyReason {
    pub contrasted_subject: String,
    pub difference_kind: DifferenceKind,
    pub difference: String,
    pub evidence: String,
}

#[derive(Debug, serde::Serialize)]
pub struct Resolution {
    pub action: ResolutionAction,
    pub selected_subject: Option<String>,
    pub candidates: Vec<Candidate>,
    /// `Rationale_v0.5.md §294`: una justificación libre ("este Subject es
    /// diferente") no demuestra novedad. Este campo lo puebla el CALLER
    /// (`finalize_change`, Fase F5) cuando decide proponer `Create` pese a
    /// haber candidatos fuertes — `resolve()` nunca lo redacta por sí sola,
    /// solo expone los candidatos contra los que debe contrastar.
    pub novelty_reason: Option<NoveltyReason>,
}

pub fn validate_novelty_reason(
    reason: &NoveltyReason,
    candidates: &[Candidate],
) -> Result<(), String> {
    if reason.contrasted_subject.trim().is_empty() {
        return Err("contrasted_subject no puede estar vacío".to_string());
    }
    if !candidates
        .iter()
        .any(|candidate| candidate.id == reason.contrasted_subject)
    {
        return Err(format!(
            "contrasted_subject '{}' no corresponde a ningún candidato real del Resolver",
            reason.contrasted_subject
        ));
    }
    if reason.difference.trim().is_empty() {
        return Err("difference no puede estar vacío".to_string());
    }
    if reason.evidence.trim().is_empty() {
        return Err("evidence no puede estar vacío".to_string());
    }
    Ok(())
}

// Umbrales sin calibrar contra datos reales — límite conocido, confirmado
// con contraejemplos concretos por la revisión adversarial de Fase F
// (`docs/work-items/adversarial-review-fase-f.md`, hallazgos 6 y 7):
//
// - Falso positivo (hallazgo 6): dos constraints reales y distintos que
//   comparten una plantilla de redacción larga (común en organizaciones
//   con convenciones de escritura consistentes) alcanzan 0.86 de Jaccard
//   crudo — por encima de ALIAS_SIMILARITY_THRESHOLD — y BLOQUEAN la
//   propuesta completa sin novelty_reason, aunque describan conceptos
//   genuinamente distintos.
// - Falso negativo (hallazgo 7): el mismo concepto real (ej. "no double
//   billing") expresado con vocabulario distinto no supera
//   CANDIDATE_MIN_THRESHOLD — no aparece como candidato, fragmentando el
//   canon en silencio.
//
// Filtrar stopwords estructurales ("ensure", "that", "the", "never",
// "allows"...) antes del Jaccard mitigaría el hallazgo 6 sin resolver el 7
// (son fallas en direcciones opuestas del mismo mecanismo). No se aplica
// sin evidencia real de cuán comunes son las plantillas repetidas en
// Records reales — ajustar umbrales sin esa evidencia arriesga cambiar
// un falso positivo conocido por un falso negativo nuevo sin poder medir
// la mejora real. Ver revisit trigger.
const SIGNIFICANT_SIMILARITY_THRESHOLD: f64 = 0.6;
const ALIAS_SIMILARITY_THRESHOLD: f64 = 0.85;
const CANDIDATE_MIN_THRESHOLD: f64 = 0.2;

fn normalized_tokens(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect()
}

/// Similitud léxica determinista (Jaccard sobre tokens normalizados) — el
/// mismo enfoque ya validado en `retrieval::detect_conflict`: recall barato
/// y auditable, nunca comprensión semántica (paso 5, FTS/título; el `id`
/// normalizado hace las veces del paso 2, "nombre normalizado").
fn lexical_similarity(a: &str, b: &str) -> f64 {
    let tokens_a = normalized_tokens(a);
    let tokens_b = normalized_tokens(b);
    if tokens_a.is_empty() || tokens_b.is_empty() {
        return 0.0;
    }
    let intersection = tokens_a.intersection(&tokens_b).count();
    let union = tokens_a.union(&tokens_b).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

fn binding_overlap(subject_bindings: &HashSet<String>, proposed_bindings: &HashSet<String>) -> f64 {
    if subject_bindings.is_empty() || proposed_bindings.is_empty() {
        return 0.0;
    }
    let intersection = subject_bindings.intersection(proposed_bindings).count();
    let union = subject_bindings.union(proposed_bindings).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Resolución completa (pasos 2-5 de `v0.5 §9.1`; el paso 1 ya lo cubre
/// `resolve_by_id_or_alias` y se reintenta aquí primero para short-circuit).
///
/// `existing_bindings`: para cada Subject ya existente, los `path_hint` de
/// los `binding_declarations` de los Records que lo referencian — lo
/// calcula el caller a partir de `storage::list_records` (este módulo no
/// depende de qué Records existen, solo recibe la señal ya agregada).
pub fn resolve(
    subjects: &[Subject],
    proposed_id: &str,
    proposed_title: &str,
    proposed_scope: &str,
    proposed_bindings: &[String],
    existing_bindings: &std::collections::HashMap<String, Vec<String>>,
) -> Resolution {
    if let Some(exact) = resolve_by_id_or_alias(subjects, proposed_id) {
        return Resolution {
            action: ResolutionAction::Reuse,
            selected_subject: Some(exact.id.clone()),
            candidates: vec![],
            novelty_reason: None,
        };
    }

    let proposed_binding_set: HashSet<String> = proposed_bindings.iter().cloned().collect();
    let empty = Vec::new();

    let mut candidates: Vec<Candidate> = subjects
        .iter()
        .map(|s| {
            let subject_bindings: HashSet<String> = existing_bindings
                .get(&s.id)
                .unwrap_or(&empty)
                .iter()
                .cloned()
                .collect();
            let lexical = lexical_similarity(proposed_title, &s.title)
                .max(lexical_similarity(proposed_id, &s.id));
            let overlap = binding_overlap(&subject_bindings, &proposed_binding_set);
            Candidate {
                id: s.id.clone(),
                signals: CandidateSignals {
                    binding_overlap: overlap,
                    lexical_similarity: lexical,
                    scope_compatible: s.scope == proposed_scope,
                },
            }
        })
        .filter(|c| {
            c.signals.binding_overlap.max(c.signals.lexical_similarity) >= CANDIDATE_MIN_THRESHOLD
        })
        .collect();

    candidates.sort_by(|a, b| {
        let score_a = a.signals.binding_overlap.max(a.signals.lexical_similarity);
        let score_b = b.signals.binding_overlap.max(b.signals.lexical_similarity);
        score_b
            .partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let strongest = candidates.first();
    let action = match strongest {
        Some(top)
            if top.signals.scope_compatible
                && top
                    .signals
                    .binding_overlap
                    .max(top.signals.lexical_similarity)
                    >= ALIAS_SIMILARITY_THRESHOLD =>
        {
            ResolutionAction::Alias
        }
        Some(top)
            if top
                .signals
                .binding_overlap
                .max(top.signals.lexical_similarity)
                >= SIGNIFICANT_SIMILARITY_THRESHOLD =>
        {
            ResolutionAction::MergeCandidate
        }
        _ => ResolutionAction::Create,
    };

    let selected_subject = match action {
        ResolutionAction::Alias | ResolutionAction::MergeCandidate => {
            strongest.map(|c| c.id.clone())
        }
        _ => None,
    };

    Resolution {
        action,
        selected_subject,
        candidates,
        novelty_reason: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subject(id: &str, title: &str, scope: &str) -> Subject {
        Subject {
            id: id.to_string(),
            subject_type: "system-behavior".to_string(),
            title: title.to_string(),
            scope: scope.to_string(),
            aliases: vec![],
            applies_to: vec![],
        }
    }

    #[test]
    fn resolve_returns_reuse_for_exact_id_match() {
        let subjects = vec![subject(
            "authorization.entity-scoped-staff-access",
            "Entity-scoped staff authorization",
            "project",
        )];
        let resolution = resolve(
            &subjects,
            "authorization.entity-scoped-staff-access",
            "anything",
            "project",
            &[],
            &std::collections::HashMap::new(),
        );
        assert_eq!(resolution.action, ResolutionAction::Reuse);
        assert_eq!(
            resolution.selected_subject,
            Some("authorization.entity-scoped-staff-access".to_string())
        );
        assert!(resolution.candidates.is_empty());
    }

    #[test]
    fn resolve_suggests_alias_for_near_identical_title_same_scope() {
        let subjects = vec![subject(
            "authorization.entity-scoped-staff-access",
            "Entity scoped staff authorization access",
            "project",
        )];
        // ID distinto (dos agentes crearon nombres distintos para el mismo
        // concepto — v0.5 §9.1, ejemplo exacto), mismo título exacto (más
        // allá de mayúsculas/orden de tokens, que es lo que la similitud
        // léxica por Jaccard normaliza).
        let resolution = resolve(
            &subjects,
            "auth.staff-per-entity-permissions",
            "Entity scoped staff authorization access",
            "project",
            &[],
            &std::collections::HashMap::new(),
        );
        assert_eq!(resolution.action, ResolutionAction::Alias);
        assert_eq!(
            resolution.selected_subject,
            Some("authorization.entity-scoped-staff-access".to_string())
        );
        assert_eq!(resolution.candidates.len(), 1);
    }

    #[test]
    fn resolve_suggests_create_when_no_similar_candidates() {
        let subjects = vec![subject(
            "architecture.provider-boundary",
            "Structural provider boundary and adapter contract",
            "project",
        )];
        let resolution = resolve(
            &subjects,
            "payments.no-double-charge",
            "Payments must never be processed twice for the same order",
            "project",
            &[],
            &std::collections::HashMap::new(),
        );
        assert_eq!(resolution.action, ResolutionAction::Create);
        assert!(resolution.selected_subject.is_none());
        assert!(resolution.candidates.is_empty());
    }

    #[test]
    fn resolve_detects_binding_overlap_as_a_candidate_signal() {
        let subjects = vec![subject(
            "authorization.entity-scoped-staff-access",
            "Entity-scoped staff authorization",
            "project",
        )];
        let mut existing_bindings = std::collections::HashMap::new();
        existing_bindings.insert(
            "authorization.entity-scoped-staff-access".to_string(),
            vec!["src/auth/authorization.ts".to_string()],
        );

        // Título deliberadamente sin relación léxica, pero el mismo binding
        // estructural — debe surgir como candidato por overlap, no por texto.
        let resolution = resolve(
            &subjects,
            "some.unrelated-name",
            "Completely unrelated wording",
            "project",
            &["src/auth/authorization.ts".to_string()],
            &existing_bindings,
        );
        assert_eq!(resolution.candidates.len(), 1);
        assert_eq!(resolution.candidates[0].signals.binding_overlap, 1.0);
    }

    #[test]
    fn resolve_never_auto_suggests_split_candidate() {
        // SplitCandidate requiere juicio humano — resolve() nunca debe
        // producirlo por sí sola, sin importar la combinación de señales.
        let subjects = vec![
            subject("a.one", "Alpha behavior one", "project"),
            subject("a.two", "Alpha behavior two", "project"),
            subject("a.three", "Alpha behavior three", "workspace"),
        ];
        for candidate_title in [
            "Alpha behavior",
            "Alpha behavior one two three",
            "Completely different",
        ] {
            let resolution = resolve(
                &subjects,
                "new.id",
                candidate_title,
                "project",
                &[],
                &std::collections::HashMap::new(),
            );
            assert_ne!(resolution.action, ResolutionAction::SplitCandidate);
        }
    }

    #[test]
    fn reads_real_subject() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(".rationale/subjects/architecture.provider-boundary.yaml");
        let subject = read_subject(&path).unwrap();
        assert_eq!(subject.id, "architecture.provider-boundary");
        assert_eq!(subject.subject_type, "system-behavior");
    }

    #[test]
    fn lists_all_nine_foundational_subjects_after_f8_governance() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(".rationale/subjects");
        let subjects = list_subjects(&dir).unwrap();
        assert_eq!(
            subjects.len(),
            9,
            "deben existir los 7 Subjects de Fase A más los 2 de gobernanza F8/Fase G"
        );
    }

    #[test]
    fn resolves_by_alias() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(".rationale/subjects");
        let subjects = list_subjects(&dir).unwrap();
        let found = resolve_by_id_or_alias(&subjects, "architecture.codebase-memory-boundary");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "architecture.provider-boundary");
    }

    #[test]
    fn resolve_returns_none_for_unknown() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(".rationale/subjects");
        let subjects = list_subjects(&dir).unwrap();
        assert!(resolve_by_id_or_alias(&subjects, "no.existe").is_none());
    }

    #[test]
    fn novelty_reason_requires_a_real_candidate_and_auditable_difference() {
        let subjects = vec![Subject {
            id: "auth.existing".to_string(),
            subject_type: "system-behavior".to_string(),
            title: "Existing authorization".to_string(),
            scope: "project".to_string(),
            aliases: vec![],
            applies_to: vec![],
        }];
        let resolution = resolve(
            &subjects,
            "auth.new",
            "Existing authorization",
            "project",
            &[],
            &std::collections::HashMap::new(),
        );
        let valid = NoveltyReason {
            contrasted_subject: "auth.existing".to_string(),
            difference_kind: DifferenceKind::Scope,
            difference: "The new rule applies only to external tenants.".to_string(),
            evidence: "The changed binding is in the tenant gateway module.".to_string(),
        };
        assert!(validate_novelty_reason(&valid, &resolution.candidates).is_ok());

        let unknown = NoveltyReason {
            contrasted_subject: "auth.missing".to_string(),
            ..valid.clone()
        };
        assert!(validate_novelty_reason(&unknown, &resolution.candidates).is_err());

        let empty = NoveltyReason {
            difference: " ".to_string(),
            ..valid
        };
        assert!(validate_novelty_reason(&empty, &resolution.candidates).is_err());
    }
}
