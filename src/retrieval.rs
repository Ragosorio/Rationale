//! Context Compiler — Rationale_v0.5.md §18, §19, Arquitectura §11.11.
//!
//! Compila un `ContextPacket` con los niveles de prioridad de §18.1, un
//! budget explícito, y ranking determinista antes que semántico (§19.1:
//! binding exacto -> vecindad estructural -> scope conceptual -> severidad
//! y autoridad -> FTS; sin embeddings, §28.3). Nunca genera un ensayo — un
//! paquete operativo (Arquitectura §11.11).

use crate::providers::{Coverage, ProviderStatus};
use crate::revision::Consistency;
use crate::storage::Record;
use serde::Serialize;

/// Budget explícito de la consulta (Rationale_v0.5.md §18).
#[derive(Debug, Clone)]
pub struct Budget {
    pub max_tokens: usize,
    pub max_critical_constraints: usize,
    pub max_risks: usize,
}

impl Default for Budget {
    /// Objetivos iniciales del piloto (v0.5 §30): mediana <600 tokens,
    /// p95 <1000. Se usa 600 como default conservador de max_tokens.
    fn default() -> Self {
        Budget {
            max_tokens: 600,
            max_critical_constraints: 5,
            max_risks: 3,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub git_revision: Option<String>,
    pub consistency: String,
    pub provider_status: String,
    pub provider_coverage: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct CriticalConstraint {
    pub id: String,
    pub statement: String,
    pub authority: String,
}

#[derive(Debug, Serialize)]
pub struct ContextPacket {
    pub snapshot: Snapshot,
    /// Nivel 1 (v0.5 §18.1): restricciones críticas aprobadas.
    pub critical_constraints: Vec<CriticalConstraint>,
    /// Nivel 2: conflictos con la intención declarada. Detección
    /// determinista por solapamiento de palabras — nunca semántica ni con
    /// embeddings (§28.3). Vacío si no se declaró intención.
    pub intent_conflicts: Vec<String>,
    /// Nivel 3: razón principal (el `rationale` del primer Record incluido).
    pub primary_reason: Option<String>,
    /// Nivel 4: riesgos conocidos directamente relevantes.
    pub known_risks: Vec<String>,
    /// Nivel 5: estructura afectada (target resuelto + bindings conocidos).
    pub affected_targets: Vec<String>,
    /// Nivel 6: historia expandible — solo el conteo, no el contenido
    /// (progressive disclosure, v0.5 §18.2).
    pub additional_history_available: usize,
    pub resolved_target: Option<String>,
    pub warnings: Vec<String>,
    /// Proxy de tokens (chars/4) del contenido de texto incluido —
    /// instrumentación, no una medición exacta de tokenizer real
    /// (Arquitectura §20.3: "si los tokens no están disponibles, se
    /// registrarán proxies").
    pub token_estimate: usize,
}

fn token_estimate(text: &str) -> usize {
    (text.chars().count() / 4).max(1)
}

fn authority_label(record: &Record) -> &'static str {
    if crate::storage::has_approved_authority(record) {
        "approved"
    } else {
        "unreviewed"
    }
}

/// Recuperación determinista (v0.5 §19.1): filtra por kind/severity, ordena
/// con autoridad aprobada primero y luego por ID (desempate estable), sin
/// ninguna señal semántica.
fn select_critical_constraints<'a>(records: &'a [Record], budget: &Budget) -> Vec<&'a Record> {
    let mut critical: Vec<&Record> = records
        .iter()
        .filter(|r| r.kind == "constraint" && r.severity == "critical")
        .collect();
    critical.sort_by(|a, b| {
        let a_approved = crate::storage::has_approved_authority(a);
        let b_approved = crate::storage::has_approved_authority(b);
        b_approved.cmp(&a_approved).then_with(|| a.id.cmp(&b.id))
    });
    critical.truncate(budget.max_critical_constraints);
    critical
}

/// Nivel 2: conflicto determinista por solapamiento de palabras
/// "significativas" (longitud > 3) entre la intención y el statement de
/// una constraint. Deliberadamente crudo — nunca pretende comprensión
/// semántica, solo una señal barata y auditable.
fn detect_conflict(intent: &str, constraint_statement: &str) -> bool {
    let intent_words: std::collections::HashSet<String> = intent
        .to_lowercase()
        .split_whitespace()
        .filter(|w| w.len() > 3)
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|w| !w.is_empty())
        .collect();
    let statement_words: std::collections::HashSet<String> = constraint_statement
        .to_lowercase()
        .split_whitespace()
        .filter(|w| w.len() > 3)
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|w| !w.is_empty())
        .collect();
    let overlap = intent_words.intersection(&statement_words).count();
    overlap >= 2
}

