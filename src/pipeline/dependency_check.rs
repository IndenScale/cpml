use chrono::Duration;

use crate::model::activity::{Activity, DependencyKind};
use crate::pipeline::keyframe::Keyframe;
use crate::pipeline::probe_check::Diagnostic;
use crate::schema::DiagnosticLevel;

/// Check that all activity dependency constraints are satisfied.
/// Generates diagnostics for violated dependencies at each keyframe.
pub fn check_dependencies(activities: &[Activity], keyframes: &[Keyframe]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Build a lookup of activities by ID
    let activity_map: std::collections::HashMap<&str, &Activity> =
        activities.iter().map(|a| (a.id.as_str(), a)).collect();

    for activity in activities {
        if activity.depends_on.is_empty() {
            continue;
        }

        for dep in &activity.depends_on {
            let predecessor = match activity_map.get(dep.activity_id.as_str()) {
                Some(a) => *a,
                None => continue, // Reference validation handled in resolve
            };

            for kf in keyframes {
                let lag = Duration::days(dep.lag_days);
                let violated = match dep.kind {
                    DependencyKind::FS => {
                        // B cannot start until A finishes + lag.
                        // B is active now, but A hasn't finished yet.
                        let a_end_plus_lag = predecessor.timespan.end + lag;
                        activity.timespan.contains(kf.date) && kf.date < a_end_plus_lag
                    }
                    DependencyKind::SS => {
                        // B cannot start until A starts + lag.
                        // B is active now, but A hasn't started yet.
                        let a_start_plus_lag = predecessor.timespan.start + lag;
                        activity.timespan.contains(kf.date) && kf.date < a_start_plus_lag
                    }
                    DependencyKind::FF => {
                        // B cannot finish until A finishes + lag.
                        // B has ended, but A hasn't finished yet.
                        let a_end_plus_lag = predecessor.timespan.end + lag;
                        kf.date >= activity.timespan.end && kf.date < a_end_plus_lag
                    }
                    DependencyKind::SF => {
                        // B cannot finish until A starts + lag.
                        // B has ended, but A hasn't started yet.
                        let a_start_plus_lag = predecessor.timespan.start + lag;
                        kf.date >= activity.timespan.end && kf.date < a_start_plus_lag
                    }
                };

                if violated {
                    let kind_label = match dep.kind {
                        DependencyKind::FS => "FS",
                        DependencyKind::SS => "SS",
                        DependencyKind::FF => "FF",
                        DependencyKind::SF => "SF",
                    };
                    diagnostics.push(Diagnostic {
                        keyframe_index: kf.index,
                        keyframe_date: kf.date,
                        activity_id: activity.id.clone(),
                        probe_id: format!("{}/dep_{}", activity.id, dep.activity_id),
                        level: DiagnosticLevel::Error,
                        message: format!(
                            "Dependency violation: {} constraint on '{}' (lag {} days). \
                             Predecessor timespan: {}..{}",
                            kind_label,
                            dep.activity_id,
                            dep.lag_days,
                            predecessor.timespan.start,
                            predecessor.timespan.end,
                        ),
                        blame: vec![],
                        series_id: activity.series.clone(),
                    });
                }
            }
        }
    }

    diagnostics.sort_by_key(|b| std::cmp::Reverse(b.level));
    diagnostics
}
