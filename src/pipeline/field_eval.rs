use std::collections::HashMap;

use parry3d::shape::SharedShape;

use crate::model::geometry::{Aabb, Geometry, RegionOverlap};
use crate::model::projection::{Contribution, PresenceRecord, Projection};
use crate::schema::{OccupancyKind, ScalarOperator};

/// Check whether a projection is occluded from the probe by any barrier geometry.
/// Casts a ray from the projection center to the probe center; if any barrier
/// intersects the ray, the projection is considered occluded.
fn is_occluded(probe_geom: &Geometry, proj_geom: &Geometry, barriers: &[Geometry]) -> bool {
    if barriers.is_empty() {
        return false;
    }

    let probe_center = probe_geom.pose.position;
    let proj_center = proj_geom.pose.position;

    let ray_origin = parry3d::glamx::DVec3::from(proj_center);
    let ray_end = parry3d::glamx::DVec3::from(probe_center);
    let ray_dir = ray_end - ray_origin;
    let ray_len = ray_dir.length();
    if ray_len < 1e-10 {
        return false; // Co-located, no meaningful ray
    }
    let ray_dir = ray_dir / ray_len;

    let ray = parry3d::query::Ray::new(ray_origin, ray_dir);
    let max_toi = ray_len;

    for barrier in barriers {
        let barrier_shape: SharedShape = barrier.shape.to_parry_shape();
        let barrier_pose = barrier.pose.to_parry_pose();
        if let Some(toi) = barrier_shape.cast_ray(&barrier_pose, &ray, max_toi, false) {
            if toi < max_toi {
                return true;
            }
        }
    }
    false
}

/// Persistent state carried across keyframes for ScalarField, PresenceField, and RateField.
/// State is spatially keyed by region_key, same as non-persistent field evaluation.
#[derive(Debug, Clone, Default)]
pub struct PersistentState {
    /// Scalar: field_name -> region_key -> accumulated value
    pub scalar: HashMap<String, HashMap<String, f64>>,
    /// Presence: field_name -> region_key -> record_key -> record
    pub presence: HashMap<String, HashMap<String, HashMap<String, PresenceRecord>>>,
    /// Rate: field_name -> region_key -> Vec<(date, value)> history for rate computation
    pub rate: HashMap<String, HashMap<String, Vec<(chrono::NaiveDateTime, f64)>>>,
}

