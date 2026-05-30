use crate::schema::{FieldType, ScalarOperator};
use serde::Serialize;

/// A resolved field instance.
#[derive(Debug, Clone, Serialize)]
pub enum Field {
    Capacity(CapacityField),
    Occupancy(OccupancyField),
    Scalar(ScalarField),
    Presence(PresenceField),
    Rate(RateField),
}

impl Field {
    pub fn name(&self) -> &str {
        match self {
            Field::Capacity(f) => &f.name,
            Field::Occupancy(f) => &f.name,
            Field::Scalar(f) => &f.name,
            Field::Presence(f) => &f.name,
            Field::Rate(f) => &f.name,
        }
    }

    pub fn field_type(&self) -> FieldType {
        match self {
            Field::Capacity(_) => FieldType::Capacity,
            Field::Occupancy(_) => FieldType::Occupancy,
            Field::Scalar(_) => FieldType::Scalar,
            Field::Presence(_) => FieldType::Presence,
            Field::Rate(_) => FieldType::Rate,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CapacityField {
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OccupancyField {
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScalarField {
    pub name: String,
    pub operator: ScalarOperator,
}

#[derive(Debug, Clone, Serialize)]
pub struct PresenceField {
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RateField {
    pub name: String,
    /// Number of keyframes over which to compute the rate of change.
    pub window_size: usize,
}
