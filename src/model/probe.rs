use std::collections::HashMap;

use serde::Serialize;

use crate::schema::DiagnosticLevel;

use super::geometry::{Aabb, Geometry};

/// A resolved probe — samples a field region and asserts a condition.
#[derive(Debug, Clone, Serialize)]
pub struct Probe {
    pub id: String,
    pub name: Option<String>,
    pub field_name: String,
    pub geometry: Geometry,
    pub assertion: Assertion,
    pub diagnostic_level: DiagnosticLevel,
    pub parent_activity_id: String,
}

impl Probe {
    /// Convenience: world-space AABB of this probe's geometry.
    pub fn aabb(&self) -> Aabb {
        self.geometry.world_aabb()
    }

    /// Convenience: region key for spatial hashing.
    pub fn region_key(&self) -> String {
        self.geometry.region_key()
    }
}

/// Type-discriminated assertion evaluated against field state.
#[derive(Debug, Clone, Serialize)]
pub enum Assertion {
    /// OccupancyField: the sampled region must be empty
    Empty,
    /// CapacityField / ScalarField / RateField: value must be >= threshold
    Gte(f64),
    /// CapacityField / ScalarField / RateField: value must be <= ceiling
    Lte(f64),
    /// CapacityField / ScalarField / RateField: value must be within [min, max]
    Range { min: f64, max: f64 },
    /// PresenceField: record matching criteria must exist and be valid
    Present(PresenceCriteria),
}

/// Criteria for matching a PresenceField record.
#[derive(Debug, Clone, Serialize)]
pub struct PresenceCriteria {
    pub key: String,
    pub record_type: Option<String>,
    pub attributes: HashMap<String, String>,
}
