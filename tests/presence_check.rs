use cpml::pipeline::run_pipeline;
use cpml::schema::DiagnosticLevel;

#[test]
fn test_early_excavation_fails_without_permit() {
    let input = include_str!("../samples/presence_permit.cpml");
    let result = run_pipeline(input).unwrap();

    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.level >= DiagnosticLevel::Error)
        .collect();

    assert_eq!(
        errors.len(),
        1,
        "Expected exactly 1 error for early excavation"
    );

    let error = &errors[0];
    assert_eq!(error.activity_id, "excavation_early");
    assert_eq!(error.keyframe_index, 0); // 2026-01-05, before permit approval
    assert!(
        error.message.contains("excavation_permit_001"),
        "Error should mention the missing permit"
    );
}

#[test]
fn test_late_excavation_passes_with_permit() {
    let input = include_str!("../samples/presence_permit.cpml");
    let result = run_pipeline(input).unwrap();

    let late_errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.activity_id == "excavation_on_time" && d.level >= DiagnosticLevel::Error)
        .collect();
    assert!(
        late_errors.is_empty(),
        "On-time excavation should pass (permit is approved)"
    );
}
