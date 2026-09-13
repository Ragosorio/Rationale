//! Señales de captura de alto valor y niveles de captura — Rationale_v0.5.md
//! §15.4 y §16.
//!
//! Detección determinista — coincidencia de palabras/paths, nunca un LLM
//! ni una heurística difusa (`policy.no-inferred-blocks.yaml`).
//!
//! vNext: las señales ya no deciden si se escribe memoria. Los "niveles de
//! captura" 0-3 creaban propuestas a partir del diff mismo, justo el ruido
//! que el canon autónomo elimina; ahora la memoria durable nace solo de
//! candidatos explícitos que pasan el gate de `canon`. Las señales quedan
//! como hechos observados del cambio (`finalize_change` las reporta), útiles
//! para el agente y la UI, sin poder de escritura.

use crate::capture::ChangedFile;

/// Rationale_v0.5.md §15.4 — señales de alto valor. `NormativeLanguage` es
/// distinta de las demás: no describe un dominio, describe una FORMA de
/// hablar (`must`, `never`, `because`, `avoid`, `do not`) que suele
/// acompañar una decisión real, en cualquier dominio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Signal {
    Authorization,
    Payments,
    Billing,
    Security,
    DestructiveMigration,
    SchemaChange,
    DeliberateException,
    IrreversibleProcess,
    ExternalIntegration,
    IncidentFix,
    NormativeLanguage,
}

/// Palabras de dominio para cada señal basada en paths, en minúsculas.
/// Coincidencia por substring sobre el path completo — barata, auditable,
/// y explícitamente incompleta (nunca pretende ser una taxonomía cerrada).
///
/// Trade-off de precisión/recall evaluado y aceptado deliberadamente
/// (revisión adversarial de Fase F, hallazgo 4): un archivo cosmético como
/// `auth_helper_unrelated.rs` dispara `Authorization` por substring aunque
/// no tenga relación real. Se evaluó cambiar a coincidencia por palabra
/// completa sobre segmentos del path (mismo patrón que `contains_word`
/// usa para `NORMATIVE_WORDS`) — **se descartó**: tokenizar
/// `src/authorization.rs` da `["src", "authorization", "rs"]`, y ningún
/// token es exactamente `"auth"`, así que ese archivo — un caso real y
/// común de autorización genuina — dejaría de detectarse por completo.
/// La coincidencia por palabra completa habría cambiado un falso positivo
/// conocido por un falso negativo silencioso en el caso más común
/// (perder recall en el nombre compuesto más típico del dominio), sin
/// siquiera arreglar el caso que motivó el cambio (`auth_helper_unrelated`
/// sí tiene `"auth"` como token propio, así que seguiría marcando).
/// Mecanismo aditivo (nunca bloquea, `retrieval::detect_conflict` es el
/// precedente) — el costo del ruido ocasional es menor que perder
/// recall silenciosamente en el caso común.
const PATH_KEYWORDS: &[(Signal, &[&str])] = &[
    (
        Signal::Authorization,
        &["auth", "authz", "permission", "role", "rbac"],
    ),
    (
        Signal::Payments,
        &["payment", "checkout", "billing_charge", "stripe", "invoice"],
    ),
    (Signal::Billing, &["billing", "subscription", "pricing"]),
    (
        Signal::Security,
        &["security", "crypto", "secret", "credential", "token"],
    ),
    (Signal::SchemaChange, &["migration", "schema"]),
    (
        Signal::ExternalIntegration,
        &["webhook", "integration", "provider", "client"],
    ),
];

/// Palabras que activan lenguaje normativo (v0.5 §15.4), en minúsculas,
/// buscadas por palabra completa (no substring) para evitar falsos
/// positivos triviales (`avoid` dentro de `avoidance-list`, por ejemplo).
const NORMATIVE_WORDS: &[&str] = &["must", "never", "because", "avoid", "do not", "must not"];

/// Fragmentos que, dentro de un path de migración, sugieren una operación
/// destructiva (irreversible sin backup) — solo se evalúa si el path ya
/// coincidió con `SchemaChange`.
const DESTRUCTIVE_MIGRATION_MARKERS: &[&str] = &["drop", "truncate", "delete_all", "destroy"];

