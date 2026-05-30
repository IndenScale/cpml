use cpml::pipeline::run_pipeline;
use cpml::schema::DiagnosticLevel;

#[test]
fn test_hard_bodies_no_overlap_no_error() {
    let input = include_str!("../samples/collision_demo.cpml");
    let result = run_pipeline(input).unwrap();

    let hard_probe_errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.probe_id.contains("hard") && d.level >= DiagnosticLevel::Error)
        .collect();
    assert!(
        hard_probe_errors.is_empty(),
        "Expected no errors from hard probes, got {}",
        hard_probe_errors.len()
    );
}

#[test]
fn test_soft_swing_overlap_produces_warning() {
    let input = include_str!("../samples/collision_demo.cpml");
    let result = run_pipeline(input).unwrap();

    let soft_warnings: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.probe_id.contains("soft") && d.level == DiagnosticLevel::Warning)
        .collect();
    assert_eq!(
        soft_warnings.len(),
        2,
        "Expected 2 soft collision warnings, got {}",
        soft_warnings.len()
    );

    let crane_a_warn: Vec<_> = soft_warnings
        .iter()
        .filter(|d| d.activity_id == "crane_A")
        .collect();
    let crane_b_warn: Vec<_> = soft_warnings
        .iter()
        .filter(|d| d.activity_id == "crane_B")
        .collect();
    assert_eq!(crane_a_warn.len(), 1);
    assert_eq!(crane_b_warn.len(), 1);
}

#[test]
fn test_no_errors_when_zones_clear_bodies() {
    let input = include_str!("../samples/collision_demo.cpml");
    let result = run_pipeline(input).unwrap();

    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.level >= DiagnosticLevel::Error)
        .collect();
    assert!(errors.is_empty(), "Expected no errors with radius=8");
}

#[test]
fn test_occlusion_blocks_soft_collision() {
    let input = include_str!("../samples/occlusion_demo.cpml");
    let result = run_pipeline(input).unwrap();

    // With the barrier wall between the cranes, soft collision warnings
    // should be blocked by occlusion culling.
    let soft_warnings: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.probe_id.contains("soft") && d.level == DiagnosticLevel::Warning)
        .collect();
    assert!(
        soft_warnings.is_empty(),
        "Expected no soft collision warnings with barrier occlusion, got {}",
        soft_warnings.len()
    );

    // Hard collisions should still be absent (bodies don't overlap)
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.level >= DiagnosticLevel::Error)
        .collect();
    assert!(errors.is_empty(), "Expected no hard errors");
}
