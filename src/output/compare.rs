use crate::comparison::ComparisonSummary;

/// Print a human-readable comparison to stdout.
pub fn print_comparison(summary: &ComparisonSummary) {
    println!("=== CPML Scenario Comparison ===");
    println!("Scenario A: {}", summary.name_a);
    println!("Scenario B: {}", summary.name_b);
    println!();

    // Schedule
    println!("── Schedule ──");
    println!("  A: {} days", summary.schedule_days_a);
    println!("  B: {} days", summary.schedule_days_b);
    let delta = summary.schedule_delta_days;
    if delta > 0 {
        println!("  Delta: +{} days (A is longer)", delta);
    } else if delta < 0 {
        println!("  Delta: {} days (A is shorter)", delta);
    } else {
        println!("  Delta: equal");
    }
    println!();

    // Risk Index
    println!("── Risk Index ──");
    println!("  A: {:.1} points", summary.risk_final_a);
    println!("  B: {:.1} points", summary.risk_final_b);
    let rd = summary.risk_delta;
    if rd > 0.0 {
        println!("  Delta: +{:.1} (A is riskier)", rd);
    } else if rd < 0.0 {
        println!("  Delta: {:.1} (A is safer)", rd);
    } else {
        println!("  Delta: equal");
    }
    println!();

    // Cost Impact
    println!("── Cost Impact ──");
    println!("  A: {:.1} 万元", summary.cost_final_a);
    println!("  B: {:.1} 万元", summary.cost_final_b);
    let cd = summary.cost_delta;
    if cd > 0.0 {
        println!("  Delta: +{:.1} (A costs more)", cd);
    } else if cd < 0.0 {
        println!("  Delta: {:.1} (A costs less)", cd);
    } else {
        println!("  Delta: equal");
    }
    println!();

    // Diagnostics
    println!("── Diagnostics ──");
    println!("  A: {} total", summary.diag_count_a);
    println!("  B: {} total", summary.diag_count_b);

    if !summary.unique_to_a.is_empty() {
        println!();
        println!("  Unique to A ({}):", summary.unique_to_a.len());
        for d in &summary.unique_to_a {
            println!(
                "    [{}] KF {} | {} | {}",
                format!("{:?}", d.level).to_uppercase(),
                d.keyframe_index,
                d.activity_id,
                d.message
            );
        }
    }

    if !summary.unique_to_b.is_empty() {
        println!();
        println!("  Unique to B ({}):", summary.unique_to_b.len());
        for d in &summary.unique_to_b {
            println!(
                "    [{}] KF {} | {} | {}",
                format!("{:?}", d.level).to_uppercase(),
                d.keyframe_index,
                d.activity_id,
                d.message
            );
        }
    }

    if summary.unique_to_a.is_empty() && summary.unique_to_b.is_empty() {
        println!("  No unique diagnostics — scenarios produce identical results.");
    }
    println!();
}

/// Print the comparison as JSON to stdout.
pub fn print_json_comparison(summary: &ComparisonSummary) {
    println!(
        "{}",
        serde_json::to_string_pretty(summary).expect("failed to serialize ComparisonSummary")
    );
}
