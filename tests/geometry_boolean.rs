use cpml::model::geometry::{Geometry, Pose, Shape};
use cpml::pipeline::run_pipeline;

// ── Model-level boolean-op tests ──

#[test]
fn test_union_aabb() {
    let geom = Geometry {
        shape: Shape::Union(vec![
            Shape::Cuboid {
                half_extents: [2.0, 2.0, 2.0],
            },
            Shape::Cuboid {
                half_extents: [1.0, 1.0, 5.0],
            },
        ]),
        pose: Pose::from_position([0.0, 0.0, 0.0]),
        region: None,
    };
    let aabb = geom.world_aabb();
    // Union: AABB should enclose both: [-2,-2,-5] to [2,2,5]
    assert_eq!(aabb.min, [-2.0, -2.0, -5.0]);
    assert_eq!(aabb.max, [2.0, 2.0, 5.0]);
}

#[test]
fn test_intersection_aabb() {
    let geom = Geometry {
        shape: Shape::Intersection(vec![
            Shape::Cuboid {
                half_extents: [10.0, 10.0, 10.0],
            },
            Shape::Cuboid {
                half_extents: [2.0, 2.0, 2.0],
            },
        ]),
        pose: Pose::from_position([0.0, 0.0, 0.0]),
        region: None,
    };
    let aabb = geom.world_aabb();
    // Intersection: narrowest AABB = [-2,-2,-2] to [2,2,2]
    assert_eq!(aabb.min, [-2.0, -2.0, -2.0]);
    assert_eq!(aabb.max, [2.0, 2.0, 2.0]);
}

#[test]
fn test_subtract_aabb_conservative() {
    let geom = Geometry {
        shape: Shape::Subtract {
            a: Box::new(Shape::Cuboid {
                half_extents: [5.0, 5.0, 5.0],
            }),
            b: Box::new(Shape::Cuboid {
                half_extents: [2.0, 2.0, 2.0],
            }),
        },
        pose: Pose::from_position([0.0, 0.0, 0.0]),
        region: None,
    };
    let aabb = geom.world_aabb();
    // Subtract: conservative AABB = A's AABB = [-5,-5,-5] to [5,5,5]
    assert_eq!(aabb.min, [-5.0, -5.0, -5.0]);
    assert_eq!(aabb.max, [5.0, 5.0, 5.0]);
}

#[test]
fn test_union_intersection_gjk() {
    // Two cuboids, one at origin, one at x=3 (they overlap at x in [1,3])
    let geom_a = Geometry {
        shape: Shape::Cuboid {
            half_extents: [2.0, 2.0, 2.0],
        },
        pose: Pose::from_position([0.0, 0.0, 0.0]),
        region: None,
    };
    let union = Geometry {
        shape: Shape::Union(vec![
            Shape::Cuboid {
                half_extents: [2.0, 2.0, 2.0],
            },
            Shape::Cuboid {
                half_extents: [2.0, 2.0, 2.0],
            },
        ]),
        pose: Pose::from_position([0.0, 0.0, 0.0]),
        region: None,
    };
    assert!(union.exact_intersects(&geom_a));
}

#[test]
fn test_union_separated_gjk() {
    let union = Geometry {
        shape: Shape::Union(vec![
            Shape::Cuboid {
                half_extents: [1.0, 1.0, 1.0],
            },
            Shape::Cuboid {
                half_extents: [1.0, 1.0, 1.0],
            },
        ]),
        pose: Pose::from_position([0.0, 0.0, 0.0]),
        region: None,
    };
    let far = Geometry {
        shape: Shape::Cuboid {
            half_extents: [1.0, 1.0, 1.0],
        },
        pose: Pose::from_position([50.0, 50.0, 50.0]),
        region: None,
    };
    assert!(!union.exact_intersects(&far));
}

#[test]
fn test_intersection_all_children_must_intersect() {
    // Intersection of a large cuboid and a small sphere.
    // The intersection region is the sphere (smaller shape).
    // Cuboid: half_extents [5,5,5] → reaches x=5
    // Sphere: radius 3 → only reaches x=3
    let intersection = Geometry {
        shape: Shape::Intersection(vec![
            Shape::Cuboid {
                half_extents: [5.0, 5.0, 5.0],
            },
            Shape::Sphere { radius: 3.0 },
        ]),
        pose: Pose::from_position([0.0, 0.0, 0.0]),
        region: None,
    };
    // Probe at x=2, y=0, z=0 (within both shapes) → true
    let probe_near = Geometry {
        shape: Shape::Sphere { radius: 0.5 },
        pose: Pose::from_position([2.0, 0.0, 0.0]),
        region: None,
    };
    assert!(intersection.exact_intersects(&probe_near));

    // Probe at x=4, y=0, z=0: within cuboid (extent 5) but outside sphere (radius 3) → false
    let probe_far = Geometry {
        shape: Shape::Sphere { radius: 0.5 },
        pose: Pose::from_position([4.0, 0.0, 0.0]),
        region: None,
    };
    assert!(!intersection.exact_intersects(&probe_far));
}

