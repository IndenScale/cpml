use serde::Deserialize;
use std::collections::HashMap;

use super::DiagnosticLevel;

/// A probe definition — samples a field and asserts a condition.
#[derive(Debug, Deserialize, Clone)]
pub struct ProbeDef {
    pub name: Option<String>,
    pub field: String,
    pub geometry: String,
    #[serde(flatten)]
    pub assert: AssertionDef,
    pub diagnostic_level: Option<DiagnosticLevel>,
}

/// Type-discriminated assertion. Serde tries each variant via `#[serde(untagged)]`.
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum AssertionDef {
    Empty { empty: bool },
    Gte { gte: f64 },
    Lte { lte: f64 },
    Range { min: f64, max: f64 },
    Present { present: PresentCriteriaDef },
}

/// Criteria for checking a PresenceField record.
#[derive(Debug, Deserialize, Clone)]
pub struct PresentCriteriaDef {
    pub key: String,
    #[serde(rename = "type")]
    pub record_type: Option<String>,
    #[serde(default)]
    pub attributes: HashMap<String, String>,
}
