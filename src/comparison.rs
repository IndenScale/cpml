use serde::Serialize;

use crate::pipeline::orchestrator::PipelineResult;
use crate::pipeline::probe_check::Diagnostic;

/// Summary of a comparison between two pipeline results.
#[derive(Debug, Serialize)]
pub struct ComparisonSummary {
    pub name_a: String,
    pub name_b: String,
    pub schedule_days_a: i64,
    pub schedule_days_b: i64,
    pub schedule_delta_days: i64,
    pub risk_final_a: f64,
    pub risk_final_b: f64,
    pub risk_delta: f64,
    pub cost_final_a: f64,
    pub cost_final_b: f64,
    pub cost_delta: f64,
    pub diag_count_a: usize,
    pub diag_count_b: usize,
    pub unique_to_a: Vec<Diagnostic>,
    pub unique_to_b: Vec<Diagnostic>,
}

/// Compare two pipeline results and produce a structured summary.
pub fn compare_results(a: &PipelineResult, b: &PipelineResult) -> ComparisonSummary {
    let risk_a = a
        .metrics
        .iter()
        .find(|m| m.config.name == "risk_index")
        .and_then(|m| m.points.last().map(|p| p.cumulative))
        .unwrap_or(0.0);
    let risk_b = b
        .metrics
        .iter()
        .find(|m| m.config.name == "risk_index")
        .and_then(|m| m.points.last().map(|p| p.cumulative))
        .unwrap_or(0.0);

    let cost_a = a
        .metrics
        .iter()
        .find(|m| m.config.name == "cost_impact")
        .and_then(|m| m.points.last().map(|p| p.cumulative))
        .unwrap_or(0.0);
    let cost_b = b
        .metrics
        .iter()
        .find(|m| m.config.name == "cost_impact")
        .and_then(|m| m.points.last().map(|p| p.cumulative))
        .unwrap_or(0.0);

    let unique_to_a: Vec<Diagnostic> = a
        .diagnostics
        .iter()
        .filter(|da| !b.diagnostics.iter().any(|db| same_diagnostic(da, db)))
        .cloned()
        .collect();
    let unique_to_b: Vec<Diagnostic> = b
        .diagnostics
        .iter()
        .filter(|db| !a.diagnostics.iter().any(|da| same_diagnostic(da, db)))
        .cloned()
        .collect();

    let name_a = a.model.name.clone().unwrap_or_else(|| "Scenario A".into());
    let name_b = b.model.name.clone().unwrap_or_else(|| "Scenario B".into());

    ComparisonSummary {
        name_a,
        name_b,
        schedule_days_a: a.schedule_duration,
        schedule_days_b: b.schedule_duration,
        schedule_delta_days: a.schedule_duration - b.schedule_duration,
        risk_final_a: risk_a,
        risk_final_b: risk_b,
        risk_delta: risk_a - risk_b,
        cost_final_a: cost_a,
        cost_final_b: cost_b,
        cost_delta: cost_a - cost_b,
        diag_count_a: a.diagnostics.len(),
        diag_count_b: b.diagnostics.len(),
        unique_to_a,
        unique_to_b,
    }
}

fn same_diagnostic(a: &Diagnostic, b: &Diagnostic) -> bool {
    a.keyframe_index == b.keyframe_index
        && a.activity_id == b.activity_id
        && a.probe_id == b.probe_id
        && a.message == b.message
}
