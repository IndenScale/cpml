use cpml::pipeline::run_pipeline;
use cpml::schema::DiagnosticLevel;

#[test]
fn test_fs_dependency_passes() {
    let input = r#"
version: "1.0"
name: "FS Pass Test"

fields:
  - name: "power"
    type: capacity

geometries:
  - id: "site"
    cuboid:
      half_extents: [10, 10, 10]
    pose:
      position: [0, 0, 0]

activities:
  - id: "pour_concrete"
    timespan:
      start: "2026-01-01"
      end: "2026-01-05"
    projections:
      - field: "power"
        geometry: "site"
        value: 100.0

  - id: "install_equipment"
    timespan:
      start: "2026-01-06"
      end: "2026-01-10"
    depends_on:
      - activity_id: "pour_concrete"
        kind: FS
    probes:
      - field: "power"
        geometry: "site"
        gte: 50.0
"#;
    let result = run_pipeline(input).expect("pipeline should succeed");
    let dep_errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.probe_id.contains("dep_"))
        .collect();
    assert!(
        dep_errors.is_empty(),
        "Expected no dependency errors, got {:?}",
        dep_errors
    );
}

#[test]
fn test_fs_dependency_violated() {
    let input = r#"
version: "1.0"
name: "FS Violation Test"

fields:
  - name: "power"
    type: capacity

geometries:
  - id: "site"
    cuboid:
      half_extents: [10, 10, 10]
    pose:
      position: [0, 0, 0]

activities:
  - id: "pour_concrete"
    timespan:
      start: "2026-01-01"
      end: "2026-01-10"

  - id: "install_equipment"
    timespan:
      start: "2026-01-05"
      end: "2026-01-15"
    depends_on:
      - activity_id: "pour_concrete"
        kind: FS
    probes:
      - field: "power"
        geometry: "site"
        gte: 0.0
"#;
    let result = run_pipeline(input).expect("pipeline should succeed");
    let dep_errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.probe_id.contains("dep_") && d.level >= DiagnosticLevel::Error)
        .collect();
    assert!(!dep_errors.is_empty(), "Expected FS dependency violation");
    assert!(dep_errors[0].message.contains("FS"));
    assert!(dep_errors[0].message.contains("pour_concrete"));
}

#[test]
fn test_fs_with_lag_passes() {
    let input = r#"
version: "1.0"
name: "FS Lag Pass Test"

fields:
  - name: "power"
    type: capacity

geometries:
  - id: "site"
    cuboid:
      half_extents: [10, 10, 10]
    pose:
      position: [0, 0, 0]

activities:
  - id: "curing"
    timespan:
      start: "2026-01-01"
      end: "2026-01-05"

  - id: "next_phase"
    timespan:
      start: "2026-01-12"
      end: "2026-01-20"
    depends_on:
      - activity_id: "curing"
        kind: FS
        lag_days: 7
    probes:
      - field: "power"
        geometry: "site"
        gte: 0.0
"#;
    let result = run_pipeline(input).expect("pipeline should succeed");
    // curing ends Jan 5 + 7 days lag = Jan 12. next_phase starts Jan 12. OK.
    let dep_errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.probe_id.contains("dep_"))
        .collect();
    assert!(
        dep_errors.is_empty(),
        "FS with 7-day lag should pass, got {:?}",
        dep_errors
    );
}

#[test]
fn test_fs_with_lag_violated() {
    let input = r#"
version: "1.0"
name: "FS Lag Violation Test"

fields:
  - name: "power"
    type: capacity

geometries:
  - id: "site"
    cuboid:
      half_extents: [10, 10, 10]
    pose:
      position: [0, 0, 0]

activities:
  - id: "curing"
    timespan:
      start: "2026-01-01"
      end: "2026-01-05"

  - id: "next_phase"
    timespan:
      start: "2026-01-08"
      end: "2026-01-20"
    depends_on:
      - activity_id: "curing"
        kind: FS
        lag_days: 7
    probes:
      - field: "power"
        geometry: "site"
        gte: 0.0
"#;
    let result = run_pipeline(input).expect("pipeline should succeed");
    // curing ends Jan 5 + 7 days lag = Jan 12. next_phase starts Jan 8 → violated.
    let dep_errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.probe_id.contains("dep_") && d.level >= DiagnosticLevel::Error)
        .collect();
    assert!(
        !dep_errors.is_empty(),
        "FS lag violation should be detected"
    );
}