#[allow(clippy::too_many_arguments)]
pub fn compile_packet(
    git_head: Option<String>,
    consistency: Consistency,
    provider_status: ProviderStatus,
    provider_coverage: Coverage,
    records: &[Record],
    intent: Option<&str>,
    resolved_target: Option<String>,
    provider_warnings: Vec<String>,
    budget: &Budget,
) -> ContextPacket {
    let provider_status_str = match provider_status {
        ProviderStatus::Successful => "successful",
        ProviderStatus::Degraded => "degraded",
        ProviderStatus::Unavailable => "unavailable",
    };
    let coverage_str = match provider_coverage {
        Coverage::Complete => "complete",
        Coverage::Partial => "partial",
        Coverage::Unknown => "unknown",
    };

    let selected = select_critical_constraints(records, budget);

    let critical_constraints: Vec<CriticalConstraint> = selected
        .iter()
        .map(|r| CriticalConstraint {
            id: r.id.clone(),
            statement: r.statement.clone(),
            authority: authority_label(r).to_string(),
        })
        .collect();

    let intent_conflicts: Vec<String> = match intent {
        Some(text) => selected
            .iter()
            .filter(|r| detect_conflict(text, &r.statement))
            .map(|r| {
                format!(
                    "La intención puede entrar en conflicto con '{}': {}",
                    r.id, r.statement
                )
            })
            .collect(),
        None => vec![],
    };

    let primary_reason = selected.first().and_then(|r| r.rationale.clone());

    let mut known_risks: Vec<String> = selected
        .iter()
        .flat_map(|r| r.risks.iter().map(|risk| risk.statement.clone()))
        .collect();
    known_risks.truncate(budget.max_risks);

    let mut affected_targets: Vec<String> = Vec::new();
    if let Some(t) = &resolved_target {
        affected_targets.push(t.clone());
    }
    for r in &selected {
        for binding in &r.binding_declarations {
            if let Some(path) = &binding.path_hint {
                if !affected_targets.contains(path) {
                    affected_targets.push(path.clone());
                }
            }
        }
    }

    let total_critical_matching = records
        .iter()
        .filter(|r| r.kind == "constraint" && r.severity == "critical")
        .count();
    let mut additional_history_available = total_critical_matching.saturating_sub(selected.len());

    // Nivel 0-3 (salud, constraints críticas, conflictos, razón principal)
    // nunca se recortan por presupuesto — v0.5 §30.1.7: omitir una
    // constraint crítica invalida el paquete aunque parezca "eficiente".
    // Solo los niveles 4-6 se recortan progresivamente si el estimado de
    // tokens excede el budget.
    let protected_text: String = critical_constraints
        .iter()
        .map(|c| c.statement.as_str())
        .chain(intent_conflicts.iter().map(|s| s.as_str()))
        .chain(primary_reason.iter().map(|s| s.as_str()))
        .collect::<Vec<_>>()
        .join(" ");
    let protected_tokens = token_estimate(&protected_text);

    while protected_tokens
        + token_estimate(&known_risks.join(" "))
        + token_estimate(&affected_targets.join(" "))
        > budget.max_tokens
    {
        if !affected_targets.is_empty() {
            affected_targets.pop();
        } else if !known_risks.is_empty() {
            known_risks.pop();
        } else {
            break;
        }
    }
    if known_risks.len()
        < selected
            .iter()
            .flat_map(|r| r.risks.iter())
            .count()
            .min(budget.max_risks)
    {
        additional_history_available += 1;
    }

    let token_estimate_total = protected_tokens
        + token_estimate(&known_risks.join(" "))
        + token_estimate(&affected_targets.join(" "));

    ContextPacket {
        snapshot: Snapshot {
            git_revision: git_head,
            consistency: consistency.to_string(),
            provider_status: provider_status_str.to_string(),
            provider_coverage: coverage_str.to_string(),
        },
        critical_constraints,
        intent_conflicts,
        primary_reason,
        known_risks,
        affected_targets,
        additional_history_available,
        resolved_target,
        warnings: provider_warnings,
        token_estimate: token_estimate_total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Approval, BindingDeclaration, EpistemicStatus, Record, Risk};

    fn fixed_record(id: &str, approved: bool) -> Record {
        Record {
            id: id.to_string(),
            kind: "constraint".to_string(),
            severity: "critical".to_string(),
            statement: "Golden packet statement.".to_string(),
            rationale: Some("Because golden reasons.".to_string()),
            epistemic_status: EpistemicStatus::Stated,
            evidence: vec![],
            risks: vec![Risk {
                id: "risk.golden".to_string(),
                statement: "Golden risk statement.".to_string(),
                epistemic_status: EpistemicStatus::Stated,
            }],
            approvals: if approved {
                vec![Approval {
                    actor: "user:security-owner".to_string(),
                    authority: "security-owner".to_string(),
                    status: "approved".to_string(),
                }]
            } else {
                vec![]
            },
            binding_declarations: vec![BindingDeclaration {
                id: "binding.golden".to_string(),
                kind: "symbol".to_string(),
                provider: Some("codebase-memory".to_string()),
                structural_id: Some("function:typescript:golden".to_string()),
                path_hint: Some("src/golden.ts".to_string()),
            }],
            bound_revision: Some("abc123fixed".to_string()),
            subject: None,
        }
    }

    /// D5/E6 — "golden packet": mismos inputs fijos deben producir SIEMPRE
    /// el mismo JSON, byte a byte (Arquitectura §19.4).
    #[test]
    fn golden_packet_is_byte_for_byte_deterministic() {
        let records = vec![fixed_record("constraint.golden-test", true)];
        let packet = compile_packet(
            Some("abc123fixed".to_string()),
            Consistency::Exact,
            ProviderStatus::Successful,
            Coverage::Complete,
            &records,
            None,
            Some("golden.qualifiedName".to_string()),
            vec![],
            &Budget::default(),
        );

        let json = serde_json::to_string(&packet).unwrap();
        let expected = r#"{"snapshot":{"git_revision":"abc123fixed","consistency":"exact","provider_status":"successful","provider_coverage":"complete"},"critical_constraints":[{"id":"constraint.golden-test","statement":"Golden packet statement.","authority":"approved"}],"intent_conflicts":[],"primary_reason":"Because golden reasons.","known_risks":["Golden risk statement."],"affected_targets":["golden.qualifiedName","src/golden.ts"],"additional_history_available":0,"resolved_target":"golden.qualifiedName","warnings":[],"token_estimate":25}"#;
        assert_eq!(json, expected);
    }

    #[test]
    fn budget_caps_critical_constraints() {
        let records: Vec<Record> = (0..10)
            .map(|i| fixed_record(&format!("constraint.many-{i}"), true))
            .collect();
        let budget = Budget {
            max_critical_constraints: 3,
            ..Budget::default()
        };
        let packet = compile_packet(
            None,
            Consistency::Unresolved,
            ProviderStatus::Unavailable,
            Coverage::Unknown,
            &records,
            None,
            None,
            vec![],
            &budget,
        );
        assert_eq!(packet.critical_constraints.len(), 3);
        assert_eq!(packet.additional_history_available, 7);
    }

    #[test]
    fn unapproved_record_is_never_exposed_as_approved() {
        let records = vec![fixed_record("constraint.unreviewed", false)];
        let packet = compile_packet(
            None,
            Consistency::Unresolved,
            ProviderStatus::Unavailable,
            Coverage::Unknown,
            &records,
            None,
            None,
            vec![],
            &Budget::default(),
        );
        assert_eq!(packet.critical_constraints[0].authority, "unreviewed");
    }

    #[test]
    fn approved_records_sort_before_unreviewed() {
        let records = vec![
            fixed_record("constraint.b-unreviewed", false),
            fixed_record("constraint.a-approved", true),
        ];
        let packet = compile_packet(
            None,
            Consistency::Unresolved,
            ProviderStatus::Unavailable,
            Coverage::Unknown,
            &records,
            None,
            None,
            vec![],
            &Budget::default(),
        );
        assert_eq!(packet.critical_constraints[0].id, "constraint.a-approved");
        assert_eq!(packet.critical_constraints[1].id, "constraint.b-unreviewed");
    }

    #[test]
    fn intent_conflict_detected_by_word_overlap() {
        let records = vec![fixed_record("constraint.golden-test", true)];
        let packet = compile_packet(
            None,
            Consistency::Unresolved,
            ProviderStatus::Unavailable,
            Coverage::Unknown,
            &records,
            Some("I want to change the golden packet statement behavior"),
            None,
            vec![],
            &Budget::default(),
        );
        assert_eq!(packet.intent_conflicts.len(), 1);
    }

    #[test]
    fn no_intent_means_no_conflicts_reported() {
        let records = vec![fixed_record("constraint.golden-test", true)];
        let packet = compile_packet(
            None,
            Consistency::Unresolved,
            ProviderStatus::Unavailable,
            Coverage::Unknown,
            &records,
            None,
            None,
            vec![],
            &Budget::default(),
        );
        assert!(packet.intent_conflicts.is_empty());
    }

    /// Nivel 0-3 nunca se recorta por presupuesto, aunque el budget de
    /// tokens sea extremadamente pequeño — solo los niveles 4-6 ceden.
    #[test]
    fn tiny_budget_never_drops_critical_constraints() {
        let records = vec![fixed_record("constraint.golden-test", true)];
        let tiny_budget = Budget {
            max_tokens: 1,
            max_critical_constraints: 5,
            max_risks: 3,
        };
        let packet = compile_packet(
            None,
            Consistency::Unresolved,
            ProviderStatus::Unavailable,
            Coverage::Unknown,
            &records,
            None,
            None,
            vec![],
            &tiny_budget,
        );
        assert_eq!(packet.critical_constraints.len(), 1);
        assert!(packet.known_risks.is_empty());
        assert!(packet.affected_targets.is_empty());
    }
}
