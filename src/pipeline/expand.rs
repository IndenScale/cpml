use crate::error::CpmlError;
use crate::model::activity::Activity;
use crate::model::geometry::Geometry;
use crate::model::probe::{Assertion, Probe};
use crate::model::projection::{Contribution, Projection};
use crate::schema::{DiagnosticLevel, OccupancyKind};

use super::resolve::resolve_assertion;

/// Expand collision, structure, and demand sugar from schema defs into probes + projections.
/// Modifies the activity's probe and projection vectors in place.
pub fn expand_activity(
    activity: &mut Activity,
    ad: &crate::schema::ActivityDef,
    field_map: &std::collections::HashMap<String, crate::model::field::Field>,
    geometry_map: &std::collections::HashMap<String, Geometry>,
) -> Result<(), CpmlError> {
    // Expand collision sugar
    if let Some(ref collision) = ad.collision {
        let occupancy_field = find_any_occupancy_field(field_map)?;

        // Hard collision
        if let Some(ref hard) = collision.hard {
            let geo = geometry_map.get(&hard.geometry).ok_or_else(|| {
                CpmlError::ReferenceError(format!(
                    "Collision hard geometry '{}' not found for activity '{}'",
                    hard.geometry, ad.id
                ))
            })?;

            let probe_id = format!("{}/collision_hard_probe", ad.id);
            activity.probes.push(Probe {
                id: probe_id,
                name: Some("collision_hard_probe".into()),
                field_name: occupancy_field.clone(),
                geometry: geo.clone(),
                assertion: Assertion::Empty,
                diagnostic_level: DiagnosticLevel::Error,
                parent_activity_id: ad.id.clone(),
            });

            let proj_id = format!("{}/collision_hard_projection", ad.id);
            activity.projections.push(Projection {
                id: proj_id,
                name: Some("collision_hard_projection".into()),
                field_name: occupancy_field.clone(),
                geometry: geo.clone(),
                contribution: Contribution::Occupancy {
                    kind: OccupancyKind::Hard,
                },
                parent_activity_id: ad.id.clone(),
                confidence: None,
            });
        }

        // Soft collision
        if let Some(ref soft) = collision.soft {
            let geo = geometry_map.get(&soft.geometry).ok_or_else(|| {
                CpmlError::ReferenceError(format!(
                    "Collision soft geometry '{}' not found for activity '{}'",
                    soft.geometry, ad.id
                ))
            })?;

            let probe_id = format!("{}/collision_soft_probe", ad.id);
            activity.probes.push(Probe {
                id: probe_id,
                name: Some("collision_soft_probe".into()),
                field_name: occupancy_field.clone(),
                geometry: geo.clone(),
                assertion: Assertion::Empty,
                diagnostic_level: DiagnosticLevel::Warning,
                parent_activity_id: ad.id.clone(),
            });

            let proj_id = format!("{}/collision_soft_projection", ad.id);
            activity.projections.push(Projection {
                id: proj_id,
                name: Some("collision_soft_projection".into()),
                field_name: occupancy_field.clone(),
                geometry: geo.clone(),
                contribution: Contribution::Occupancy {
                    kind: OccupancyKind::Soft,
                },
                parent_activity_id: ad.id.clone(),
                confidence: None,
            });
        }
    }

    // Expand structure sugar
    for (i, sd) in ad.structures.iter().enumerate() {
        let geo = geometry_map.get(&sd.geometry).ok_or_else(|| {
            CpmlError::ReferenceError(format!(
                "Structure '{}' geometry '{}' not found in activity '{}'",
                sd.name.as_deref().unwrap_or("unnamed"),
                sd.geometry,
                ad.id
            ))
        })?;

        let field = field_map.get(&sd.field).ok_or_else(|| {
            CpmlError::ReferenceError(format!(
                "Structure '{}' field '{}' not found in activity '{}'",
                sd.name.as_deref().unwrap_or("unnamed"),
                sd.field,
                ad.id
            ))
        })?;

        let struct_name = sd
            .name
            .clone()
            .unwrap_or_else(|| format!("structure_{}", i));

        // Auto-generate probe
        let assertion = if let Some(ref a) = sd.assert {
            resolve_assertion(a, field)?
        } else if matches!(field, crate::model::field::Field::Presence(_)) {
            return Err(CpmlError::ValidationError(format!(
                "Structure '{}' on presence field '{}' in activity '{}' must specify an explicit 'assert' (e.g. 'present: {{key: \"...\"}}'). Cannot infer a default assertion for presence fields.",
                struct_name, sd.field, ad.id
            )));
        } else {
            default_assertion_for_field(field)
        };

        let diag_level = sd
            .diagnostic_level
            .unwrap_or_else(|| default_diagnostic_level_for_field(field));

        let probe_id = format!("{}/struct_{}_probe", ad.id, struct_name);
        activity.probes.push(Probe {
            id: probe_id,
            name: Some(format!("struct_{}_probe", struct_name)),
            field_name: sd.field.clone(),
            geometry: geo.clone(),
            assertion,
            diagnostic_level: diag_level,
            parent_activity_id: ad.id.clone(),
        });

        // Auto-generate projection
        let contribution = build_structure_contribution(sd, field, Some(&ad.timespan.start))?;

        let proj_id = format!("{}/struct_{}_projection", ad.id, struct_name);
        activity.projections.push(Projection {
            id: proj_id,
            name: Some(format!("struct_{}_projection", struct_name)),
            field_name: sd.field.clone(),
            geometry: geo.clone(),
            contribution,
            parent_activity_id: ad.id.clone(),
            confidence: sd.confidence,
        });
    }

    // Expand demand sugar
    for (i, dd) in ad.demands.iter().enumerate() {
        let field = field_map.get(&dd.field).ok_or_else(|| {
            CpmlError::ReferenceError(format!(
                "Demand '{}' in activity '{}' references unknown field '{}'",
                dd.field, ad.id, dd.field
            ))
        })?;

        if !matches!(field, crate::model::field::Field::Capacity(_)) {
            return Err(CpmlError::TypeMismatchError(format!(
                "Demand '{}' in activity '{}' must reference a capacity field, got {:?}",
                dd.field,
                ad.id,
                field.field_type()
            )));
        }

        let geo = geometry_map.get(&dd.geometry).ok_or_else(|| {
            CpmlError::ReferenceError(format!(
                "Demand '{}' geometry '{}' not found in activity '{}'",
                dd.field, dd.geometry, ad.id
            ))
        })?;

        let demand_name = format!("demand_{}", i);
        let diag_level = dd.diagnostic_level.unwrap_or(DiagnosticLevel::Error);

        // Auto-generate probe: check that capacity >= required amount
        let probe_id = format!("{}/{}_probe", ad.id, demand_name);
        activity.probes.push(Probe {
            id: probe_id,
            name: Some(format!("{}_probe", demand_name)),
            field_name: dd.field.clone(),
            geometry: geo.clone(),
            assertion: Assertion::Gte(dd.amount),
            diagnostic_level: diag_level,
            parent_activity_id: ad.id.clone(),
        });

        // Auto-generate projection: consume the capacity
        let proj_id = format!("{}/{}_projection", ad.id, demand_name);
        activity.projections.push(Projection {
            id: proj_id,
            name: Some(format!("{}_projection", demand_name)),
            field_name: dd.field.clone(),
            geometry: geo.clone(),
            contribution: Contribution::Capacity(-dd.amount),
            parent_activity_id: ad.id.clone(),
            confidence: None,
        });
    }

    Ok(())
}

