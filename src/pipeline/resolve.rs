use std::collections::HashMap;

use chrono::{NaiveDate, NaiveDateTime};

use crate::error::CpmlError;
use crate::model::activity::{Activity, CpmlModel, Dependency, DependencyKind, Timespan};
use crate::model::field::{
    CapacityField, Field, OccupancyField, PresenceField, RateField, ScalarField,
};
use crate::model::geometry::{Geometry, Pose, Shape};
use crate::model::probe::{Assertion, PresenceCriteria, Probe};
use crate::model::projection::{Contribution, PresenceRecord, Projection};
use crate::schema::{self, AssertionDef, DiagnosticLevel, FieldType, OccupancyKind};

/// Resolve a schema-level CpmlDocument into a validated CpmlModel.
pub fn resolve(doc: schema::CpmlDocument) -> Result<CpmlModel, CpmlError> {
    // Build geometry lookup (with region mapping for capacity hierarchy)
    let mut geometry_map: HashMap<String, Geometry> = HashMap::new();
    let mut region_to_geom: HashMap<String, String> = HashMap::new(); // region_id → geometry_id
    for geo in &doc.geometries {
        if geometry_map.contains_key(&geo.id) {
            return Err(CpmlError::DuplicateIdError(format!(
                "Duplicate geometry ID: {}",
                geo.id
            )));
        }
        let geometry = resolve_geometry_with_region(&geo.shape, geo.pose.as_ref(), geo.region.clone());
        if let Some(ref region_id) = geo.region {
            region_to_geom.insert(region_id.clone(), geo.id.clone());
        }
        geometry_map.insert(geo.id.clone(), geometry);
    }

    // Build region hierarchy: child region_key → parent region_key.
    // RegionDefs reference geometry IDs, which resolve to AABB region keys.
    let mut region_hierarchy: HashMap<String, String> = HashMap::new();
    for rd in &doc.regions {
        let child_geom_id = region_to_geom
            .get(&rd.id)
            .or_else(|| {
                // Allow region ID to directly name a geometry
                if geometry_map.contains_key(&rd.id) {
                    Some(&rd.id)
                } else {
                    None
                }
            });
        if let Some(parent_region_id) = &rd.parent {
            let parent_geom_id = region_to_geom
                .get(parent_region_id)
                .or_else(|| {
                    if geometry_map.contains_key(parent_region_id) {
                        Some(parent_region_id)
                    } else {
                        None
                    }
                });
            if let (Some(child_gid), Some(parent_gid)) = (child_geom_id, parent_geom_id) {
                if let (Some(child_geom), Some(parent_geom)) = (
                    geometry_map.get(child_gid),
                    geometry_map.get(parent_gid),
                ) {
                    let child_key = child_geom.region_key();
                    let parent_key = parent_geom.region_key();
                    region_hierarchy.insert(child_key, parent_key);
                }
            }
        }
    }

    // Build field lookup and validated fields
    let mut field_map: HashMap<String, Field> = HashMap::new();
    for f in &doc.fields {
        if field_map.contains_key(&f.name) {
            return Err(CpmlError::DuplicateIdError(format!(
                "Duplicate field name: {}",
                f.name
            )));
        }
        let field = resolve_field(f)?;
        field_map.insert(f.name.clone(), field);
    }

    // Validate unique activity IDs
    let mut activity_ids: HashMap<String, bool> = HashMap::new();
    for a in &doc.activities {
        if activity_ids.contains_key(&a.id) {
            return Err(CpmlError::DuplicateIdError(format!(
                "Duplicate activity ID: {}",
                a.id
            )));
        }
        activity_ids.insert(a.id.clone(), true);
    }

    // Resolve activities
    let mut activities: Vec<Activity> = Vec::new();
    for ad in &doc.activities {
        let activity = resolve_activity(ad, &field_map, &geometry_map, &activity_ids)?;
        activities.push(activity);
    }

    // Validate activity series: members must have non-overlapping timespans
    validate_series(&activities)?;

    // Resolve barriers for occlusion culling
    let mut barriers: Vec<Geometry> = Vec::new();
    for bd in &doc.barriers {
        let barrier_geom = geometry_map.get(&bd.geometry).ok_or_else(|| {
            CpmlError::ReferenceError(format!(
                "Barrier references unknown geometry '{}'",
                bd.geometry
            ))
        })?;
        barriers.push(barrier_geom.clone());
    }

    Ok(CpmlModel {
        version: doc.version,
        name: doc.name,
        description: doc.description,
        fields: field_map.into_values().collect(),
        activities,
        barriers,
        region_hierarchy,
    })
}

