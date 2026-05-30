use cpml::comparison::compare_results;
use cpml::pipeline::run_pipeline;

#[test]
fn test_compare_identical() {
    let input = r#"
version: "1.0"
name: "Test Scenario"

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
  - id: "gen"
    timespan:
      start: "2026-01-01"
      end: "2026-01-10"
    projections:
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
        gte: 50.0
"#;

    let result_a = run_pipeline(input).unwrap();
    let result_b = run_pipeline(input).unwrap();
    let summary = compare_results(&result_a, &result_b);

    assert_eq!(summary.schedule_delta_days, 0);
    assert_eq!(summary.risk_delta, 0.0);
    assert_eq!(summary.cost_delta, 0.0);
    assert_eq!(summary.diag_count_a, summary.diag_count_b);
    assert!(summary.unique_to_a.is_empty());
    assert!(summary.unique_to_b.is_empty());
}

#[test]
fn test_compare_different_schedules() {
    let input_a = r#"
version: "1.0"
name: "Shorter Schedule"

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
  - id: "task"
    timespan:
      start: "2026-01-01"
      end: "2026-01-05"
    probes:
      - field: "power"
        geometry: "zone"
        gte: 0.0
"#;

    let input_b = r#"
version: "1.0"
name: "Longer Schedule"

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
  - id: "task"
    timespan:
      start: "2026-01-01"
      end: "2026-01-20"
    probes:
      - field: "power"
        geometry: "zone"
        gte: 0.0
"#;

    let result_a = run_pipeline(input_a).unwrap();
    let result_b = run_pipeline(input_b).unwrap();
    let summary = compare_results(&result_a, &result_b);

    assert_eq!(summary.schedule_days_a, 4);
    assert_eq!(summary.schedule_days_b, 19);
    assert_eq!(summary.schedule_delta_days, -15); // A is 15 days shorter
}

#[test]
fn test_compare_unique_diagnostics() {
    let input_a = r#"
version: "1.0"
name: "With Error"

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
  - id: "consumer"
    timespan:
      start: "2026-01-01"
      end: "2026-01-10"
    probes:
      - field: "power"
        geometry: "zone"
        gte: 100.0
"#;

    let input_b = r#"
version: "1.0"
name: "Clean"

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
  - id: "gen"
    timespan:
      start: "2026-01-01"
      end: "2026-01-10"
    projections:
      - field: "power"
        geometry: "zone"
        value: 200.0
  - id: "consumer"
    timespan:
      start: "2026-01-01"
      end: "2026-01-10"
    probes:
      - field: "power"
        geometry: "zone"
        gte: 100.0
"#;

    let result_a = run_pipeline(input_a).unwrap();
    let result_b = run_pipeline(input_b).unwrap();
    let summary = compare_results(&result_a, &result_b);

    // A has 0 power → fails gte 100. B has 200 → passes.
    assert!(
        !summary.unique_to_a.is_empty(),
        "A should have unique errors"
    );
    assert!(
        summary.unique_to_b.is_empty(),
        "B should have no unique errors"
    );
    assert!(summary.risk_delta > 0.0, "A should be riskier");
}
