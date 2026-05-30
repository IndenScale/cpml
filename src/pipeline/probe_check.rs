use std::collections::HashMap;

use chrono::NaiveDateTime;
use serde::Serialize;

use crate::model::activity::Activity;
use crate::model::field::Field;
use crate::model::geometry::{Aabb, Geometry};
use crate::model::probe::Assertion;
use crate::model::projection::Projection;
use crate::schema::DiagnosticLevel;

use super::field_eval::PersistentState;

/// A diagnostic produced when a probe assertion fails.
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub keyframe_index: usize,
    pub keyframe_date: NaiveDateTime,
    pub activity_id: String,
    pub probe_id: String,
    pub level: DiagnosticLevel,
    pub message: String,
    pub blame: Vec<BlameEntry>,
    /// Series ID of the activity that owns the failing probe.
    pub series_id: Option<String>,
}

/// An entry in the diagnostic blame list — which projection contributed to the failure.
#[derive(Debug, Clone, Serialize)]
pub struct BlameEntry {
    pub activity_id: String,
    pub projection_id: String,
    pub contribution_summary: String,
    pub confidence: Option<f64>,
}

/// Check all probes for active activities at a given keyframe.
pub fn check_probes(
    keyframe_index: usize,
    keyframe_date: NaiveDateTime,
    active_activities: &[&Activity],
    all_activities: &[Activity],
    field_map: &HashMap<String, Field>,
    persistent_state: &PersistentState,
    barriers: &[Geometry],
    region_hierarchy: &HashMap<String, String>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Build a flat list of all projections for blame tracing
    let all_projections: Vec<&Projection> = all_activities
        .iter()
        .flat_map(|a| a.projections.iter())
        .collect();

    // Collect active projection IDs
    let active_projection_ids: std::collections::HashSet<String> = active_activities
        .iter()
        .flat_map(|a| a.projections.iter().map(|p| p.id.clone()))
        .collect();

    // Build active projections as &[&Projection]
    let active_projections: Vec<&Projection> = all_projections
        .iter()
        .filter(|p| active_projection_ids.contains(&p.id))
        .copied()
        .collect();

    // Build series lookup: activity_id -> series_id
    let series_map: HashMap<String, Option<String>> = all_activities
        .iter()
        .map(|a| (a.id.clone(), a.series.clone()))
        .collect();

    for activity in active_activities {
        for probe in &activity.probes {
            let field = match field_map.get(&probe.field_name) {
                Some(f) => f,
                None => continue,
            };

            let failed = match &probe.assertion {
                Assertion::Empty => {
                    if !matches!(field, Field::Occupancy(_)) {
                        continue;
                    };
                    let worst = super::field_eval::eval_occupancy(
                        &active_projections,
                        &probe.geometry,
                        &activity.id,
                        activity.series.as_deref(),
                        &series_map,
                        barriers,
                    );
                    if let Some(kind) = worst {
                        // Auto-level by worst kind; probe level acts as floor (cannot demote)
                        let auto_level = match kind {
                            crate::schema::OccupancyKind::Hard => DiagnosticLevel::Error,
                            crate::schema::OccupancyKind::Soft => DiagnosticLevel::Warning,
                        };
                        let level = if auto_level > probe.diagnostic_level {
                            auto_level
                        } else {
                            probe.diagnostic_level
                        };
                        let blame = build_occupancy_blame(
                            &active_projections,
                            &probe.region_key(),
                            &activity.id,
                        );
                        let kind_label = match kind {
                            crate::schema::OccupancyKind::Hard => "Hard",
                            crate::schema::OccupancyKind::Soft => "Soft",
                        };
                        diagnostics.push(Diagnostic {
                            keyframe_index,
                            keyframe_date,
                            activity_id: activity.id.clone(),
                            probe_id: probe.id.clone(),
                            level,
                            message: format!(
                                "Occupancy collision: {} overlap detected",
                                kind_label
                            ),
                            blame,
                            series_id: activity.series.clone(),
                        });
                    }
                    worst.is_some()
                }
                Assertion::Gte(threshold) => {
                    let sampled = match sample_field_value(
                        field,
                        &active_projections,
                        persistent_state,
                        &probe.field_name,
                        &probe.region_key(),
                        keyframe_date,
                        region_hierarchy,
                    ) {
                        Some(v) => v,
                        None => continue,
                    };
                    if sampled < *threshold {
                        let blame = build_value_blame(
                            &active_projections,
                            &probe.field_name,
                            &probe.region_key(),
                        );
                        diagnostics.push(Diagnostic {
                            keyframe_index,
                            keyframe_date,
                            activity_id: activity.id.clone(),
                            probe_id: probe.id.clone(),
                            level: probe.diagnostic_level,
                            message: format!(
                                "Value below threshold: sampled {:.2} < required {:.2} on field '{}'",
                                sampled, threshold, probe.field_name
                            ),
                            blame,
                            series_id: activity.series.clone(),
                        });
                    }
                    sampled < *threshold
                }
                Assertion::Lte(ceiling) => {
                    let sampled = match sample_field_value(
                        field,
                        &active_projections,
                        persistent_state,
                        &probe.field_name,
                        &probe.region_key(),
                        keyframe_date,
                        region_hierarchy,
                    ) {
                        Some(v) => v,
                        None => continue,
                    };
                    if sampled > *ceiling {
                        let blame = build_value_blame(
                            &active_projections,
                            &probe.field_name,
                            &probe.region_key(),
                        );
                        diagnostics.push(Diagnostic {
                            keyframe_index,
                            keyframe_date,
                            activity_id: activity.id.clone(),
                            probe_id: probe.id.clone(),
                            level: probe.diagnostic_level,
                            message: format!(
                                "Value above ceiling: sampled {:.2} > max {:.2} on field '{}'",
                                sampled, ceiling, probe.field_name
                            ),
                            blame,
                            series_id: activity.series.clone(),
                        });
                    }
                    sampled > *ceiling
                }
                Assertion::Range { min, max } => {
                    let sampled = match sample_field_value(
                        field,
                        &active_projections,
                        persistent_state,
                        &probe.field_name,
                        &probe.region_key(),
                        keyframe_date,
                        region_hierarchy,
                    ) {
                        Some(v) => v,
                        None => continue,
                    };
                    if sampled < *min || sampled > *max {
                        let blame = build_value_blame(
                            &active_projections,
                            &probe.field_name,
                            &probe.region_key(),
                        );
                        diagnostics.push(Diagnostic {
                            keyframe_index,
                            keyframe_date,
                            activity_id: activity.id.clone(),
                            probe_id: probe.id.clone(),
                            level: probe.diagnostic_level,
                            message: format!(
                                "Value out of range: sampled {:.2} not in [{:.2}, {:.2}] on field '{}'",
                                sampled, min, max, probe.field_name
                            ),
                            blame,
                            series_id: activity.series.clone(),
                        });
                    }
                    sampled < *min || sampled > *max
                }
                Assertion::Present(criteria) => {
                    let prev = persistent_state
                        .presence
                        .get(&probe.field_name)
                        .cloned()
                        .unwrap_or_default();
                    let present = super::field_eval::eval_presence(
                        &active_projections,
                        &prev,
                        &probe.region_key(),
                        &criteria.key,
                        &criteria.record_type,
                        &criteria.attributes,
                        keyframe_date,
                    );
                    if !present {
                        let blame = build_presence_blame(
                            &active_projections,
                            &probe.field_name,
                            &probe.region_key(),
                        );
                        diagnostics.push(Diagnostic {
                            keyframe_index,
                            keyframe_date,
                            activity_id: activity.id.clone(),
                            probe_id: probe.id.clone(),
                            level: probe.diagnostic_level,
                            message: format!(
                                "Required presence record '{}' not found or invalid on field '{}'",
                                criteria.key, probe.field_name
                            ),
                            blame,
                            series_id: activity.series.clone(),
                        });
                    }
                    !present
                }
            };
            let _ = failed;
        }
    }

    diagnostics.sort_by(|a, b| b.level.cmp(&a.level));
    diagnostics
}