pub(crate) fn resolve_geometry_with_region(
    shape_def: &schema::ShapeDef,
    pose_def: Option<&schema::PoseDef>,
    region: Option<String>,
) -> Geometry {
    let pose = pose_def.map(|p| Pose {
        position: p.position,
        rotation: p.rotation,
    });

    let mut geom = match shape_def {
        schema::ShapeDef::Aabb { min, max } => {
            let half_extents = [
                (max[0] - min[0]) / 2.0,
                (max[1] - min[1]) / 2.0,
                (max[2] - min[2]) / 2.0,
            ];
            let center = [
                (min[0] + max[0]) / 2.0,
                (min[1] + max[1]) / 2.0,
                (min[2] + max[2]) / 2.0,
            ];
            let final_pose = pose.unwrap_or_else(|| Pose::from_position(center));
            Geometry {
                shape: Shape::Cuboid { half_extents },
                pose: final_pose,
                region: None,
            }
        }
        schema::ShapeDef::Cuboid { half_extents } => Geometry {
            shape: Shape::Cuboid {
                half_extents: *half_extents,
            },
            pose: pose.unwrap_or_default(),
            region: None,
        },
        schema::ShapeDef::Cylinder {
            radius,
            half_height,
        } => Geometry {
            shape: Shape::Cylinder {
                radius: *radius,
                half_height: *half_height,
            },
            pose: pose.unwrap_or_default(),
            region: None,
        },
        schema::ShapeDef::Sphere { radius } => Geometry {
            shape: Shape::Sphere { radius: *radius },
            pose: pose.unwrap_or_default(),
            region: None,
        },
        schema::ShapeDef::Hemisphere { radius } => Geometry {
            shape: Shape::Hemisphere { radius: *radius },
            pose: pose.unwrap_or_default(),
            region: None,
        },
        schema::ShapeDef::Cone {
            radius,
            half_height,
        } => Geometry {
            shape: Shape::Cone {
                radius: *radius,
                half_height: *half_height,
            },
            pose: pose.unwrap_or_default(),
            region: None,
        },
        schema::ShapeDef::Union(children) => {
            let shapes: Vec<Shape> = children
                .iter()
                .map(|c| resolve_geometry_with_region(c, None, None).shape)
                .collect();
            Geometry {
                shape: Shape::Union(shapes),
                pose: pose.unwrap_or_default(),
                region: None,
            }
        }
        schema::ShapeDef::Intersection(children) => {
            let shapes: Vec<Shape> = children
                .iter()
                .map(|c| resolve_geometry_with_region(c, None, None).shape)
                .collect();
            Geometry {
                shape: Shape::Intersection(shapes),
                pose: pose.unwrap_or_default(),
                region: None,
            }
        }
        schema::ShapeDef::Subtract { a, b } => {
            let shape_a = resolve_geometry_with_region(a, None, None).shape;
            let shape_b = resolve_geometry_with_region(b, None, None).shape;
            Geometry {
                shape: Shape::Subtract {
                    a: Box::new(shape_a),
                    b: Box::new(shape_b),
                },
                pose: pose.unwrap_or_default(),
                region: None,
            }
        }
    };
    geom.region = region;
    geom
}