#[test]
fn test_ss_dependency_violated() {
    let input = r#"
version: "1.0"
name: "SS Violation Test"

fields:
  - name: "power"
    type: capacity

geometries:
  - id: "site"
    cuboid:
      half_extents: [10, 10, 10]
    pose:
      position: [0, 0, 0]

activities:
  - id: "predecessor"
    timespan:
      start: "2026-01-10"
      end: "2026-01-20"

  - id: "successor"
    timespan:
      start: "2026-01-05"
      end: "2026-01-15"
    depends_on:
      - activity_id: "predecessor"
        kind: SS
    probes:
      - field: "power"
        geometry: "site"
        gte: 0.0
"#;
    let result = run_pipeline(input).expect("pipeline should succeed");
    // Successor starts Jan 5 but predecessor only starts Jan 10 → SS violation
    let dep_errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.probe_id.contains("dep_") && d.level >= DiagnosticLevel::Error)
        .collect();
    assert!(!dep_errors.is_empty(), "Expected SS dependency violation");
    assert!(dep_errors[0].message.contains("SS"));
}

#[test]
fn test_ff_dependency_violated() {
    let input = r#"
version: "1.0"
name: "FF Violation Test"

fields:
  - name: "power"
    type: capacity

geometries:
  - id: "site"
    cuboid:
      half_extents: [10, 10, 10]
    pose:
      position: [0, 0, 0]

activities:
  - id: "predecessor"
    timespan:
      start: "2026-01-01"
      end: "2026-01-20"

  - id: "successor"
    timespan:
      start: "2026-01-05"
      end: "2026-01-10"
    depends_on:
      - activity_id: "predecessor"
        kind: FF
    probes:
      - field: "power"
        geometry: "site"
        gte: 0.0
"#;
    let result = run_pipeline(input).expect("pipeline should succeed");
    // Successor ends Jan 10 but predecessor ends Jan 20 → FF violation
    let dep_errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.probe_id.contains("dep_") && d.level >= DiagnosticLevel::Error)
        .collect();
    assert!(!dep_errors.is_empty(), "Expected FF dependency violation");
    assert!(dep_errors[0].message.contains("FF"));
}

#[test]
fn test_self_dependency_rejected() {
    let input = r#"
version: "1.0"
name: "Self Dependency Test"

fields:
  - name: "power"
    type: capacity

geometries:
  - id: "site"
    cuboid:
      half_extents: [10, 10, 10]
    pose:
      position: [0, 0, 0]

activities:
  - id: "self_ref"
    timespan:
      start: "2026-01-01"
      end: "2026-01-10"
    depends_on:
      - activity_id: "self_ref"
        kind: FS
    probes:
      - field: "power"
        geometry: "site"
        gte: 0.0
"#;
    let result = run_pipeline(input);
    assert!(result.is_err(), "Self-dependency should be rejected");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("cannot depend on itself"));
}

#[test]
fn test_unknown_dependency_rejected() {
    let input = r#"
version: "1.0"
name: "Unknown Dependency Test"

fields:
  - name: "power"
    type: capacity

geometries:
  - id: "site"
    cuboid:
      half_extents: [10, 10, 10]
    pose:
      position: [0, 0, 0]

activities:
  - id: "test_activity"
    timespan:
      start: "2026-01-01"
      end: "2026-01-10"
    depends_on:
      - activity_id: "nonexistent"
        kind: FS
    probes:
      - field: "power"
        geometry: "site"
        gte: 0.0
"#;
    let result = run_pipeline(input);
    assert!(result.is_err(), "Unknown dependency should be rejected");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("nonexistent"));
}