/// Sample a numeric field value for Capacity, Scalar, or Rate fields.
/// Returns None if the field type is not numeric (Occupancy/Presence).
fn sample_field_value(
    field: &Field,
    active_projections: &[&Projection],
    persistent_state: &PersistentState,
    field_name: &str,
    region_key: &str,
    keyframe_date: NaiveDateTime,
    region_hierarchy: &HashMap<String, String>,
) -> Option<f64> {
    match field {
        Field::Capacity(_) => Some(super::field_eval::eval_capacity(
            active_projections,
            region_key,
            field_name,
            region_hierarchy,
        )),
        Field::Scalar(sf) => {
            let prev = persistent_state
                .scalar
                .get(field_name)
                .cloned()
                .unwrap_or_default();
            Some(super::field_eval::eval_scalar(
                active_projections,
                &prev,
                region_key,
                sf.operator,
            ))
        }
        Field::Rate(rf) => {
            let prev = persistent_state
                .rate
                .get(field_name)
                .and_then(|regions| regions.get(region_key).cloned())
                .unwrap_or_default();
            Some(super::field_eval::eval_rate(
                active_projections,
                &prev,
                region_key,
                rf.window_size,
                keyframe_date,
            ))
        }
        _ => None,
    }
}

fn build_occupancy_blame(
    all_projections: &[&Projection],
    region_key: &str,
    self_activity_id: &str,
) -> Vec<BlameEntry> {
    all_projections
        .iter()
        .filter(|p| p.parent_activity_id != self_activity_id)
        .filter(|p| {
            p.region_key() == region_key
                || Aabb::from_region_key(region_key)
                    .map(|r| p.aabb().overlaps(&r))
                    .unwrap_or(false)
        })
        .filter_map(|p| {
            if let crate::model::projection::Contribution::Occupancy { kind } = &p.contribution {
                Some(BlameEntry {
                    activity_id: p.parent_activity_id.clone(),
                    projection_id: p.id.clone(),
                    contribution_summary: format!("occupancy({:?})", kind),
                    confidence: p.confidence,
                })
            } else {
                None
            }
        })
        .collect()
}

