use serde::Deserialize;

use super::{AssertionDef, DiagnosticLevel, OccupancyKind, ScalarOperator};

/// A structure definition — syntax sugar that auto-generates a probe+projection pair.
#[derive(Debug, Deserialize, Clone)]
pub struct StructureDef {
    pub name: Option<String>,
    pub field: String,
    pub geometry: String,
    #[serde(flatten, default)]
    pub assert: Option<AssertionDef>,
    pub diagnostic_level: Option<DiagnosticLevel>,
    pub kind: Option<OccupancyKind>,
    pub value: Option<f64>,
    pub operator: Option<ScalarOperator>,
    /// Confidence score (0.0–1.0) for this structure's projection.
    /// Lower confidence means the contribution is weighted down.
    #[serde(default)]
    pub confidence: Option<f64>,
}
