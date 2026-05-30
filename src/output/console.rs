use chrono::NaiveDateTime;

use crate::pipeline::metric::MetricSeries;
use crate::pipeline::orchestrator::PipelineResult;
use crate::pipeline::probe_check::Diagnostic;
use crate::schema::DiagnosticLevel;

/// Print a human-readable pipeline result to stdout.
pub fn print_result(result: &PipelineResult) {
    println!("=== CPML Compilation Report ===");
    if let Some(ref name) = result.model.name {
        println!("Project: {}", name);
    }
    if let Some(ref desc) = result.model.description {
        println!("Description: {}", desc);
    }
    println!();

    let total = result.diagnostics.len();

    let errors: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.level >= DiagnosticLevel::Error)
        .collect();

    let warnings: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.level == DiagnosticLevel::Warning)
        .collect();

    let infos: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.level <= DiagnosticLevel::Info)
        .collect();

    // Summary
    println!(
        "Total diagnostics: {} ({} errors, {} warnings, {} info/debug)",
        total,
        errors.len(),
        warnings.len(),
        infos.len()
    );
    println!();

    println!("Schedule duration: {} days", result.schedule_duration);
    println!();

    if total == 0 {
        println!("No issues detected. All probes passed.");
    } else {
        // Print by severity
        print_diagnostics("ERRORS", &errors);
        print_diagnostics("WARNINGS", &warnings);
        print_diagnostics("INFO/DEBUG", &infos);
    }

    // Print metric time series
    print_metrics(&result.metrics);
}

fn print_diagnostics(label: &str, diags: &[&Diagnostic]) {
    if diags.is_empty() {
        return;
    }
    println!("── {} ──", label);
    for d in diags {
        let level_str = format!("{:?}", d.level).to_uppercase();
        let date_str = format_keyframe_date(d.keyframe_date);
        print!(
            "  [{}] Keyframe {} ({}) | Activity: {}",
            level_str, d.keyframe_index, date_str, d.activity_id
        );
        if let Some(ref sid) = d.series_id {
            print!(" (series: {})", sid);
        }
        println!(" | Probe: {}", d.probe_id);
        println!("    {}", d.message);
        if !d.blame.is_empty() {
            println!("    Blame:");
            for b in &d.blame {
                if let Some(conf) = b.confidence {
                    println!(
                        "      - {} :: {} ({}) [confidence: {:.2}]",
                        b.activity_id, b.projection_id, b.contribution_summary, conf
                    );
                } else {
                    println!(
                        "      - {} :: {} ({})",
                        b.activity_id, b.projection_id, b.contribution_summary
                    );
                }
            }
        }
        println!();
    }
}

/// Format a NaiveDateTime for display. Shows only the date if time is midnight,
/// otherwise includes the time component.
fn format_keyframe_date(dt: NaiveDateTime) -> String {
    if dt.format("%H:%M:%S").to_string() == "00:00:00" {
        dt.format("%Y-%m-%d").to_string()
    } else {
        dt.format("%Y-%m-%dT%H:%M").to_string()
    }
}

fn print_metrics(metrics: &[MetricSeries]) {
    if metrics.is_empty() {
        return;
    }
    println!("── METRICS ──");
    for series in metrics {
        println!(
            "  {} [{}] — {}",
            series.config.name, series.config.unit, series.config.description
        );
        if let Some(last) = series.points.last() {
            println!(
                "    Final cumulative: {:.1} {}",
                last.cumulative, series.config.unit
            );
        }
        // Print per-keyframe breakdown
        for pt in &series.points {
            if pt.incremental > 0.0 {
                println!(
                    "    KF {} ({}): +{:.1} → {:.1}",
                    pt.keyframe_index,
                    format_keyframe_date(pt.keyframe_date),
                    pt.incremental,
                    pt.cumulative
                );
            }
        }
        println!();
    }
}
