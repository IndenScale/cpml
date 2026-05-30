use std::collections::HashMap;

use chrono::NaiveDateTime;
use serde::Serialize;

use super::field::Field;
use super::geometry::Geometry;
use super::probe::Probe;
use super::projection::Projection;

/// A validated, resolved activity ready for compilation.
#[derive(Debug, Clone, Serialize)]
pub struct Activity {
    pub id: String,
    pub name: Option<String>,
    pub series: Option<String>,
    pub timespan: Timespan,
    pub geometry: Option<String>,
    pub probes: Vec<Probe>,
    pub projections: Vec<Projection>,
    pub depends_on: Vec<Dependency>,
}

/// Resolved timespan with parsed datetimes.
#[derive(Debug, Clone, Serialize)]
pub struct Timespan {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
}

impl Timespan {
    /// An activity is active at keyframe K if start <= K < end (half-open).
    pub fn contains(&self, dt: NaiveDateTime) -> bool {
        dt >= self.start && dt < self.end
    }
}

/// A resolved dependency between two activities.
#[derive(Debug, Clone, Serialize)]
pub struct Dependency {
    pub activity_id: String,
    pub kind: DependencyKind,
    pub lag_days: i64,
}

/// The four standard dependency types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DependencyKind {
    FS,
    SS,
    FF,
    SF,
}

/// The fully resolved CPML model, ready for the compiler pipeline.
#[derive(Debug, Clone, Serialize)]
pub struct CpmlModel {
    pub version: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub fields: Vec<Field>,
    pub activities: Vec<Activity>,
    /// Resolved barrier geometries for occlusion culling.
    pub barriers: Vec<Geometry>,
    /// Region hierarchy for capacity field evaluation.
    /// Maps child region key → parent region key. Built from the document's
    /// `regions` section during resolution.
    #[serde(skip)]
    pub region_hierarchy: HashMap<String, String>,
}
