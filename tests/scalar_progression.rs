use cpml::pipeline::run_pipeline;
use cpml::schema::DiagnosticLevel;

#[test]
fn test_strength_fails_before_curing_complete() {
    let input = include_str!("../samples/scalar_progression.cpml");
    let result = run_pipeline(input).unwrap();

    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.level >= DiagnosticLevel::Error)
        .collect();

    // Should have at least one error when strength < 0.7
    assert!(
        !errors.is_empty(),
        "Expected errors when strength is insufficient"
    );

    // The error should be at keyframe 3 (2026-01-10), when only curing phase 1 is active (0.50)
    let early_errors: Vec<_> = errors.iter().filter(|d| d.keyframe_index == 3).collect();
    assert_eq!(
        early_errors.len(),
        1,
        "Expected exactly 1 error at keyframe 3 (strength 0.50 < 0.70)"
    );
}

#[test]
fn test_strength_passes_after_full_curing() {
    let input = include_str!("../samples/scalar_progression.cpml");
    let result = run_pipeline(input).unwrap();

    // After keyframe 6 (curing phase 2 complete, strength 0.85), no more errors
    let late_errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.level >= DiagnosticLevel::Error && d.keyframe_index >= 5)
        .collect();
    assert!(
        late_errors.is_empty(),
        "Expected no errors after curing phase 2 (strength >= 0.70)"
    );
}
