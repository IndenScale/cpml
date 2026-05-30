use serde::{Deserialize, Serialize};

use super::{ActivityDef, GeometryDef};

/// Top-level CPML document, deserialized from YAML.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct CpmlDocument {
    pub version: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub fields: Vec<FieldDef>,
    #[serde(default)]
    pub geometries: Vec<GeometryDef>,
    #[serde(default)]
    pub activities: Vec<ActivityDef>,
    /// Optional barrier geometries for occlusion culling.
    /// Barriers block projections from reaching probes behind them.
    #[serde(default)]
    pub barriers: Vec<BarrierDef>,
    /// Optional region hierarchy for capacity field evaluation.
    /// Child regions inherit capacity from parent regions.
    #[serde(default)]
    pub regions: Vec<RegionDef>,
}

/// A barrier definition for occlusion culling.
/// References a geometry that can block field projections.
#[derive(Debug, Deserialize, Clone)]
pub struct BarrierDef {
    /// References a geometry by ID.
    pub geometry: String,
}

/// A named spatial region for capacity hierarchy.
/// Regions form a tree: a child region's capacity probes see contributions
/// from all ancestor regions (walking up through `parent`).
#[derive(Debug, Deserialize, Clone)]
pub struct RegionDef {
    pub id: String,
    /// Optional parent region ID — this region inherits capacity visibility
    /// from its parent and all ancestors.
    #[serde(default)]
    pub parent: Option<String>,
}

/// Supported field types (the `type` discriminator in YAML).
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Capacity,
    Occupancy,
    Scalar,
    Presence,
    Rate,
}

/// A field declaration.
#[derive(Debug, Deserialize, Clone)]
pub struct FieldDef {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    /// ScalarField only
    #[serde(default)]
    pub operator: Option<ScalarOperator>,
    /// RateField only: number of keyframes for rate calculation window
    #[serde(default)]
    pub window_size: Option<usize>,
}

/// Occupancy field kind: hard (physical collision) or soft (risk/safety zone).
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OccupancyKind {
    Hard,
    Soft,
}

/// ScalarField accumulation operator.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScalarOperator {
    Max,
    Min,
    Sum,
    Replace,
}

/// Diagnostic severity level.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticLevel {
    Debug,
    Info,
    Warning,
    Error,
    Fatal,
}
