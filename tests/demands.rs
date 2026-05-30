use cpml::pipeline::run_pipeline;
use cpml::schema::DiagnosticLevel;

#[test]
fn test_demands_generates_probes_and_projections() {
    let input = include_str!("../samples/demands_demo.cpml");
    let result = run_pipeline(input).expect("demands demo should run");

    // concrete_pour_A should have generated probes from demands
    let pour_a = result
        .model
        .activities
        .iter()
        .find(|a| a.id == "concrete_pour_A")
        .expect("concrete_pour_A should exist");

    let demand_probes: Vec<_> = pour_a
        .probes
        .iter()
        .filter(|p| p.id.contains("demand_"))
        .collect();
    assert_eq!(
        demand_probes.len(),
        2,
        "Should have 2 demand probes for power and water"
    );

    let demand_projections: Vec<_> = pour_a
        .projections
        .iter()
        .filter(|p| p.id.contains("demand_"))
        .collect();
    assert_eq!(
        demand_projections.len(),
        2,
        "Should have 2 demand projections"
    );
}

#[test]
fn test_demands_power_shortfall_detected() {
    let input = include_str!("../samples/demands_demo.cpml");
    let result = run_pipeline(input).expect("demands demo should run");

    // Total power: 500 - 150 - 200 - 50 = 100, which is < 150 and < 200
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.level >= DiagnosticLevel::Error && d.message.contains("power_supply"))
        .collect();
    assert_eq!(errors.len(), 2, "Should have 2 power shortfall errors");

    for e in &errors {
        assert!(e.message.contains("power_supply"));
        assert!(e.message.contains("below threshold"));
    }
}

#[test]
fn test_demands_custom_diagnostic_level() {
    let input = include_str!("../samples/demands_demo.cpml");
    let result = run_pipeline(input).expect("demands demo should run");

    let night_shift = result
        .model
        .activities
        .iter()
        .find(|a| a.id == "night_shift_work")
        .expect("night_shift_work should exist");

    let demand_probe = night_shift
        .probes
        .iter()
        .find(|p| p.id.contains("demand_"))
        .expect("night_shift_work should have a demand probe");

    assert_eq!(
        demand_probe.diagnostic_level,
        DiagnosticLevel::Warning,
        "Custom diagnostic_level should be Warning"
    );
}

#[test]
fn test_demands_water_usage_not_failing() {
    let input = include_str!("../samples/demands_demo.cpml");
    let result = run_pipeline(input).expect("demands demo should run");

    // Water: 100 - 30 - 40 = 30, which is >= 30 for pour_A, but < 40 for pour_B
    let water_errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("water_supply") && d.level >= DiagnosticLevel::Error)
        .collect();
    assert_eq!(
        water_errors.len(),
        1,
        "pour_B needs 40 water but only 30 remains"
    );
    assert!(water_errors[0].probe_id.contains("concrete_pour_B"));
}