fn find_any_occupancy_field(
    field_map: &std::collections::HashMap<String, crate::model::field::Field>,
) -> Result<String, CpmlError> {
    for (name, field) in field_map {
        if matches!(field, crate::model::field::Field::Occupancy(_)) {
            return Ok(name.clone());
        }
    }
    Err(CpmlError::ReferenceError(
        "No occupancy field found. Declare one in the fields section.".into(),
    ))
}

fn default_assertion_for_field(field: &crate::model::field::Field) -> Assertion {
    match field {
        crate::model::field::Field::Capacity(_) => Assertion::Gte(0.0),
        crate::model::field::Field::Occupancy(_) => Assertion::Empty,
        crate::model::field::Field::Scalar(_) => Assertion::Gte(0.0),
        crate::model::field::Field::Presence(_) => {
            unreachable!("PresenceField must have explicit assert; caller should validate first")
        }
        crate::model::field::Field::Rate(_) => Assertion::Gte(0.0),
    }
}

fn default_diagnostic_level_for_field(field: &crate::model::field::Field) -> DiagnosticLevel {
    match field {
        crate::model::field::Field::Occupancy(_) => DiagnosticLevel::Error,
        _ => DiagnosticLevel::Error,
    }
}

fn build_structure_contribution(
    sd: &crate::schema::StructureDef,
    field: &crate::model::field::Field,
    valid_from: Option<&str>,
) -> Result<Contribution, CpmlError> {
    match field {
        crate::model::field::Field::Capacity(_) => {
            let value = sd.value.unwrap_or(0.0);
            Ok(Contribution::Capacity(value))
        }
        crate::model::field::Field::Occupancy(_) => {
            let kind = sd.kind.unwrap_or(OccupancyKind::Hard);
            Ok(Contribution::Occupancy { kind })
        }
        crate::model::field::Field::Scalar(sf) => {
            let value = sd.value.unwrap_or(0.0);
            let operator = sd.operator.unwrap_or(sf.operator);
            Ok(Contribution::Scalar { value, operator })
        }
        crate::model::field::Field::Presence(_) => {
            // Structures on presence fields need a key — use the structure name
            let key = sd.name.clone().unwrap_or_else(|| "unnamed".into());
            let valid_from_date = valid_from
                .map(|s| {
                    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| {
                        CpmlError::DateParseError(format!("Invalid date '{}': {}", s, e))
                    })
                })
                .transpose()?
                .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
            Ok(Contribution::Presence(
                crate::model::projection::PresenceRecord {
                    key,
                    record_type: None,
                    attributes: std::collections::HashMap::new(),
                    valid_from: valid_from_date,
                    valid_until: None,
                },
            ))
        }
        crate::model::field::Field::Rate(_) => {
            let value = sd.value.unwrap_or(0.0);
            Ok(Contribution::Rate(value))
        }
    }
}
