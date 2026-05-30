use cpml::pipeline::run_pipeline;
use cpml::schema::DiagnosticLevel;

#[test]
fn test_resource_contention_detected() {
    let input = include_str!("../samples/resource_contention.cpml");
    let result = run_pipeline(input).unwrap();

    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.level >= DiagnosticLevel::Error)
        .collect();

    // Activities B and C should fail (total consumption exceeds supply)
    assert!(!errors.is_empty(), "Expected resource contention errors");

    let has_excavation_b = errors.iter().any(|d| d.activity_id == "excavation_B");
    let has_excavation_c = errors.iter().any(|d| d.activity_id == "excavation_C");
    assert!(has_excavation_b, "Expected excavation_B to fail");
    assert!(has_excavation_c, "Expected excavation_C to fail");
}

#[test]
fn test_excavation_a_passes() {
    let input = include_str!("../samples/resource_contention.cpml");
    let result = run_pipeline(input).unwrap();

    let a_errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.activity_id == "excavation_A")
        .collect();
    assert!(
        a_errors.is_empty(),
        "Excavation A should pass (has enough capacity)"
    );
}