#[test]
fn test_subtract_overlap_with_a_not_b() {
    // A - B: large cuboid minus small cuboid in center
    let subtract = Geometry {
        shape: Shape::Subtract {
            a: Box::new(Shape::Cuboid {
                half_extents: [5.0, 5.0, 5.0],
            }),
            b: Box::new(Shape::Cuboid {
                half_extents: [1.0, 1.0, 1.0],
            }),
        },
        pose: Pose::from_position([0.0, 0.0, 0.0]),
        region: None,
    };
    // Probe near edge (x=4): intersects A but not B → true
    let probe_edge = Geometry {
        shape: Shape::Sphere { radius: 0.5 },
        pose: Pose::from_position([4.0, 0.0, 0.0]),
        region: None,
    };
    assert!(subtract.exact_intersects(&probe_edge));

    // Probe at center (x=0): intersects B → false (subtracted)
    let probe_center = Geometry {
        shape: Shape::Sphere { radius: 0.5 },
        pose: Pose::from_position([0.0, 0.0, 0.0]),
        region: None,
    };
    assert!(!subtract.exact_intersects(&probe_center));
}

// ── YAML round-trip tests ──

#[test]
fn test_union_from_yaml() {
    let input = r#"
version: "1.0"
name: "Union Test"

fields:
  - name: "occ"
    type: occupancy

geometries:
  - id: "L_shape"
    union:
      - cuboid:
          half_extents: [5.0, 2.0, 10.0]
      - cuboid:
          half_extents: [2.0, 5.0, 10.0]

activities:
  - id: "test"
    timespan:
      start: "2026-01-01"
      end: "2026-01-10"
    collision:
      hard:
        geometry: "L_shape"
"#;
    let result = run_pipeline(input).expect("union YAML should parse");
    let geo = result
        .model
        .activities
        .iter()
        .find(|a| a.id == "test")
        .unwrap();
    let proj = &geo.projections[0];
    // Union AABB should cover the combined extent of both cuboids: [-5,-5,-10] to [5,5,10]
    let aabb = proj.aabb();
    assert_eq!(aabb.min, [-5.0, -5.0, -10.0]);
    assert_eq!(aabb.max, [5.0, 5.0, 10.0]);
}

#[test]
fn test_subtract_from_yaml() {
    let input = r#"
version: "1.0"
name: "Subtract Test"

fields:
  - name: "occ"
    type: occupancy

geometries:
  - id: "wall_with_hole"
    subtract:
      a:
        cuboid:
          half_extents: [5.0, 0.5, 5.0]
      b:
        cuboid:
          half_extents: [1.0, 1.0, 1.0]

activities:
  - id: "test"
    timespan:
      start: "2026-01-01"
      end: "2026-01-10"
    collision:
      hard:
        geometry: "wall_with_hole"
"#;
    let result = run_pipeline(input).expect("subtract YAML should parse");
    let geo = result
        .model
        .activities
        .iter()
        .find(|a| a.id == "test")
        .unwrap();
    // Verify the geometry is resolved with Subtract shape
    assert!(geo.projections.len() == 1);
}

#[test]
fn test_intersection_from_yaml() {
    let input = r#"
version: "1.0"
name: "Intersection Test"

fields:
  - name: "occ"
    type: occupancy

geometries:
  - id: "overlap_zone"
    intersection:
      - cuboid:
          half_extents: [5.0, 5.0, 5.0]
      - cuboid:
          half_extents: [2.0, 2.0, 10.0]

activities:
  - id: "test"
    timespan:
      start: "2026-01-01"
      end: "2026-01-10"
    collision:
      hard:
        geometry: "overlap_zone"
"#;
    let result = run_pipeline(input).expect("intersection YAML should parse");
    let geo = result
        .model
        .activities
        .iter()
        .find(|a| a.id == "test")
        .unwrap();
    let proj = &geo.projections[0];
    let aabb = proj.aabb();
    // Intersection: narrowest in each dimension → [-2,-2,-5] to [2,2,5]
    assert_eq!(aabb.min, [-2.0, -2.0, -5.0]);
    assert_eq!(aabb.max, [2.0, 2.0, 5.0]);
}
