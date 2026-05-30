use cpml::pipeline::run_pipeline;

/// Verify that a structure with confidence: 0.5 produces a projection
/// with the confidence value, and that the weighted contribution affects
/// the sampled field value.
#[test]
fn test_structure_confidence_weights_capacity() {
    let input = r#"
version: "1.0"
name: "Structure Confidence Test"

fields:
  - name: "power"
    type: capacity

geometries:
  - id: "zone"
    cuboid:
      half_extents: [5, 5, 5]
    pose:
      position: [0, 0, 0]

activities:
  - id: "generator_full"
    timespan:
      start: "2026-01-01"
      end: "2026-01-10"
    structures:
      - field: "power"
        geometry: "zone"
        value: 100.0
        confidence: 1.0

  - id: "generator_low_confidence"
    timespan:
      start: "2026-01-01"
      end: "2026-01-10"
    structures:
      - field: "power"
        geometry: "zone"
        value: 100.0
        confidence: 0.2

  - id: "consumer"
    timespan:
      start: "2026-01-01"
      end: "2026-01-10"
    probes:
      - field: "power"
        geometry: "zone"
        gte: 150.0
"#;

    let result = run_pipeline(input).expect("pipeline should succeed");

    // Conf 1.0 + Conf 0.2 → 100*1.0 + 100*0.2 = 120 < 150 → FAIL
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.probe_id.contains("consumer"))
        .collect();
    assert!(!errors.is_empty(), "Expected probe to fail (120 < 150)");

    // Verify the generated projection has confidence
    let full_gen = result
        .model
        .activities
        .iter()
        .find(|a| a.id == "generator_full")
        .unwrap();
    let full_proj = full_gen
        .projections
        .iter()
        .find(|p| p.name.as_deref() == Some("struct_structure_0_projection"))
        .unwrap();
    assert_eq!(full_proj.confidence, Some(1.0));

    let low_gen = result
        .model
        .activities
        .iter()
        .find(|a| a.id == "generator_low_confidence")
        .unwrap();
    let low_proj = low_gen
        .projections
        .iter()
        .find(|p| p.name.as_deref() == Some("struct_structure_0_projection"))
        .unwrap();
    assert_eq!(low_proj.confidence, Some(0.2));
}

/// Verify that structure without explicit confidence defaults to None (= full 1.0 weight).
#[test]
fn test_structure_no_confidence_defaults_to_full() {
    let input = r#"
version: "1.0"
name: "Default Confidence Test"

fields:
  - name: "power"
    type: capacity

geometries:
  - id: "zone"
    cuboid:
      half_extents: [5, 5, 5]
    pose:
      position: [0, 0, 0]

activities:
  - id: "generator"
    timespan:
      start: "2026-01-01"
      end: "2026-01-10"
    structures:
      - field: "power"
        geometry: "zone"
        value: 100.0

  - id: "consumer"
    timespan:
      start: "2026-01-01"
      end: "2026-01-10"
    probes:
      - field: "power"
        geometry: "zone"
        gte: 90.0
"#;

    let result = run_pipeline(input).expect("pipeline should succeed");

    // No explicit confidence → defaults to None → treated as 1.0
    // 100 * 1.0 = 100 >= 90 → PASS
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.probe_id.contains("consumer"))
        .collect();
    assert!(errors.is_empty(), "Expected probe to pass (100 >= 90)");

    let gen = result
        .model
        .activities
        .iter()
        .find(|a| a.id == "generator")
        .unwrap();
    let proj = gen
        .projections
        .iter()
        .find(|p| p.name.as_deref() == Some("struct_structure_0_projection"))
        .unwrap();
    assert_eq!(proj.confidence, None);
}
