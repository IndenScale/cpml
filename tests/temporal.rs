use cpml::pipeline::run_pipeline;

#[test]
fn test_date_only_backward_compat() {
    // Date-only format should still work (midnight default)
    let input = r#"
version: "1.0"
name: "Backward Compat Test"

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
      end: "2026-01-10"
    probes:
      - field: "power"
        geometry: "zone"
        gte: 0.0
"#;
    let result = run_pipeline(input).expect("date-only format should still work");
    assert_eq!(result.schedule_duration, 9);
}

#[test]
fn test_datetime_with_time() {
    // Full datetime format
    let input = r#"
version: "1.0"
name: "DateTime Test"

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
  - id: "morning_task"
    timespan:
      start: "2026-01-01T08:00:00"
      end: "2026-01-01T17:00:00"
    probes:
      - field: "power"
        geometry: "zone"
        gte: 0.0

  - id: "afternoon_task"
    timespan:
      start: "2026-01-01T13:00:00"
      end: "2026-01-01T14:00:00"
    probes:
      - field: "power"
        geometry: "zone"
        gte: 0.0
"#;
    let result = run_pipeline(input).expect("datetime format should work");
    // Both tasks within same day but different hours → 3 keyframes
    assert!(result.diagnostics.is_empty());
    // Verify keyframes include time boundaries
    assert!(!result.metrics.is_empty());
}

#[test]
fn test_subday_half_open() {
    let input = r#"
version: "1.0"
name: "Subday Half-Open Test"

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
  - id: "morning_only"
    timespan:
      start: "2026-01-05T08:00:00"
      end: "2026-01-05T12:00:00"
    projections:
      - field: "power"
        geometry: "zone"
        value: 100.0

  - id: "afternoon_check"
    timespan:
      start: "2026-01-05T14:00:00"
      end: "2026-01-05T18:00:00"
    probes:
      - field: "power"
        geometry: "zone"
        gte: 50.0
"#;
    let result = run_pipeline(input).expect("subday should work");
    // morning_only ends at 12:00, afternoon_check starts at 14:00.
    // Afternoon check should see 0 power (morning projection is inactive).
    // Probe gte 50.0 should fail.
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.activity_id == "afternoon_check")
        .collect();
    // No power projection active → 0 < 50 → should fail
    assert!(
        !errors.is_empty(),
        "Expected afternoon_check to fail (0 < 50)"
    );
}

#[test]
fn test_mixed_date_datetime_formats() {
    // Mix date-only and datetime formats in same document
    let input = r#"
version: "1.0"
name: "Mixed Format Test"

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
  - id: "day_task"
    timespan:
      start: "2026-01-01"
      end: "2026-01-10"
    projections:
      - field: "power"
        geometry: "zone"
        value: 100.0

  - id: "hour_task"
    timespan:
      start: "2026-01-05T10:00:00"
      end: "2026-01-05T11:00:00"
    probes:
      - field: "power"
        geometry: "zone"
        gte: 50.0
"#;
    let result = run_pipeline(input).expect("mixed formats should work");
    // hour_task runs during day_task → sees 100 >= 50 → passes
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.activity_id == "hour_task")
        .collect();
    assert!(
        errors.is_empty(),
        "hour_task should see power from day_task and pass"
    );
}
