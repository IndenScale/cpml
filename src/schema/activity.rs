use serde::Deserialize;

use super::{DiagnosticLevel, ProbeDef, ProjectionDef, StructureDef};

/// An activity definition in the CPML document.
#[derive(Debug, Deserialize, Clone)]
pub struct ActivityDef {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    /// Activity series group identifier
    #[serde(default)]
    pub series: Option<String>,
    pub timespan: TimespanDef,
    /// Reference to a named geometry (for display/bounding)
    #[serde(default)]
    pub geometry: Option<String>,
    /// Collision sugar: auto-generates hard/soft probe+projection pairs
    #[serde(default)]
    pub collision: Option<CollisionDef>,
    #[serde(default)]
    pub probes: Vec<ProbeDef>,
    #[serde(default)]
    pub projections: Vec<ProjectionDef>,
    #[serde(default)]
    pub structures: Vec<StructureDef>,
    /// Explicit predecessor/successor dependencies with optional lag.
    #[serde(default)]
    pub depends_on: Vec<DependencyDef>,
    /// Resource demand sugar: auto-generates Gte probe + negative Capacity projection.
    #[serde(default)]
    pub demands: Vec<DemandDef>,
}

/// Resource demand syntax sugar — declares a resource requirement
/// and auto-generates a probe (gte: amount) + projection (value: -amount).
#[derive(Debug, Deserialize, Clone)]
pub struct DemandDef {
    /// Capacity field name to draw from.
    pub field: String,
    /// Geometry reference for the demand's spatial scope.
    pub geometry: String,
    /// Amount required (positive number; projection auto-negates).
    pub amount: f64,
    /// Optional diagnostic level override for the generated probe.
    #[serde(default)]
    pub diagnostic_level: Option<DiagnosticLevel>,
}

/// Timespan of an activity (ISO 8601 dates).
#[derive(Debug, Deserialize, Clone)]
pub struct TimespanDef {
    pub start: String,
    pub end: String,
}

/// Collision syntax sugar definition.
#[derive(Debug, Deserialize, Clone)]
pub struct CollisionDef {
    pub hard: Option<CollisionEntry>,
    pub soft: Option<CollisionEntry>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CollisionEntry {
    pub geometry: String,
}

/// An explicit dependency between two activities.
#[derive(Debug, Deserialize, Clone)]
pub struct DependencyDef {
    pub activity_id: String,
    /// Dependency type: FS (finish-to-start), SS, FF, or SF.
    #[serde(default)]
    pub kind: DependencyKind,
    /// Optional lag in days (positive = gap, negative = overlap allowed).
    #[serde(default)]
    pub lag_days: Option<i64>,
}

/// The four standard dependency types.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    /// Finish-to-Start: predecessor must finish before successor starts.
    #[serde(alias = "FS")]
    #[default]
    Fs,
    /// Start-to-Start: predecessor must start before successor starts.
    #[serde(alias = "SS")]
    Ss,
    /// Finish-to-Finish: predecessor must finish before successor finishes.
    #[serde(alias = "FF")]
    Ff,
    /// Start-to-Finish: predecessor must start before successor finishes.
    #[serde(alias = "SF")]
    Sf,
}
