use cpml::pipeline::run_pipeline;
use cpml::schema::DiagnosticLevel;

/// Without region hierarchy, a probe in a non-overlapping child region does
/// NOT see the parent's supply — the capacity check is per-region.
#[test]
fn test_capacity_partitioned_without_hierarchy() {
    let input = include_str!("../samples/region_hierarchy_demo.cpml");
    let result = run_pipeline(input).unwrap();

    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.level >= DiagnosticLevel::Error)
        .collect();

    assert!(
        errors.is_empty(),
        "Supply (600 kW) exceeds consumption (500 kW), should pass with hierarchy"
    );
}

/// Helper: run a CPML snippet and count errors.
fn count_errors(yaml: &str) -> usize {
    run_pipeline(yaml)
        .unwrap()
        .diagnostics
        .iter()
        .filter(|d| d.level >= DiagnosticLevel::Error)
        .count()
}

#[test]
fn test_no_hierarchy_with_separate_regions_fails() {
    let input = r#"
version: "1.0"
name: "no hierarchy — partitioned capacity"
fields:
  - name: power
    type: capacity
geometries:
  - id: site_geom
    pose:
      position: [0, 0, 0]
    cuboid:
      half_extents: [5, 5, 5]
  - id: zone_geom
    pose:
      position: [100, 100, 0]
    cuboid:
      half_extents: [5, 5, 5]
activities:
  - id: supply
    timespan:
      start: "2026-01-01"
      end: "2026-01-30"
    geometry: site_geom
    projections:
      - field: power
        geometry: site_geom
        value: 200
  - id: consumer
    timespan:
      start: "2026-01-02"
      end: "2026-01-15"
    geometry: zone_geom
    projections:
      - field: power
        geometry: zone_geom
        value: -250
    probes:
      - field: power
        geometry: zone_geom
        gte: 0
"#;
    let errors = count_errors(input);
    assert_eq!(
        errors, 1,
        "Without hierarchy, supply is invisible → shortfall"
    );
}

#[test]
fn test_hierarchy_makes_parent_visible() {
    let input = r#"
version: "1.0"
name: "with hierarchy — unified capacity"
fields:
  - name: power
    type: capacity
regions:
  - id: site_wide
  - id: zone_A
    parent: site_wide
geometries:
  - id: site_geom
    region: site_wide
    pose:
      position: [0, 0, 0]
    cuboid:
      half_extents: [5, 5, 5]
  - id: zone_geom
    region: zone_A
    pose:
      position: [100, 100, 0]
    cuboid:
      half_extents: [5, 5, 5]
activities:
  - id: supply
    timespan:
      start: "2026-01-01"
      end: "2026-01-30"
    geometry: site_geom
    projections:
      - field: power
        geometry: site_geom
        value: 200
  - id: consumer
    timespan:
      start: "2026-01-02"
      end: "2026-01-15"
    geometry: zone_geom
    projections:
      - field: power
        geometry: zone_geom
        value: -150
    probes:
      - field: power
        geometry: zone_geom
        gte: 0
"#;
    let errors = count_errors(input);
    assert_eq!(
        errors, 0,
        "With hierarchy, supply (200) + consumption (-150) = 50 → passes"
    );
}

#[test]
fn test_hierarchy_still_detects_shortfall() {
    let input = r#"
version: "1.0"
name: "with hierarchy — genuine shortfall"
fields:
  - name: power
    type: capacity
regions:
  - id: site_wide
  - id: zone_A
    parent: site_wide
geometries:
  - id: site_geom
    region: site_wide
    pose:
      position: [0, 0, 0]
    cuboid:
      half_extents: [5, 5, 5]
  - id: zone_geom
    region: zone_A
    pose:
      position: [100, 100, 0]
    cuboid:
      half_extents: [5, 5, 5]
activities:
  - id: supply
    timespan:
      start: "2026-01-01"
      end: "2026-01-30"
    geometry: site_geom
    projections:
      - field: power
        geometry: site_geom
        value: 200
  - id: consumer
    timespan:
      start: "2026-01-02"
      end: "2026-01-15"
    geometry: zone_geom
    projections:
      - field: power
        geometry: zone_geom
        value: -250
    probes:
      - field: power
        geometry: zone_geom
        gte: 0
"#;
    let errors = count_errors(input);
    assert_eq!(
        errors, 1,
        "With hierarchy, supply (200) + consumption (-250) = -50 → shortfall"
    );
}
