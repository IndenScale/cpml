use chrono::NaiveDateTime;
use serde::Serialize;

use crate::schema::DiagnosticLevel;

use super::keyframe::Keyframe;
use super::probe_check::Diagnostic;

/// Configuration for a metric.
#[derive(Debug, Clone, Serialize)]
pub struct MetricConfig {
    pub name: String,
    pub description: String,
    pub unit: String,
}

/// A single metric data point at a keyframe.
#[derive(Debug, Clone, Serialize)]
pub struct MetricPoint {
    pub keyframe_index: usize,
    pub keyframe_date: NaiveDateTime,
    /// Increment added at this keyframe.
    pub incremental: f64,
    /// Cumulative total from keyframe 0 to this point.
    pub cumulative: f64,
}

/// A full time series for one metric.
#[derive(Debug, Clone, Serialize)]
pub struct MetricSeries {
    pub config: MetricConfig,
    pub points: Vec<MetricPoint>,
}

/// Weight mapping from (DiagnosticLevel) to metric increment.
fn level_weight(level: DiagnosticLevel) -> f64 {
    match level {
        DiagnosticLevel::Debug => 0.0,
        DiagnosticLevel::Info => 1.0,
        DiagnosticLevel::Warning => 3.0,
        DiagnosticLevel::Error => 10.0,
        DiagnosticLevel::Fatal => 50.0,
    }
}

/// Compute metric time series from diagnostics and keyframes.
pub fn compute_metrics(diagnostics: &[Diagnostic], keyframes: &[Keyframe]) -> Vec<MetricSeries> {
    let risk_config = MetricConfig {
        name: "risk_index".into(),
        description: "累积风险指数，按诊断严重度加权".into(),
        unit: "points".into(),
    };
    let cost_config = MetricConfig {
        name: "cost_impact".into(),
        description: "预估成本影响，按诊断严重度加权".into(),
        unit: "万元".into(),
    };

    let mut risk_points: Vec<MetricPoint> = Vec::new();
    let mut cost_points: Vec<MetricPoint> = Vec::new();
    let mut cum_risk = 0.0;
    let mut cum_cost = 0.0;

    for kf in keyframes {
        let frame_diags: Vec<&Diagnostic> = diagnostics
            .iter()
            .filter(|d| d.keyframe_index == kf.index)
            .collect();

        let inc_risk: f64 = frame_diags.iter().map(|d| level_weight(d.level)).sum();
        let inc_cost: f64 = frame_diags
            .iter()
            .map(|d| match d.level {
                DiagnosticLevel::Debug => 0.0,
                DiagnosticLevel::Info => 0.0,
                DiagnosticLevel::Warning => 1.0,
                DiagnosticLevel::Error => 5.0,
                DiagnosticLevel::Fatal => 20.0,
            })
            .sum();

        cum_risk += inc_risk;
        cum_cost += inc_cost;

        risk_points.push(MetricPoint {
            keyframe_index: kf.index,
            keyframe_date: kf.date,
            incremental: inc_risk,
            cumulative: cum_risk,
        });
        cost_points.push(MetricPoint {
            keyframe_index: kf.index,
            keyframe_date: kf.date,
            incremental: inc_cost,
            cumulative: cum_cost,
        });
    }

    vec![
        MetricSeries {
            config: risk_config,
            points: risk_points,
        },
        MetricSeries {
            config: cost_config,
            points: cost_points,
        },
    ]
}