fn resolve_field(f: &schema::FieldDef) -> Result<Field, CpmlError> {
    match f.field_type {
        FieldType::Capacity => Ok(Field::Capacity(CapacityField {
            name: f.name.clone(),
        })),
        FieldType::Occupancy => Ok(Field::Occupancy(OccupancyField {
            name: f.name.clone(),
        })),
        FieldType::Scalar => {
            let operator = f.operator.unwrap_or(schema::ScalarOperator::Max);
            Ok(Field::Scalar(ScalarField {
                name: f.name.clone(),
                operator,
            }))
        }
        FieldType::Presence => Ok(Field::Presence(PresenceField {
            name: f.name.clone(),
        })),
        FieldType::Rate => {
            let window_size = f.window_size.unwrap_or(3);
            Ok(Field::Rate(RateField {
                name: f.name.clone(),
                window_size,
            }))
        }
    }
}

fn resolve_activity(
    ad: &schema::ActivityDef,
    field_map: &HashMap<String, Field>,
    geometry_map: &HashMap<String, Geometry>,
    activity_ids: &HashMap<String, bool>,
) -> Result<Activity, CpmlError> {
    let start = parse_datetime(&ad.timespan.start)?;
    let end = parse_datetime(&ad.timespan.end)?;

    if end <= start {
        return Err(CpmlError::ValidationError(format!(
            "Activity '{}': end date must be after start date",
            ad.id
        )));
    }

    let mut probes = Vec::new();
    for (i, pd) in ad.probes.iter().enumerate() {
        let probe = resolve_probe(pd, &ad.id, i, field_map, geometry_map)?;
        probes.push(probe);
    }

    let mut projections = Vec::new();
    for (i, prd) in ad.projections.iter().enumerate() {
        let projection = resolve_projection(prd, &ad.id, i, field_map, geometry_map, start)?;
        projections.push(projection);
    }

    let mut dependencies = Vec::new();
    for dd in &ad.depends_on {
        if !activity_ids.contains_key(&dd.activity_id) {
            return Err(CpmlError::ReferenceError(format!(
                "Activity '{}' depends on unknown activity '{}'",
                ad.id, dd.activity_id
            )));
        }
        if dd.activity_id == ad.id {
            return Err(CpmlError::ValidationError(format!(
                "Activity '{}' cannot depend on itself",
                ad.id
            )));
        }
        let kind = match dd.kind {
            schema::DependencyKind::Fs => DependencyKind::FS,
            schema::DependencyKind::Ss => DependencyKind::SS,
            schema::DependencyKind::Ff => DependencyKind::FF,
            schema::DependencyKind::Sf => DependencyKind::SF,
        };
        dependencies.push(Dependency {
            activity_id: dd.activity_id.clone(),
            kind,
            lag_days: dd.lag_days.unwrap_or(0),
        });
    }

    Ok(Activity {
        id: ad.id.clone(),
        name: ad.name.clone(),
        series: ad.series.clone(),
        timespan: Timespan { start, end },
        geometry: ad.geometry.clone(),
        probes,
        projections,
        depends_on: dependencies,
    })
}

fn resolve_probe(
    pd: &schema::ProbeDef,
    activity_id: &str,
    index: usize,
    field_map: &HashMap<String, Field>,
    geometry_map: &HashMap<String, Geometry>,
) -> Result<Probe, CpmlError> {
    let field = field_map.get(&pd.field).ok_or_else(|| {
        CpmlError::ReferenceError(format!(
            "Probe '{}' in activity '{}' references unknown field '{}'",
            pd.name.as_deref().unwrap_or("unnamed"),
            activity_id,
            pd.field
        ))
    })?;

    let geom = geometry_map.get(&pd.geometry).ok_or_else(|| {
        CpmlError::ReferenceError(format!(
            "Probe '{}' in activity '{}' references unknown geometry '{}'",
            pd.name.as_deref().unwrap_or("unnamed"),
            activity_id,
            pd.geometry
        ))
    })?;

    let assertion = resolve_assertion(&pd.assert, field)?;

    let probe_name = pd
        .name
        .clone()
        .unwrap_or_else(|| format!("probe_{}", index));
    let id = format!("{}/{}", activity_id, probe_name);

    let diagnostic_level = pd
        .diagnostic_level
        .unwrap_or_else(|| default_diagnostic_level(field, &assertion));

    Ok(Probe {
        id,
        name: pd.name.clone(),
        field_name: pd.field.clone(),
        geometry: geom.clone(),
        assertion,
        diagnostic_level,
        parent_activity_id: activity_id.to_string(),
    })
}