fn build_value_blame(
    all_projections: &[&Projection],
    field_name: &str,
    region_key: &str,
) -> Vec<BlameEntry> {
    all_projections
        .iter()
        .filter(|p| p.field_name == field_name)
        .filter(|p| {
            p.region_key() == region_key
                || Aabb::from_region_key(region_key)
                    .map(|r| p.aabb().overlaps(&r))
                    .unwrap_or(false)
        })
        .map(|p| {
            let summary = match &p.contribution {
                crate::model::projection::Contribution::Capacity(v) => format!("capacity({})", v),
                crate::model::projection::Contribution::Scalar { value, operator } => {
                    format!("scalar({}, {:?})", value, operator)
                }
                crate::model::projection::Contribution::Rate(v) => format!("rate({})", v),
                other => format!("{:?}", other),
            };
            BlameEntry {
                activity_id: p.parent_activity_id.clone(),
                projection_id: p.id.clone(),
                contribution_summary: summary,
                confidence: p.confidence,
            }
        })
        .collect()
}

fn build_presence_blame(
    all_projections: &[&Projection],
    field_name: &str,
    region_key: &str,
) -> Vec<BlameEntry> {
    all_projections
        .iter()
        .filter(|p| p.field_name == field_name)
        .filter(|p| {
            p.region_key() == region_key
                || Aabb::from_region_key(region_key)
                    .map(|r| p.aabb().overlaps(&r))
                    .unwrap_or(false)
        })
        .map(|p| {
            let summary = match &p.contribution {
                crate::model::projection::Contribution::Presence(rec) => {
                    format!("presence(key={})", rec.key)
                }
                other => format!("{:?}", other),
            };
            BlameEntry {
                activity_id: p.parent_activity_id.clone(),
                projection_id: p.id.clone(),
                contribution_summary: summary,
                confidence: p.confidence,
            }
        })
        .collect()
}
