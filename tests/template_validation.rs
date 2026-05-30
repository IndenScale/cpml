use cpml::pipeline::run_pipeline;

/// Instantiate the welding_arc template with sample values and verify it compiles.
#[test]
fn test_welding_arc_template_instantiates() {
    // Template expanded with concrete values — fire_load, combustible_gas,
    // occupancy, certified_welders, and height_access_equipment fields required.
    let yaml = r#"
version: "1.0"
name: "Welding Arc Template Validation"

fields:
  - name: "fire_load"
    type: scalar
    operator: max
  - name: "combustible_gas"
    type: scalar
    operator: max
  - name: "occupancy"
    type: occupancy
  - name: "certified_welders"
    type: capacity
  - name: "height_access_equipment"
    type: capacity

geometries:
  - id: "weld_point_A"
    pose: { position: [10, 10, 8] }
    sphere: { radius: 0.5 }
  - id: "work_zone_A"
    pose: { position: [10, 10, 8] }
    cuboid: { half_extents: [3, 3, 3] }
  - id: "fall_zone_A"
    pose: { position: [10, 10, 4] }
    sphere: { radius: 3.0 }

activities:
  # Provision welders and equipment
  - id: "resource_provision"
    timespan: { start: "2026-06-01", end: "2026-06-10" }
    projections:
      - field: "certified_welders"
        geometry: "work_zone_A"
        value: 5
      - field: "height_access_equipment"
        geometry: "work_zone_A"
        value: 3

  # --- welding_arc template instantiation ---
  - id: "steel_weld_beam_A1"
    name: "钢结构电弧焊接"
    timespan:
      start: "2026-06-05T08:00"
      end: "2026-06-05T17:00"
    collision:
      hard:
        geometry: "work_zone_A"
    demands:
      - field: "certified_welders"
        geometry: "work_zone_A"
        amount: 2
      - field: "height_access_equipment"
        geometry: "work_zone_A"
        amount: 1
    projections:
      - name: "arc_fire_source"
        field: "fire_load"
        geometry: "weld_point_A"
        value: 0.6
      - name: "slag_fire_source"
        field: "fire_load"
        geometry: "work_zone_A"
        value: 0.4
      - name: "fall_risk_zone"
        field: "occupancy"
        geometry: "fall_zone_A"
        kind: soft
    probes:
      - name: "combustible_gas_check"
        field: "combustible_gas"
        geometry: "work_zone_A"
        lte: 0.3
        diagnostic_level: warning
      - name: "clearance_check"
        field: "occupancy"
        geometry: "work_zone_A"
        empty: true
        diagnostic_level: error
    structures:
      - name: "height_work_reminder"
        field: "occupancy"
        geometry: "fall_zone_A"
        kind: soft
        diagnostic_level: info
"#;

    let result = run_pipeline(yaml).expect("welding_arc template should compile");

    // Verify the activity was expanded with all probes and projections
    let weld = result
        .model
        .activities
        .iter()
        .find(|a| a.id == "steel_weld_beam_A1")
        .expect("welding activity should exist");

    // Should have: combustible_gas_check, clearance_check, struct_height_work_reminder_probe,
    // demand_0_probe, demand_1_probe = 5 probes
    assert!(
        weld.probes.len() >= 5,
        "Should have at least 5 probes, got {}",
        weld.probes.len()
    );

    // Should have: arc_fire_source, slag_fire_source, fall_risk_zone,
    // struct_height_work_reminder_projection, demand_0_projection, demand_1_projection = 6 projections
    assert!(
        weld.projections.len() >= 6,
        "Should have at least 6 projections, got {}",
        weld.projections.len()
    );

    // Info-level structure should NOT generate ERROR diagnostics
    let info_diags: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.probe_id.contains("height_work_reminder"))
        .collect();
    for d in &info_diags {
        assert!(
            d.level <= cpml::schema::DiagnosticLevel::Info
                || d.level == cpml::schema::DiagnosticLevel::Info,
            "Height work reminder should be info-level, not {:?}",
            d.level
        );
    }
}

/// Verify that wolf_arc template's combustible_gas check fires when
/// a spray coating activity runs nearby.
#[test]
fn test_welding_arc_detects_combustible_gas() {
    let yaml = r#"
version: "1.0"
name: "Welding vs Spray Coating"

fields:
  - name: "fire_load"
    type: scalar
    operator: max
  - name: "combustible_gas"
    type: scalar
    operator: max
  - name: "occupancy"
    type: occupancy
  - name: "certified_welders"
    type: capacity
  - name: "height_access_equipment"
    type: capacity

geometries:
  - id: "weld_zone"
    pose: { position: [5, 5, 8] }
    cuboid: { half_extents: [3, 3, 3] }
  - id: "weld_point"
    pose: { position: [5, 5, 8] }
    sphere: { radius: 0.5 }
  - id: "fall_zone"
    pose: { position: [5, 5, 4] }
    sphere: { radius: 3.0 }
  - id: "spray_zone"
    pose: { position: [8, 8, 8] }
    cuboid: { half_extents: [3, 3, 3] }

activities:
  - id: "provision"
    timespan: { start: "2026-06-01", end: "2026-06-10" }
    projections:
      - field: "certified_welders"
        geometry: "weld_zone"
        value: 5
      - field: "height_access_equipment"
        geometry: "weld_zone"
        value: 3

  # Spray coating generates combustible gas
  - id: "spray_coating"
    timespan: { start: "2026-06-05", end: "2026-06-07" }
    collision:
      hard: { geometry: "spray_zone" }
    projections:
      - field: "combustible_gas"
        geometry: "spray_zone"
        value: 0.8

  # Welding arc — same time, nearby location
  - id: "welding"
    timespan: { start: "2026-06-05", end: "2026-06-07" }
    collision:
      hard: { geometry: "weld_zone" }
    demands:
      - field: "certified_welders"
        geometry: "weld_zone"
        amount: 2
      - field: "height_access_equipment"
        geometry: "weld_zone"
        amount: 1
    projections:
      - field: "fire_load"
        geometry: "weld_point"
        value: 0.6
      - field: "fire_load"
        geometry: "weld_zone"
        value: 0.4
      - field: "occupancy"
        geometry: "fall_zone"
        kind: soft
    probes:
      - name: "combustible_gas_check"
        field: "combustible_gas"
        geometry: "weld_zone"
        lte: 0.3
        diagnostic_level: warning
    structures:
      - field: "occupancy"
        geometry: "fall_zone"
        kind: soft
        diagnostic_level: info
"#;

    let result = run_pipeline(yaml).expect("should compile");

    // combustible_gas at weld_zone: spray_coating injects 0.8
    // Weld checks lte: 0.3 → 0.8 > 0.3 → should trigger warning
    let gas_warnings: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("combustible_gas"))
        .collect();
    assert!(
        !gas_warnings.is_empty(),
        "Combustible gas from spray coating should trigger welding probe warning"
    );
    assert!(
        gas_warnings[0].message.contains("above ceiling")
            || gas_warnings[0].message.contains("0.80"),
        "Should report gas level above ceiling"
    );
}