pub(crate) fn resolve_assertion(ad: &AssertionDef, field: &Field) -> Result<Assertion, CpmlError> {
    match ad {
        AssertionDef::Empty { .. } => match field {
            Field::Occupancy(_) => Ok(Assertion::Empty),
            _ => Err(CpmlError::TypeMismatchError(format!(
                "Empty assertion is only valid for occupancy fields, got {:?}",
                field.field_type()
            ))),
        },
        AssertionDef::Gte { gte } => match field {
            Field::Capacity(_) | Field::Scalar(_) | Field::Rate(_) => Ok(Assertion::Gte(*gte)),
            _ => Err(CpmlError::TypeMismatchError(format!(
                "Gte assertion is only valid for capacity/scalar/rate fields, got {:?}",
                field.field_type()
            ))),
        },
        AssertionDef::Lte { lte } => match field {
            Field::Capacity(_) | Field::Scalar(_) | Field::Rate(_) => Ok(Assertion::Lte(*lte)),
            _ => Err(CpmlError::TypeMismatchError(format!(
                "Lte assertion is only valid for capacity/scalar/rate fields, got {:?}",
                field.field_type()
            ))),
        },
        AssertionDef::Range { min, max } => match field {
            Field::Capacity(_) | Field::Scalar(_) | Field::Rate(_) => Ok(Assertion::Range {
                min: *min,
                max: *max,
            }),
            _ => Err(CpmlError::TypeMismatchError(format!(
                "Range assertion is only valid for capacity/scalar/rate fields, got {:?}",
                field.field_type()
            ))),
        },
        AssertionDef::Present { present } => match field {
            Field::Presence(_) => Ok(Assertion::Present(PresenceCriteria {
                key: present.key.clone(),
                record_type: present.record_type.clone(),
                attributes: present.attributes.clone(),
            })),
            _ => Err(CpmlError::TypeMismatchError(format!(
                "Present assertion is only valid for presence fields, got {:?}",
                field.field_type()
            ))),
        },
    }
}

