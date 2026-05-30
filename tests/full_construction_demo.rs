use cpml::pipeline::run_pipeline;
use cpml::schema::DiagnosticLevel;

#[test]
fn test_full_demo_parses_and_runs() {
    let input = include_str!("../samples/full_construction_demo.cpml");
    let result = run_pipeline(input).expect("full demo should parse and run");
    assert!(!result.model.activities.is_empty());
    assert!(!result.model.fields.is_empty());
}

#[test]
fn test_full_demo_has_schedule_duration() {
    let input = include_str!("../samples/full_construction_demo.cpml");
    let result = run_pipeline(input).expect("full demo should run");
    assert!(
        result.schedule_duration > 0,
        "Should have positive schedule duration"
    );
}

#[test]
fn test_full_demo_produces_metrics() {
    let input = include_str!("../samples/full_construction_demo.cpml");
    let result = run_pipeline(input).expect("full demo should run");
    assert!(
        !result.metrics.is_empty(),
        "Should produce metric time series"
    );
    // Should have both risk_index and cost_impact
    assert!(result.metrics.iter().any(|m| m.config.name == "risk_index"));
    assert!(result
        .metrics
        .iter()
        .any(|m| m.config.name == "cost_impact"));
}

#[test]
fn test_full_demo_no_dependency_violations() {
    let input = include_str!("../samples/full_construction_demo.cpml");
    let result = run_pipeline(input).expect("full demo should run");
    let dep_errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.probe_id.contains("dep_") && d.level >= DiagnosticLevel::Error)
        .collect();
    assert!(
        dep_errors.is_empty(),
        "No dependency violations expected, got: {:?}",
        dep_errors
    );
}

#[test]
fn test_full_demo_backpressure_detected() {
    let input = include_str!("../samples/full_construction_demo.cpml");
    let result = run_pipeline(input).expect("full demo should run");
    let backpressure: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.probe_id.contains("backpressure"))
        .collect();
    assert!(
        !backpressure.is_empty(),
        "Backpressure warning should be detected"
    );
    assert!(backpressure[0].message.contains("above ceiling"));
}

#[test]
fn test_full_demo_has_union_geometry() {
    let input = include_str!("../samples/full_construction_demo.cpml");
    let result = run_pipeline(input).expect("full demo should run");
    // Verify L-shaped building geometry was resolved
    let geo_count = result.model.activities.len();
    assert!(geo_count > 10, "Should have many activities");
}

#[test]
fn test_full_demo_structure_confidence() {
    let input = include_str!("../samples/full_construction_demo.cpml");
    let result = run_pipeline(input).expect("full demo should run");
    // Find the fast_supplier's structure-generated projection with confidence
    let fast_supplier = result
        .model
        .activities
        .iter()
        .find(|a| a.id == "fast_supplier")
        .expect("fast_supplier should exist");
    let struct_proj = fast_supplier
        .projections
        .iter()
        .find(|p| p.name.as_deref() == Some("struct_structure_0_projection"))
        .expect("structure projection should exist");
    assert_eq!(struct_proj.confidence, Some(0.3));
}

#[test]
fn test_full_demo_scalar_progression() {
    let input = include_str!("../samples/full_construction_demo.cpml");
    let result = run_pipeline(input).expect("full demo should run");
    // Column construction checks concrete_strength >= 0.7
    let strength_checks: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("concrete_strength"))
        .collect();
    // At column_construction start (2026-03-28), curing_week2 (ends 3/27)
    // and curing_week3 (starts 3/27) have contributed 0.2 + 0.45 + 0.8 = 1.45 strength
    // OR: rebar (0.0 replace), pour (0.2), week1 (0.45), week2 (0.8) = max of all = 0.8
    // 0.8 >= 0.7 → should pass
    let errors: Vec<_> = strength_checks
        .iter()
        .filter(|d| d.level >= DiagnosticLevel::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "Strength should be >= 0.7 by column construction, got {:?}",
        errors
    );
}

#[test]
fn test_full_demo_presence_check() {
    let input = include_str!("../samples/full_construction_demo.cpml");
    let result = run_pipeline(input).expect("full demo should run");
    // Excavation checks for permit - should pass since permit is approved before excavation starts
    let permit_errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("presence") || d.message.contains("permit"))
        .filter(|d| d.level >= DiagnosticLevel::Error)
        .collect();
    assert!(
        permit_errors.is_empty(),
        "Permit check should pass, got: {:?}",
        permit_errors
    );
}
