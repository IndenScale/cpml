use std::collections::HashMap;

use chrono::NaiveDateTime;
use serde::Serialize;

use crate::error::CpmlError;
use crate::model::activity::CpmlModel;
use crate::model::field::Field;
use crate::model::geometry::Geometry;
use crate::schema::CpmlDocument;

use super::expand::expand_activity;
use super::field_eval::PersistentState;
use super::keyframe::{active_activities_at, extract_keyframes};
use super::metric::compute_metrics;
use super::probe_check::Diagnostic;
use super::resolve::{self, resolve};

use super::metric::MetricSeries;

/// The result of running the full compiler pipeline.
#[derive(Debug, Serialize)]
pub struct PipelineResult {
    pub model: CpmlModel,
    pub diagnostics: Vec<Diagnostic>,
    /// Metric time series computed from diagnostics.
    pub metrics: Vec<MetricSeries>,
    /// Project wall-clock duration in days: max(end) - min(start) across all activities.
    pub schedule_duration: i64,
}

/// Run the full CPML compiler pipeline on a YAML input string.
pub fn run_pipeline(input: &str) -> Result<PipelineResult, CpmlError> {
    // Stage 1: Parse YAML
    let doc: CpmlDocument = serde_yaml::from_str(input)?;

    // Stage 2+3: Validate, resolve, build model
    let mut model = resolve(doc.clone())?;

    // Stage 4: Expand collision/structures
    let mut field_map: HashMap<String, Field> = model
        .fields
        .iter()
        .map(|f| (f.name().to_string(), f.clone()))
        .collect();

    let geometry_map: HashMap<String, Geometry> = doc
        .geometries
        .iter()
        .map(|g| {
            let geometry =
                resolve::resolve_geometry_with_region(&g.shape, g.pose.as_ref(), g.region.clone());
            (g.id.clone(), geometry)
        })
        .collect();

    for (i, ad) in doc.activities.iter().enumerate() {
        expand_activity(&mut model.activities[i], ad, &field_map, &geometry_map)?;
    }

    // Rebuild field map after expansion
    field_map = model
        .fields
        .iter()
        .map(|f| (f.name().to_string(), f.clone()))
        .collect();

    // Stage 5: Extract keyframes
    let keyframes = extract_keyframes(&model.activities);

    // Stage 6: Evaluate each keyframe
    let mut all_diagnostics: Vec<Diagnostic> = Vec::new();
    let mut persistent_state = PersistentState::new();

    for kf in &keyframes {
        let active = active_activities_at(kf, &model.activities);

        // Check probes for this frame
        let ctx = super::probe_check::ProbeCheckCtx {
            all_activities: &model.activities,
            field_map: &field_map,
            barriers: &model.barriers,
            region_hierarchy: &model.region_hierarchy,
        };
        let frame_diags = super::probe_check::check_probes(
            kf.index,
            kf.date,
            &active,
            &persistent_state,
            &ctx,
        );
        all_diagnostics.extend(frame_diags);

        // Update persistent state for next frame
        update_persistent_state(
            &mut persistent_state,
            &active,
            &model.activities,
            &field_map,
            kf.date,
        );
    }

    // Check activity dependencies
    let dep_diags = super::dependency_check::check_dependencies(&model.activities, &keyframes);
    all_diagnostics.extend(dep_diags);

    // Compute schedule duration: wall time from earliest start to latest end
    let schedule_duration = model
        .activities
        .iter()
        .map(|a| (a.timespan.start, a.timespan.end))
        .fold(
            None,
            |acc: Option<(NaiveDateTime, NaiveDateTime)>, (s, e)| match acc {
                None => Some((s, e)),
                Some((min_start, max_end)) => Some((min_start.min(s), max_end.max(e))),
            },
        )
        .map(|(min_start, max_end)| (max_end - min_start).num_days())
        .unwrap_or(0);

    // Compute metric time series from diagnostics
    let metrics = compute_metrics(&all_diagnostics, &keyframes);

    Ok(PipelineResult {
        model,
        diagnostics: all_diagnostics,
        metrics,
        schedule_duration,
    })
}

fn update_persistent_state(
    state: &mut PersistentState,
    active_activities: &[&crate::model::activity::Activity],
    all_activities: &[crate::model::activity::Activity],
    field_map: &HashMap<String, Field>,
    current_date: chrono::NaiveDateTime,
) {
    // Collect active projection IDs
    let active_ids: std::collections::HashSet<String> = active_activities
        .iter()
        .flat_map(|a| a.projections.iter().map(|p| p.id.clone()))
        .collect();

    let all_projections: Vec<&crate::model::projection::Projection> = all_activities
        .iter()
        .flat_map(|a| a.projections.iter())
        .collect();

    let active_projections: Vec<&crate::model::projection::Projection> = all_projections
        .iter()
        .filter(|p| active_ids.contains(&p.id))
        .copied()
        .collect();

    for (name, field) in field_map {
        let field_projections: Vec<&crate::model::projection::Projection> = active_projections
            .iter()
            .filter(|p| p.field_name == *name)
            .copied()
            .collect();

        match field {
            Field::Scalar(sf) => {
                let prev = state.scalar.get(name).cloned().unwrap_or_default();
                let updated =
                    super::field_eval::update_scalar_state(&prev, &field_projections, sf.operator);
                state.scalar.insert(name.clone(), updated);
            }
            Field::Presence(_) => {
                let prev = state.presence.get(name).cloned().unwrap_or_default();
                let updated = super::field_eval::update_presence_state(
                    &prev,
                    &field_projections,
                    current_date,
                );
                state.presence.insert(name.clone(), updated);
            }
            Field::Rate(rf) => {
                let prev = state.rate.get(name).cloned().unwrap_or_default();
                let updated = super::field_eval::update_rate_state(
                    &prev,
                    &field_projections,
                    current_date,
                    rf.window_size,
                );
                state.rate.insert(name.clone(), updated);
            }
            _ => {}
        }
    }
}
