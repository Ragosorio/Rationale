//! Consejo de ejecución: qué modelo y cuánto esfuerzo de razonamiento merece
//! un cambio, derivado solo de lo que el packet ya sabe.
//!
//! Es una heurística determinista y explicada, no una medición: no se validó
//! contra resultados de tareas reales. Por eso es opcional (`advise: true` en
//! `prepare_change`, o `advice.enabled` en `.rationale/config.yaml`), viene con
//! sus razones y la persona o el agente la puede ignorar.
//!
//! Separa dos ejes que no son lo mismo:
//! - **esfuerzo** (`effort`): cuánto razonar — sube con el riesgo y la
//!   ambigüedad (reglas fijadas o graves, conflictos con la intención,
//!   invariantes que cruzan módulos, dominios sensibles, índice incompleto);
//! - **contexto** (`context`): cuánto hay que leer — sube con el volumen de
//!   conocimiento que gobierna y el alcance (carpeta, muchas relaciones).
//!
//! El modelo se elige por el esfuerzo; un contexto grande sugiere delegar a
//! un subagente con contexto propio, no un modelo más caro.

use crate::retrieval::ContextPacket;
use serde::{Deserialize, Serialize};

/// Alias de Claude Code: se resuelven al modelo vigente de cada familia en el
/// cliente, así que el consejo no envejece con cada versión. Configurables en
/// `.rationale/config.yaml` (`advice.models`).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ModelMap {
    #[serde(default = "default_fast")]
    pub fast: String,
    #[serde(default = "default_balanced")]
    pub balanced: String,
    #[serde(default = "default_deep")]
    pub deep: String,
}

fn default_fast() -> String {
    "haiku".to_string()
}
fn default_balanced() -> String {
    "sonnet".to_string()
}
fn default_deep() -> String {
    "opus".to_string()
}

impl Default for ModelMap {
    fn default() -> Self {
        ModelMap {
            fast: default_fast(),
            balanced: default_balanced(),
            deep: default_deep(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AdviceConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub models: ModelMap,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExecutionAdvice {
    /// `fast | balanced | deep`.
    pub tier: &'static str,
    /// Alias del modelo para ese nivel (`advice.models`).
    pub model: String,
    /// `low | medium | high | xhigh`.
    pub effort: &'static str,
    /// `small | medium | large`: volumen de contexto, no dificultad.
    pub context: &'static str,
    pub reasons: Vec<String>,
    pub how: &'static str,
}

const HOW: &str = "Advice only, derived from this packet; the person's choice wins. In Claude \
     Code, delegate with the Agent tool passing model and effort, or ask the person before \
     switching the session with /model and /effort.";

/// Señales de riesgo del packet, cada una con su peso y su razón.
pub fn advise(
    packet: &ContextPacket,
    target_path: Option<&str>,
    models: &ModelMap,
) -> ExecutionAdvice {
    let mut score = 0u32;
    let mut reasons = Vec::new();
    let mut add = |weight: u32, reason: String| {
        score += weight;
        reasons.push(reason);
    };

    let pinned = packet
        .critical_constraints
        .iter()
        .filter(|c| c.governs_target && c.authority == "pinned")
        .count()
        + packet
            .decisions
            .iter()
            .filter(|d| d.authority == "pinned")
            .count();
    if pinned > 0 {
        add(2, format!("{pinned} pinned rule(s) govern the target"));
    }
    let grave = packet
        .critical_constraints
        .iter()
        .filter(|c| c.governs_target && matches!(c.severity.as_str(), "critical" | "high"))
        .count();
    if grave > 0 {
        add(
            1,
            format!("{grave} critical or high constraint(s) govern it"),
        );
    }
    let opposed = packet
        .intent_conflicts
        .iter()
        .filter(|c| c.governs_target && c.polarity == crate::retrieval::Polarity::Opposed)
        .count();
    if opposed > 0 {
        add(
            2,
            format!("the intent looks opposed to {opposed} governing Record(s)"),
        );
    }
    if !packet.inherited_constraints.is_empty() {
        add(
            1,
            format!(
                "{} invariant(s) cross into this code from modules it depends on",
                packet.inherited_constraints.len()
            ),
        );
    }
    let flagged = packet
        .decisions
        .iter()
        .filter(|d| !d.caveats.flags.is_empty())
        .count();
    if flagged > 0 {
        add(1, format!("{flagged} exception(s) lost their rule"));
    }
    if matches!(
        packet.retrieval.status.as_str(),
        "index_incomplete" | "target_missing"
    ) {
        add(1, format!("retrieval is {}", packet.retrieval.status));
    }
    if let Some(path) = target_path {
        let changed = [crate::capture::ChangedFile {
            path: path.to_string(),
            change_type: crate::capture::ChangeType::Modified,
            origin: crate::capture::ChangeOrigin::Unstaged,
        }];
        let mut domains: Vec<String> = crate::signals::signals_from_paths(&changed)
            .into_iter()
            .filter(|s| {
                matches!(
                    s,
                    crate::signals::Signal::Authorization
                        | crate::signals::Signal::Payments
                        | crate::signals::Signal::Security
                        | crate::signals::Signal::SchemaChange
                        | crate::signals::Signal::DestructiveMigration
                )
            })
            .map(|s| format!("{s:?}").to_lowercase())
            .collect();
        domains.sort();
        if !domains.is_empty() {
            add(1, format!("sensitive domain: {}", domains.join(", ")));
        }
    }

    let effort = match score {
        0 => "low",
        1..=2 => "medium",
        3..=5 => "high",
        _ => "xhigh",
    };
    let (tier, model) = match score {
        0 => ("fast", models.fast.clone()),
        1..=3 => ("balanced", models.balanced.clone()),
        _ => ("deep", models.deep.clone()),
    };
    let tokens = packet.budget.estimated_tokens;
    let breadth = packet.retrieval.scope == "folder"
        || packet
            .structure
            .as_ref()
            .is_some_and(|s| s.considered_nodes > 30);
    let context = if tokens > 2000 || (breadth && tokens > 1200) {
        "large"
    } else if tokens > 900 || breadth {
        "medium"
    } else {
        "small"
    };
    if context == "large" {
        reasons.push(
            "large context: consider a subagent with its own context rather than a larger model"
                .to_string(),
        );
    }
    if reasons.is_empty() {
        reasons.push("no governing rule, conflict, or sensitive domain".to_string());
    }
    ExecutionAdvice {
        tier,
        model,
        effort,
        context,
        reasons,
        how: HOW,
    }
}
