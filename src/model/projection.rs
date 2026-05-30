use std::collections::HashMap;

use chrono::NaiveDate;
use serde::Serialize;

use crate::schema::{OccupancyKind, ScalarOperator};

use super::geometry::{Aabb, Geometry};

/// A resolved projection — injects a contribution into a field.
#[derive(Debug, Clone, Serialize)]
pub struct Projection {
    pub id: String,
    pub name: Option<String>,
    pub field_name: String,
    pub geometry: Geometry,
    pub contribution: Contribution,
    pub parent_activity_id: String,
    /// Confidence weight (0.0–1.0) for this projection. None = full confidence (1.0).
    pub confidence: Option<f64>,
}

impl Projection {
    /// Convenience: world-space AABB of this projection's geometry.
    pub fn aabb(&self) -> Aabb {
        self.geometry.world_aabb()
    }

    /// Convenience: region key for spatial hashing.
    pub fn region_key(&self) -> String {
        self.geometry.region_key()
    }
}

/// The type of contribution a projection makes to its field.
#[derive(Debug, Clone, Serialize)]
pub enum Contribution {
    /// OccupancyField: declares a region as occupied (hard or soft)
    Occupancy { kind: OccupancyKind },
    /// CapacityField: real-valued contribution (positive = supply, negative = consumption)
    Capacity(f64),
    /// ScalarField: persistent scalar value with an accumulation operator
    Scalar {
        value: f64,
        operator: ScalarOperator,
    },
    /// PresenceField: a record with key, type, attributes, and validity range
    Presence(PresenceRecord),
    /// RateField: a flow-rate contribution (units per keyframe interval)
    Rate(f64),
}

/// A record stored in a PresenceField.
#[derive(Debug, Clone, Serialize)]
pub struct PresenceRecord {
    pub key: String,
    pub record_type: Option<String>,
    pub attributes: HashMap<String, String>,
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
}
