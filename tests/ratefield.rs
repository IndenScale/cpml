use cpml::pipeline::run_pipeline;
use cpml::schema::DiagnosticLevel;

#[test]
fn test_ratefield_basic_flow() {
    let input = include_str!("../samples/ratefield_demo.cpml");
    let result = run_pipeline(input).unwrap();

    // Both material_flow probes should pass: slow (gte 30) and fast (gte 120)
    // Total rate = 50*0.8 + 100 = 140, which exceeds both thresholds
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("material_flow") && d.level >= DiagnosticLevel::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "Expected no errors on material_flow, got {:?}",
        errors
    );

    // Backpressure: fast_upstream (250) > lte 150 → warning
    let backpressure: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.probe_id.contains("backpressure_check"))
        .collect();
    assert_eq!(backpressure.len(), 1);
    assert_eq!(backpressure[0].level, DiagnosticLevel::Warning);
    assert!(backpressure[0].message.contains("above ceiling"));

    // Starvation: slow_upstream (5) < range [30, 300] → error
    let starvation: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.probe_id.contains("starvation_check"))
        .collect();
    assert_eq!(starvation.len(), 1);
    assert_eq!(starvation[0].level, DiagnosticLevel::Error);
    assert!(starvation[0].message.contains("out of range"));
}

#[test]
fn test_ratefield_confidence_weighting() {
    // Verify confidence weighting reduces effective contribution
    let input = r#"
version: "1.0"
name: "Confidence Test"

fields:
  - name: "flow"
    type: rate

activities:
  - id: "low_confidence_source"
    timespan:
      start: "2026-01-05"
      end: "2026-01-15"
    projections:
      - field: "flow"
        geometry: "zone"
        value: 100.0
        confidence: 0.1
    probes:
      - field: "flow"
        geometry: "zone"
        gte: 50.0

geometries:
  - id: "zone"
    cuboid:
      half_extents: [5.0, 5.0, 5.0]
    pose:
      position: [0.0, 0.0, 0.0]
"#;
    let result = run_pipeline(input).unwrap();

    // With 0.1 confidence, effective contribution = 100 * 0.1 = 10
    // Probe requires gte 50, which should fail
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.level >= DiagnosticLevel::Error)
        .collect();
    assert_eq!(errors.len(), 1, "Expected 1 error, got {:?}", errors);
    assert!(errors[0].message.contains("sampled 10.00"));
}

#[test]
fn test_lte_assertion_passes() {
    let input = r#"
version: "1.0"
name: "Lte Pass Test"

fields:
  - name: "flow"
    type: rate

activities:
  - id: "moderate_source"
    timespan:
      start: "2026-01-05"
      end: "2026-01-15"
    projections:
      - field: "flow"
        geometry: "zone"
        value: 50.0
    probes:
      - field: "flow"
        geometry: "zone"
        lte: 100.0

geometries:
  - id: "zone"
    cuboid:
      half_extents: [5, 5, 5]
    pose:
      position: [0, 0, 0]
"#;
    let result = run_pipeline(input).unwrap();
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.level >= DiagnosticLevel::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "Lte 100 should pass with rate 50, got {:?}",
        errors
    );
}

#[test]
fn test_lte_assertion_fails() {
    let input = r#"
version: "1.0"
name: "Lte Fail Test"

fields:
  - name: "flow"
    type: rate

activities:
  - id: "fast_source"
    timespan:
      start: "2026-01-05"
      end: "2026-01-15"
    projections:
      - field: "flow"
        geometry: "zone"
        value: 200.0
    probes:
      - field: "flow"
        geometry: "zone"
        lte: 100.0

geometries:
  - id: "zone"
    cuboid:
      half_extents: [5, 5, 5]
    pose:
      position: [0, 0, 0]
"#;
    let result = run_pipeline(input).unwrap();
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.level >= DiagnosticLevel::Error)
        .collect();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("above ceiling"));
    assert!(errors[0].message.contains("200.00"));
}

#[test]
fn test_range_assertion_passes() {
    let input = r#"
version: "1.0"
name: "Range Pass Test"

fields:
  - name: "flow"
    type: rate

activities:
  - id: "source"
    timespan:
      start: "2026-01-05"
      end: "2026-01-15"
    projections:
      - field: "flow"
        geometry: "zone"
        value: 50.0
    probes:
      - field: "flow"
        geometry: "zone"
        min: 30.0
        max: 100.0

geometries:
  - id: "zone"
    cuboid:
      half_extents: [5, 5, 5]
    pose:
      position: [0, 0, 0]
"#;
    let result = run_pipeline(input).unwrap();
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.level >= DiagnosticLevel::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "Range [30,100] should pass with rate 50, got {:?}",
        errors
    );
}

#[test]
fn test_range_assertion_fails_too_low() {
    let input = r#"
version: "1.0"
name: "Range Fail Low Test"

fields:
  - name: "flow"
    type: rate

activities:
  - id: "source"
    timespan:
      start: "2026-01-05"
      end: "2026-01-15"
    projections:
      - field: "flow"
        geometry: "zone"
        value: 5.0
    probes:
      - field: "flow"
        geometry: "zone"
        min: 30.0
        max: 100.0

geometries:
  - id: "zone"
    cuboid:
      half_extents: [5, 5, 5]
    pose:
      position: [0, 0, 0]
"#;
    let result = run_pipeline(input).unwrap();
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.level >= DiagnosticLevel::Error)
        .collect();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("out of range"));
    assert!(errors[0].message.contains("5.00"));
}