impl PersistentState {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Evaluate a capacity field at a probe region: sum all active projection values
/// for the given field name, walking up the region hierarchy so that child regions
/// see contributions from ancestor regions.
///
/// Each contribution is weighted by its confidence (default 1.0). A single
/// projection is counted at most once even if it overlaps multiple ancestor
/// regions (deduplication by projection ID).
pub fn eval_capacity(
    active_projections: &[&Projection],
    probe_region_key: &str,
    field_name: &str,
    region_hierarchy: &std::collections::HashMap<String, String>,
) -> f64 {
    // Collect all region keys in the ancestor chain.
    let mut ancestor_keys = vec![probe_region_key.to_string()];
    let mut current = probe_region_key;
    while let Some(parent) = region_hierarchy.get(current) {
        ancestor_keys.push(parent.clone());
        current = parent;
    }

    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut total = 0.0;
    for region_key in &ancestor_keys {
        for p in active_projections {
            if p.field_name != field_name {
                continue;
            }
            if seen.contains(p.id.as_str()) {
                continue;
            }
            if !region_overlaps(&p.geometry, region_key) {
                continue;
            }
            seen.insert(p.id.as_str());
            if let Contribution::Capacity(v) = &p.contribution {
                let w = p.confidence.unwrap_or(1.0);
                total += *v * w;
            }
        }
    }
    total
}

/// Evaluate the unified occupancy field at a probe region.
/// Returns the most severe OccupancyKind among overlapping projections
/// (Hard > Soft), or None if the region is vacant.
/// Uses three-phase collision:
///   1. AABB fast-reject
///   2. GJK exact intersection test
///   3. Occlusion culling (raycast against barriers)
///
/// Excludes projections from the same activity (self-exclusion).
pub fn eval_occupancy(
    active_projections: &[&Projection],
    probe_geom: &Geometry,
    self_activity_id: &str,
    series_id: Option<&str>,
    series_map: &HashMap<String, Option<String>>,
    barriers: &[Geometry],
) -> Option<OccupancyKind> {
    let probe_region_key = probe_geom.region_key();
    let mut worst: Option<OccupancyKind> = None;
    for p in active_projections {
        if p.parent_activity_id == self_activity_id {
            continue;
        }
        // Self-exclusion: skip projections from activities in the same series
        if let Some(sid) = series_id {
            if let Some(proj_series) = series_map
                .get(&p.parent_activity_id)
                .and_then(|s| s.as_deref())
            {
                if proj_series == sid {
                    continue;
                }
            }
        }
        // Phase 1: AABB fast-reject
        if !region_overlaps(&p.geometry, &probe_region_key) {
            continue;
        }
        // Phase 2: GJK exact intersection test
        if !probe_geom.exact_intersects(&p.geometry) {
            continue;
        }
        // Phase 3: Occlusion culling — only applies to soft collisions
        if let Contribution::Occupancy {
            kind: OccupancyKind::Soft,
        } = &p.contribution
        {
            if is_occluded(probe_geom, &p.geometry, barriers) {
                continue;
            }
        }
        if let Contribution::Occupancy { kind } = &p.contribution {
            match kind {
                OccupancyKind::Hard => return Some(OccupancyKind::Hard),
                OccupancyKind::Soft => {
                    worst = Some(OccupancyKind::Soft);
                }
            }
        }
    }
    worst
}

/// Evaluate a scalar field at a probe region.
/// Combines persistent state from overlapping regions, then applies
/// active projections whose geometry also overlaps the probe region.
pub fn eval_scalar(
    active_projections: &[&Projection],
    prev_state: &HashMap<String, f64>,
    probe_region_key: &str,
    operator: ScalarOperator,
) -> f64 {
    // Merge persistent state from all regions that overlap the probe
    let mut base = 0.0;
    let mut found = false;
    for (region_key, value) in prev_state {
        if region_key == probe_region_key || region_overlaps_key(region_key, probe_region_key) {
            if !found {
                base = *value;
                found = true;
            } else {
                base = apply_scalar_op(base, *value, operator);
            }
        }
    }

    // Apply active projections whose geometry overlaps the probe
    let mut result = base;
    for p in active_projections {
        if !region_overlaps(&p.geometry, probe_region_key) {
            continue;
        }
        if let Contribution::Scalar {
            value,
            operator: op,
        } = &p.contribution
        {
            let w = p.confidence.unwrap_or(1.0);
            result = apply_scalar_op(result, *value * w, *op);
        }
    }
    result
}

fn apply_scalar_op(current: f64, value: f64, operator: ScalarOperator) -> f64 {
    match operator {
        ScalarOperator::Max => current.max(value),
        ScalarOperator::Min => {
            if current == 0.0 {
                value
            } else {
                current.min(value)
            }
        }
        ScalarOperator::Sum => current + value,
        ScalarOperator::Replace => value,
    }
}

/// Update persistent scalar state after evaluating all projections this keyframe.
pub fn update_scalar_state(
    current: &HashMap<String, f64>,
    active_projections: &[&Projection],
    operator: ScalarOperator,
) -> HashMap<String, f64> {
    let mut next = current.clone();
    for p in active_projections {
        if let Contribution::Scalar {
            value,
            operator: op,
        } = &p.contribution
        {
            let key = p.region_key();
            let prev = next.get(&key).copied().unwrap_or(0.0);
            let effective_op = if *op == operator { operator } else { *op };
            let w = p.confidence.unwrap_or(1.0);
            next.insert(key, apply_scalar_op(prev, *value * w, effective_op));
        }
    }
    next
}

/// Evaluate a presence field at a probe region.
pub fn eval_presence(
    active_projections: &[&Projection],
    prev_state: &HashMap<String, HashMap<String, PresenceRecord>>,
    probe_region_key: &str,
    key: &str,
    record_type: &Option<String>,
    attributes: &HashMap<String, String>,
    current_date: chrono::NaiveDateTime,
) -> bool {
    let mut all_records: Vec<&PresenceRecord> = Vec::new();
    for (region_key, records) in prev_state {
        if region_key == probe_region_key || region_overlaps_key(region_key, probe_region_key) {
            all_records.extend(records.values());
        }
    }
    for p in active_projections {
        if let Contribution::Presence(rec) = &p.contribution {
            if region_overlaps(&p.geometry, probe_region_key) {
                all_records.push(rec);
            }
        }
    }
    record_matches(&all_records, key, record_type, attributes, current_date)
}

fn record_matches(
    records: &[&PresenceRecord],
    key: &str,
    record_type: &Option<String>,
    attributes: &HashMap<String, String>,
    current_date: chrono::NaiveDateTime,
) -> bool {
    records.iter().any(|rec| {
        if rec.key != *key {
            return false;
        }
        if let Some(rt) = record_type {
            if rec.record_type.as_ref() != Some(rt) {
                return false;
            }
        }
        if current_date.date() < rec.valid_from {
            return false;
        }
        if let Some(until) = rec.valid_until {
            if current_date.date() > until {
                return false;
            }
        }
        for (ak, av) in attributes {
            if rec.attributes.get(ak) != Some(av) {
                return false;
            }
        }
        true
    })
}

/// Update persistent presence state after evaluating this keyframe.
pub fn update_presence_state(
    current: &HashMap<String, HashMap<String, PresenceRecord>>,
    active_projections: &[&Projection],
    current_date: chrono::NaiveDateTime,
) -> HashMap<String, HashMap<String, PresenceRecord>> {
    let mut next: HashMap<String, HashMap<String, PresenceRecord>> = HashMap::new();
    for (region, records) in current {
        let mut region_records = HashMap::new();
        for (k, rec) in records {
            let expired = rec
                .valid_until
                .map(|u| current_date.date() > u)
                .unwrap_or(false);
            if !expired {
                region_records.insert(k.clone(), rec.clone());
            }
        }
        if !region_records.is_empty() {
            next.insert(region.clone(), region_records);
        }
    }
    for p in active_projections {
        if let Contribution::Presence(rec) = &p.contribution {
            let region = p.region_key();
            let region_records = next.entry(region).or_default();
            region_records.insert(rec.key.clone(), rec.clone());
        }
    }
    next
}

/// Evaluate a rate field at a probe region.
/// Computes the rate of change (units per keyframe interval) from persistent state
/// across the configured window size, then adds active projection contributions.
pub fn eval_rate(
    active_projections: &[&Projection],
    prev_history: &[(chrono::NaiveDateTime, f64)],
    probe_region_key: &str,
    window_size: usize,
    current_date: chrono::NaiveDateTime,
) -> f64 {
    // Base rate from historical data
    let base_rate = compute_rate_from_history(prev_history, window_size, current_date);

    // Add active projection rate contributions (with confidence)
    let mut result = base_rate;
    for p in active_projections {
        if !region_overlaps(&p.geometry, probe_region_key) {
            continue;
        }
        if let Contribution::Rate(v) = &p.contribution {
            let w = p.confidence.unwrap_or(1.0);
            result += *v * w;
        }
    }
    result
}

/// Compute the rate of change from historical data over the configured window.
fn compute_rate_from_history(
    history: &[(chrono::NaiveDateTime, f64)],
    window_size: usize,
    _current_date: chrono::NaiveDateTime,
) -> f64 {
    if history.len() < 2 {
        return 0.0;
    }
    // Use the most recent `window_size + 1` entries for rate calculation
    let window: Vec<_> = if history.len() > window_size {
        history[history.len() - window_size - 1..].to_vec()
    } else {
        history.to_vec()
    };
    if window.len() < 2 {
        return 0.0;
    }
    let (first_date, first_val) = window[0];
    let (last_date, last_val) = window[window.len() - 1];
    let days = (last_date - first_date).num_days() as f64;
    if days <= 0.0 {
        return 0.0;
    }
    (last_val - first_val) / days
}

/// Update persistent rate state after evaluating this keyframe.
pub fn update_rate_state(
    current: &HashMap<String, Vec<(chrono::NaiveDateTime, f64)>>,
    active_projections: &[&Projection],
    current_date: chrono::NaiveDateTime,
    window_size: usize,
) -> HashMap<String, Vec<(chrono::NaiveDateTime, f64)>> {
    let mut next: HashMap<String, Vec<(chrono::NaiveDateTime, f64)>> = HashMap::new();
    for (region, history) in current {
        // Prune entries older than window_size + 1
        let mut pruned: Vec<_> = history.clone();
        if pruned.len() > window_size + 1 {
            pruned = pruned[pruned.len() - window_size - 1..].to_vec();
        }
        if !pruned.is_empty() {
            next.insert(region.clone(), pruned);
        }
    }
    for p in active_projections {
        if let Contribution::Rate(v) = &p.contribution {
            let region = p.region_key();
            let history = next.entry(region).or_default();
            let w = p.confidence.unwrap_or(1.0);
            history.push((current_date, *v * w));
            if history.len() > window_size + 1 {
                *history = history[history.len() - window_size - 1..].to_vec();
            }
        }
    }
    next
}

fn region_overlaps(geom: &Geometry, probe_key: &str) -> bool {
    let aabb = geom.world_aabb();
    aabb.region_key() == probe_key || aabb.overlaps_by_key(probe_key)
}

fn region_overlaps_key(key_a: &str, key_b: &str) -> bool {
    if key_a == key_b {
        return true;
    }
    match (Aabb::from_region_key(key_a), Aabb::from_region_key(key_b)) {
        (Some(a), Some(b)) => a.overlaps(&b),
        _ => false,
    }
}