fn contains_word(haystack: &str, word: &str) -> bool {
    let haystack = haystack.to_lowercase();
    let word = word.to_lowercase();
    haystack
        .split(|c: char| !c.is_alphanumeric())
        .any(|token| token == word)
        || haystack.contains(&format!(" {word} "))
}

/// Señales detectables a partir de los paths cambiados (`capture::diff_since`).
/// Determinista: substring sobre el path en minúsculas, sin ambigüedad.
pub fn signals_from_paths(changed_files: &[ChangedFile]) -> Vec<Signal> {
    let mut found = std::collections::HashSet::new();
    for file in changed_files {
        let path_lower = file.path.to_lowercase();
        for (signal, keywords) in PATH_KEYWORDS {
            if keywords.iter().any(|kw| path_lower.contains(kw)) {
                found.insert(*signal);
            }
        }
        if found.contains(&Signal::SchemaChange)
            && DESTRUCTIVE_MIGRATION_MARKERS
                .iter()
                .any(|marker| path_lower.contains(marker))
        {
            found.insert(Signal::DestructiveMigration);
        }
    }
    found.into_iter().collect()
}

/// Señales detectables a partir de texto libre (mensaje de commit,
/// descripción de PR, o la intención declarada por el agente) — nunca del
/// contenido de código fuente en sí, que corresponde a `signals_from_paths`.
pub fn signals_from_text(text: &str) -> Vec<Signal> {
    let mut found = std::collections::HashSet::new();
    if NORMATIVE_WORDS.iter().any(|w| contains_word(text, w)) {
        found.insert(Signal::NormativeLanguage);
    }

    let lower = text.to_lowercase();
    if lower.contains("rollback")
        || lower.contains("irreversible")
        || lower.contains("cannot be undone")
    {
        found.insert(Signal::IrreversibleProcess);
    }
    if lower.contains("incident") || lower.contains("postmortem") || lower.contains("hotfix") {
        found.insert(Signal::IncidentFix);
    }
    if lower.contains("deliberate") || lower.contains("exception") || lower.contains("except for") {
        found.insert(Signal::DeliberateException);
    }

    found.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::{ChangeOrigin, ChangeType};

    fn changed(path: &str) -> ChangedFile {
        ChangedFile {
            path: path.to_string(),
            change_type: ChangeType::Modified,
            origin: ChangeOrigin::Committed,
        }
    }

    #[test]
    fn detects_authorization_signal_from_path() {
        let signals = signals_from_paths(&[changed("src/auth/authorization.ts")]);
        assert!(signals.contains(&Signal::Authorization));
    }

    #[test]
    fn detects_payments_signal_from_path() {
        let signals = signals_from_paths(&[changed("src/checkout/process_payment.rs")]);
        assert!(signals.contains(&Signal::Payments));
    }

    #[test]
    fn detects_destructive_migration_only_when_schema_and_marker_both_present() {
        let signals = signals_from_paths(&[changed("migrations/2026_drop_legacy_table.sql")]);
        assert!(signals.contains(&Signal::SchemaChange));
        assert!(signals.contains(&Signal::DestructiveMigration));

        let benign = signals_from_paths(&[changed("migrations/2026_add_index.sql")]);
        assert!(benign.contains(&Signal::SchemaChange));
        assert!(!benign.contains(&Signal::DestructiveMigration));
    }

    #[test]
    fn detects_normative_language_in_text() {
        let signals = signals_from_text("Staff users must never receive global super_admin.");
        assert!(signals.contains(&Signal::NormativeLanguage));
    }

    #[test]
    fn does_not_false_positive_on_substring_of_normative_word() {
        // "avoidance" contiene "avoid" como substring pero no como palabra —
        // no debe activar la señal.
        let signals = signals_from_text("Updated the avoidance-list configuration file.");
        assert!(!signals.contains(&Signal::NormativeLanguage));
    }

    #[test]
    fn plain_refactor_with_no_signals_yields_no_normative_language() {
        let signals = signals_from_text("Renamed variable for clarity.");
        assert!(signals.is_empty());
    }
}
