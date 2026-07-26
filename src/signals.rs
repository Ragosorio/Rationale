//! Señales de captura de alto valor y niveles de captura — Rationale_v0.5.md
//! §15.4 y §16.
//!
//! Rationale no pregunta por todo cambio; activa captura asistida solo
//! cuando detecta señales concretas. La detección es determinista —
//! coincidencia de palabras/paths, nunca un LLM ni una heurística difusa
//! (`policy.no-inferred-blocks.yaml`: ninguna inferencia decide bloqueo o
//! captura por "parecer importante"). Este módulo solo detecta y clasifica
//! el nivel candidato; nunca escribe nada (eso es `finalize_change`, F5) ni
//! asigna autoridad (eso requiere `rationale review`, F6).

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

/// Rationale_v0.5.md §16 — a qué nivel de captura corresponde un cambio.
/// **`CriticalInvariant` nunca lo asigna esta función** — v0.5 §16 exige
/// autoridad aprobada o policy, scope explícito y evidencia para ese nivel,
/// ninguno alcanzable por un agente solo; solo `rationale review` (F6) puede
/// promover una propuesta hasta ahí.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CaptureLevel {
    /// Formato, renombres, dependencias menores, cambios mecánicos — no se
    /// crea ningún registro.
    GitOnly,
    Intent,
    Decision,
    OperationalKnowledge,
}

/// Extensiones/paths cuyo cambio, en ausencia de cualquier señal, nunca por
/// sí solo justifica una propuesta (v0.5 §16, Nivel 0). Deliberadamente
/// corta y ampliable — la garantía real es que la AUSENCIA de una lista
/// exhaustiva nunca bloquea captura real (si hay señales, este chequeo ni
/// se consulta).
fn is_mechanical_only_path(path: &str) -> bool {
    const MECHANICAL_SUFFIXES: &[&str] = &[
        ".lock",
        "Cargo.lock",
        "package-lock.json",
        "yarn.lock",
        "pnpm-lock.yaml",
        ".gitignore",
        ".editorconfig",
    ];
    MECHANICAL_SUFFIXES.iter().any(|s| path.ends_with(s))
}

/// Determina el nivel candidato — nunca el nivel final: `rationale review`
/// (Fase F6) es quien decide si una propuesta de nivel `Decision` sube a
/// `CriticalInvariant` con autoridad real.
pub fn determine_level(changed_files: &[ChangedFile], signals: &[Signal]) -> CaptureLevel {
    if signals.is_empty()
        && !changed_files.is_empty()
        && changed_files
            .iter()
            .all(|f| is_mechanical_only_path(&f.path))
    {
        return CaptureLevel::GitOnly;
    }

    let has_domain_signal = signals.iter().any(|s| {
        matches!(
            s,
            Signal::Authorization
                | Signal::Payments
                | Signal::Billing
                | Signal::Security
                | Signal::DestructiveMigration
        )
    });
    let has_normative_language = signals.contains(&Signal::NormativeLanguage);

    if has_domain_signal && has_normative_language {
        return CaptureLevel::OperationalKnowledge;
    }
    if has_normative_language || has_domain_signal {
        return CaptureLevel::Decision;
    }
    if signals.is_empty() {
        // Sin ninguna señal de alto valor pero tampoco puramente mecánico
        // (p. ej. cambios en código de aplicación sin lenguaje normativo):
        // Nivel 1, el mínimo que registra intención sin sobre-preguntar.
        return CaptureLevel::Intent;
    }
    CaptureLevel::Decision
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::ChangeType;

    fn changed(path: &str) -> ChangedFile {
        ChangedFile {
            path: path.to_string(),
            change_type: ChangeType::Modified,
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

    #[test]
    fn mechanical_only_changes_with_no_signals_are_level_zero() {
        let level = determine_level(&[changed("Cargo.lock"), changed("package-lock.json")], &[]);
        assert_eq!(level, CaptureLevel::GitOnly);
    }

    #[test]
    fn any_signal_prevents_level_zero_even_with_lockfile_changes() {
        let level = determine_level(&[changed("Cargo.lock")], &[Signal::Security]);
        assert_ne!(level, CaptureLevel::GitOnly);
    }

    #[test]
    fn domain_signal_plus_normative_language_is_operational_knowledge() {
        let level = determine_level(
            &[changed("src/auth/authorization.ts")],
            &[Signal::Authorization, Signal::NormativeLanguage],
        );
        assert_eq!(level, CaptureLevel::OperationalKnowledge);
    }

    #[test]
    fn domain_signal_alone_is_decision() {
        let level = determine_level(
            &[changed("src/auth/authorization.ts")],
            &[Signal::Authorization],
        );
        assert_eq!(level, CaptureLevel::Decision);
    }

    #[test]
    fn plain_code_change_with_no_signals_is_at_least_intent() {
        let level = determine_level(&[changed("src/utils/format.rs")], &[]);
        assert_eq!(level, CaptureLevel::Intent);
    }

    #[test]
    fn determine_level_never_returns_critical_invariant() {
        // Ningún combinación de señales debe alcanzar CriticalInvariant —
        // ese nivel solo lo asigna rationale review (F6) con autoridad real.
        // (No hay variante CriticalInvariant en CaptureLevel: si se agregara
        // sin actualizar esta función, este test seguiría compilando y
        // pasando, así que la garantía real es estructural: el enum de esta
        // fase no tiene ese variante en absoluto.)
        let level = determine_level(
            &[changed("src/payments/process.rs")],
            &[
                Signal::Payments,
                Signal::NormativeLanguage,
                Signal::Security,
            ],
        );
        assert_eq!(level, CaptureLevel::OperationalKnowledge);
    }
}