fn resolve_projection(
    prd: &schema::ProjectionDef,
    activity_id: &str,
    index: usize,
    field_map: &HashMap<String, Field>,
    geometry_map: &HashMap<String, Geometry>,
    valid_from: NaiveDateTime,
) -> Result<Projection, CpmlError> {
    let field = field_map.get(&prd.field).ok_or_else(|| {
        CpmlError::ReferenceError(format!(
            "Projection '{}' in activity '{}' references unknown field '{}'",
            prd.name.as_deref().unwrap_or("unnamed"),
            activity_id,
            prd.field
        ))
    })?;

    let geom = geometry_map.get(&prd.geometry).ok_or_else(|| {
        CpmlError::ReferenceError(format!(
            "Projection '{}' in activity '{}' references unknown geometry '{}'",
            prd.name.as_deref().unwrap_or("unnamed"),
            activity_id,
            prd.geometry
        ))
    })?;

    let contribution = match field {
        Field::Capacity(_) => {
            let value = prd.value.ok_or_else(|| {
                CpmlError::TypeMismatchError(format!(
                    "Projection '{}' on capacity field '{}' must specify 'value'",
                    prd.name.as_deref().unwrap_or("unnamed"),
                    prd.field
                ))
            })?;
            Contribution::Capacity(value)
        }
        Field::Occupancy(_) => {
            let kind = prd.kind.unwrap_or(OccupancyKind::Hard);
            Contribution::Occupancy { kind }
        }
        Field::Scalar(sf) => {
            let value = prd.value.ok_or_else(|| {
                CpmlError::TypeMismatchError(format!(
                    "Projection '{}' on scalar field '{}' must specify 'value'",
                    prd.name.as_deref().unwrap_or("unnamed"),
                    prd.field
                ))
            })?;
            let operator = prd.operator.unwrap_or(sf.operator);
            Contribution::Scalar { value, operator }
        }
        Field::Presence(_) => {
            let record_def = prd.record.as_ref().ok_or_else(|| {
                CpmlError::TypeMismatchError(format!(
                    "Projection '{}' on presence field '{}' must specify 'record'",
                    prd.name.as_deref().unwrap_or("unnamed"),
                    prd.field
                ))
            })?;
            let valid_until = record_def
                .valid_until
                .as_ref()
                .map(|s| parse_date(s))
                .transpose()?;
            Contribution::Presence(PresenceRecord {
                key: record_def.key.clone(),
                record_type: record_def.record_type.clone(),
                attributes: record_def.attributes.clone(),
                valid_from: valid_from.date(),
                valid_until,
            })
        }
        Field::Rate(_) => {
            let value = prd.value.ok_or_else(|| {
                CpmlError::TypeMismatchError(format!(
                    "Projection '{}' on rate field '{}' must specify 'value'",
                    prd.name.as_deref().unwrap_or("unnamed"),
                    prd.field
                ))
            })?;
            Contribution::Rate(value)
        }
    };

    let proj_name = prd
        .name
        .clone()
        .unwrap_or_else(|| format!("projection_{}", index));
    let id = format!("{}/{}", activity_id, proj_name);

    Ok(Projection {
        id,
        name: prd.name.clone(),
        field_name: prd.field.clone(),
        geometry: geom.clone(),
        contribution,
        parent_activity_id: activity_id.to_string(),
        confidence: prd.confidence,
    })
}

fn default_diagnostic_level(field: &Field, _assertion: &Assertion) -> DiagnosticLevel {
    match field {
        Field::Occupancy(_) => DiagnosticLevel::Error,
        _ => DiagnosticLevel::Error,
    }
}

/// Parse an ISO 8601 datetime string. Accepts full datetime with time,
/// or date-only (defaults to midnight).
fn parse_datetime(s: &str) -> Result<NaiveDateTime, CpmlError> {
    // Try full datetime format: "2026-01-15T14:30:00"
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Ok(dt);
    }
    // Try datetime with timezone offset (ignore timezone)
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%z") {
        return Ok(dt);
    }
    // Try minute-precision: "2026-06-01T08:00"
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M") {
        return Ok(dt);
    }
    // Fall back to date-only (midnight)
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map(|d| d.and_hms_opt(0, 0, 0).expect("midnight is always valid"))
        .map_err(|e| CpmlError::DateParseError(format!("Invalid date/datetime '{}': {}", s, e)))
}

fn parse_date(s: &str) -> Result<NaiveDate, CpmlError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| CpmlError::DateParseError(format!("Invalid date '{}': {}", s, e)))
}

/// Validate that activities sharing the same series ID have non-overlapping timespans.
fn validate_series(activities: &[Activity]) -> Result<(), CpmlError> {
    let mut series_map: HashMap<String, Vec<&Activity>> = HashMap::new();
    for a in activities {
        if let Some(ref sid) = a.series {
            series_map.entry(sid.clone()).or_default().push(a);
        }
    }
    for (sid, members) in &series_map {
        for i in 0..members.len() {
            for j in (i + 1)..members.len() {
                let a = members[i];
                let b = members[j];
                // Timespans overlap if a.start < b.end AND b.start < a.end
                if a.timespan.start < b.timespan.end && b.timespan.start < a.timespan.end {
                    return Err(CpmlError::ValidationError(format!(
                        "Series '{}': activities '{}' and '{}' have overlapping timespans ({}..{} and {}..{})",
                        sid,
                        a.id,
                        b.id,
                        a.timespan.start,
                        a.timespan.end,
                        b.timespan.start,
                        b.timespan.end
                    )));
                }
            }
        }
    }
    Ok(())
}
