use serde::Deserialize;
use std::collections::HashMap;

use super::{OccupancyKind, ScalarOperator};

/// A projection definition — injects a contribution into a field.
#[derive(Debug, Deserialize, Clone)]
pub struct ProjectionDef {
    pub name: Option<String>,
    pub field: String,
    pub geometry: String,
    /// For OccupancyField: hard or soft
    pub kind: Option<OccupancyKind>,
    /// For CapacityField / ScalarField
    pub value: Option<f64>,
    /// For ScalarField: override field default operator
    pub operator: Option<ScalarOperator>,
    /// For PresenceField
    pub record: Option<PresenceRecordDef>,
    /// Confidence weight (0.0 to 1.0) for this projection's contribution.
    /// Affects value-weighted fields (capacity, scalar, rate).
    #[serde(default)]
    pub confidence: Option<f64>,
}

/// A record injected into a PresenceField.
#[derive(Debug, Deserialize, Clone)]
pub struct PresenceRecordDef {
    pub key: String,
    #[serde(rename = "type")]
    pub record_type: Option<String>,
    #[serde(default)]
    pub attributes: HashMap<String, String>,
    /// ISO 8601 date string; None = never expires
    pub valid_until: Option<String>,
}
