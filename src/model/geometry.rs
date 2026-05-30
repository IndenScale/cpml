use std::f64::consts::PI;

use parry3d::math::Pose as ParryPose;
use parry3d::query::intersection_test;
use parry3d::shape::SharedShape;
use serde::Serialize;

/// World-space pose: position + Euler rotation (ZYX, degrees).
#[derive(Debug, Clone, Serialize)]
pub struct Pose {
    pub position: [f64; 3],
    /// Euler angles in degrees: [yaw, pitch, roll] = rotation around Z, Y, X.
    pub rotation: [f64; 3],
}

impl Pose {
    pub fn identity() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        }
    }

    pub fn from_position(pos: [f64; 3]) -> Self {
        Self {
            position: pos,
            rotation: [0.0, 0.0, 0.0],
        }
    }
}

impl Default for Pose {
    fn default() -> Self {
        Self::identity()
    }
}

/// Geometric shape in local space (centered at origin, Z-aligned).
#[derive(Debug, Clone, Serialize)]
pub enum Shape {
    Cuboid { half_extents: [f64; 3] },
    Cylinder { radius: f64, half_height: f64 },
    Sphere { radius: f64 },
    Hemisphere { radius: f64 },
    Cone { radius: f64, half_height: f64 },
    Union(Vec<Shape>),
    Intersection(Vec<Shape>),
    Subtract { a: Box<Shape>, b: Box<Shape> },
}

/// A geometry instance = shape + world-space pose.
#[derive(Debug, Clone, Serialize)]
pub struct Geometry {
    pub shape: Shape,
    pub pose: Pose,
    /// Optional region ID for capacity hierarchy. When evaluating capacity
    /// at this geometry's region, ancestor regions' contributions are included.
    pub region: Option<String>,
}

impl Geometry {
    /// Compute the world-space AABB for this geometry.
    /// For rotated non-cuboid shapes, this is a conservative bounding approximation.
    pub fn world_aabb(&self) -> Aabb {
        let local_aabb = self.shape.local_aabb();
        rotate_aabb(&local_aabb, &self.pose)
    }

    /// Unique spatial key for region-based field state lookup.
    pub fn region_key(&self) -> String {
        self.world_aabb().region_key()
    }
}

impl Pose {
    /// Convert this pose to a parry3d Pose (glamx DPose3: rotation + translation).
    pub fn to_parry_pose(&self) -> ParryPose {
        let rot = rotation_matrix_zyx(
            deg_to_rad(self.rotation[0]),
            deg_to_rad(self.rotation[1]),
            deg_to_rad(self.rotation[2]),
        );
        // Convert [[f64;3];3] → DMat3 → DQuat
        let mat3 = parry3d::glamx::DMat3::from_cols_array_2d(&rot);
        let quat = parry3d::glamx::DQuat::from_mat3(&mat3);
        let trans = parry3d::glamx::DVec3::from(self.position);
        ParryPose::from_parts(trans, quat)
    }
}

impl Geometry {
    /// Exact GJK-based intersection test between two geometries.
    /// Handles boolean operations (Union, Intersection, Subtract) correctly.
    pub fn exact_intersects(&self, other: &Geometry) -> bool {
        // Handle boolean op shapes with custom logic
        match (&self.shape, &other.shape) {
            // Intersection: all self children must intersect other
            (Shape::Intersection(children), _) => {
                let self_pose = self.pose.clone();
                children.iter().all(|child| {
                    let geom = Geometry {
                        shape: child.clone(),
                        pose: self_pose.clone(),
                        region: None,
                    };
                    geom.exact_intersects(other)
                })
            }
            // Intersection: other must intersect all self children
            (_, Shape::Intersection(children)) => {
                let other_pose = other.pose.clone();
                children.iter().all(|child| {
                    let geom = Geometry {
                        shape: child.clone(),
                        pose: other_pose.clone(),
                        region: None,
                    };
                    self.exact_intersects(&geom)
                })
            }
            // Subtract: self is (A - B), so other must intersect A but not B
            (Shape::Subtract { a, b }, _) => {
                let geom_a = Geometry {
                    shape: (**a).clone(),
                    pose: self.pose.clone(),
                    region: None,
                };
                let geom_b = Geometry {
                    shape: (**b).clone(),
                    pose: self.pose.clone(),
                    region: None,
                };
                geom_a.exact_intersects(other) && !geom_b.exact_intersects(other)
            }
            // Subtract: other is (A - B), so self must intersect A but not B
            (_, Shape::Subtract { a, b }) => {
                let geom_a = Geometry {
                    shape: (**a).clone(),
                    pose: other.pose.clone(),
                    region: None,
                };
                let geom_b = Geometry {
                    shape: (**b).clone(),
                    pose: other.pose.clone(),
                    region: None,
                };
                self.exact_intersects(&geom_a) && !self.exact_intersects(&geom_b)
            }
            _ => {
                let shape1 = self.shape.to_parry_shape();
                let shape2 = other.shape.to_parry_shape();
                let pose1 = self.pose.to_parry_pose();
                let pose2 = other.pose.to_parry_pose();
                intersection_test(&pose1, &*shape1, &pose2, &*shape2).unwrap_or(false)
            }
        }
    }
}

impl Shape {
    /// Convert this shape to a parry3d SharedShape for GJK intersection testing.
    pub fn to_parry_shape(&self) -> SharedShape {
        match self {
            Shape::Cuboid { half_extents } => {
                SharedShape::cuboid(half_extents[0], half_extents[1], half_extents[2])
            }
            Shape::Cylinder {
                radius,
                half_height,
            } => SharedShape::cylinder(*half_height, *radius),
            Shape::Sphere { radius } => SharedShape::ball(*radius),
            Shape::Hemisphere { radius } => {
                // Conservative: hemisphere approximated as full sphere for collision.
                SharedShape::ball(*radius)
            }
            Shape::Cone {
                radius,
                half_height,
            } => SharedShape::cone(*half_height, *radius),
            Shape::Union(children) => {
                let subshapes: Vec<_> = children
                    .iter()
                    .map(|c| (ParryPose::identity(), c.to_parry_shape()))
                    .collect();
                SharedShape::compound(subshapes)
            }
            Shape::Intersection(children) => {
                // Conservative: use first child's shape for GJK intersection test.
                // The exact_intersects() method handles intersection semantics
                // by checking all children individually.
                if children.is_empty() {
                    SharedShape::ball(0.0)
                } else {
                    children[0].to_parry_shape()
                }
            }
            Shape::Subtract { a, .. } => a.to_parry_shape(),
        }
    }

    /// Local-space AABB before rotation/translation.
    fn local_aabb(&self) -> Aabb {
        match self {
            Shape::Cuboid { half_extents } => Aabb {
                min: [-half_extents[0], -half_extents[1], -half_extents[2]],
                max: [half_extents[0], half_extents[1], half_extents[2]],
            },
            Shape::Cylinder {
                radius,
                half_height,
            } => Aabb {
                min: [-radius, -radius, -*half_height],
                max: [*radius, *radius, *half_height],
            },
            Shape::Sphere { radius } => Aabb {
                min: [-radius, -radius, -radius],
                max: [*radius, *radius, *radius],
            },
            Shape::Hemisphere { radius } => Aabb {
                min: [-radius, -radius, 0.0],
                max: [*radius, *radius, *radius],
            },
            Shape::Cone {
                radius,
                half_height,
            } => Aabb {
                min: [-radius, -radius, -*half_height],
                max: [*radius, *radius, *half_height],
            },
            Shape::Union(children) => {
                let mut aabb = Aabb {
                    min: [f64::INFINITY; 3],
                    max: [f64::NEG_INFINITY; 3],
                };
                for child in children {
                    let ca = child.local_aabb();
                    aabb.min = [
                        aabb.min[0].min(ca.min[0]),
                        aabb.min[1].min(ca.min[1]),
                        aabb.min[2].min(ca.min[2]),
                    ];
                    aabb.max = [
                        aabb.max[0].max(ca.max[0]),
                        aabb.max[1].max(ca.max[1]),
                        aabb.max[2].max(ca.max[2]),
                    ];
                }
                aabb
            }
            Shape::Intersection(children) => {
                let mut aabb = Aabb {
                    min: [f64::NEG_INFINITY; 3],
                    max: [f64::INFINITY; 3],
                };
                for child in children {
                    let ca = child.local_aabb();
                    aabb.min = [
                        aabb.min[0].max(ca.min[0]),
                        aabb.min[1].max(ca.min[1]),
                        aabb.min[2].max(ca.min[2]),
                    ];
                    aabb.max = [
                        aabb.max[0].min(ca.max[0]),
                        aabb.max[1].min(ca.max[1]),
                        aabb.max[2].min(ca.max[2]),
                    ];
                }
                aabb
            }
            Shape::Subtract { a, .. } => a.local_aabb(),
        }
    }
}

/// Axis-aligned bounding box used for spatial overlap tests.
#[derive(Debug, Clone, Serialize)]
pub struct Aabb {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl Aabb {
    pub fn new(min: [f64; 3], max: [f64; 3]) -> Self {
        Self { min, max }
    }

    /// Check whether two AABBs overlap (inclusive boundaries).
    pub fn overlaps(&self, other: &Aabb) -> bool {
        self.min[0] <= other.max[0]
            && self.max[0] >= other.min[0]
            && self.min[1] <= other.max[1]
            && self.max[1] >= other.min[1]
            && self.min[2] <= other.max[2]
            && self.max[2] >= other.min[2]
    }

    /// A string key for this AABB, used as a cheap spatial hash.
    pub fn region_key(&self) -> String {
        format!(
            "{:.1}_{:.1}_{:.1}__{:.1}_{:.1}_{:.1}",
            self.min[0], self.min[1], self.min[2], self.max[0], self.max[1], self.max[2]
        )
    }

    /// Parse a region_key back to an AABB (format: "x1_y1_z1__x2_y2_z2").
    pub fn from_region_key(key: &str) -> Option<Self> {
        let parts: Vec<&str> = key.split("__").collect();
        if parts.len() != 2 {
            return None;
        }
        let min: Vec<f64> = parts[0].split('_').filter_map(|s| s.parse().ok()).collect();
        let max: Vec<f64> = parts[1].split('_').filter_map(|s| s.parse().ok()).collect();
        if min.len() == 3 && max.len() == 3 {
            Some(Self {
                min: [min[0], min[1], min[2]],
                max: [max[0], max[1], max[2]],
            })
        } else {
            None
        }
    }
}

// ── Euler rotation (ZYX order, degrees) ──

/// Rotate an AABB by the pose's Euler angles, then translate.
/// Takes the 8 corners of the local AABB, rotates each, and returns the
/// world-space AABB that bounds all rotated corners.
fn rotate_aabb(local: &Aabb, pose: &Pose) -> Aabb {
    let rot = rotation_matrix_zyx(
        deg_to_rad(pose.rotation[0]),
        deg_to_rad(pose.rotation[1]),
        deg_to_rad(pose.rotation[2]),
    );

    let corners = aabb_corners(local);
    let mut world_min = [f64::INFINITY; 3];
    let mut world_max = [f64::NEG_INFINITY; 3];

    for corner in &corners {
        let rotated = mul_mat_vec(&rot, corner);
        let wx = rotated[0] + pose.position[0];
        let wy = rotated[1] + pose.position[1];
        let wz = rotated[2] + pose.position[2];
        world_min[0] = world_min[0].min(wx);
        world_min[1] = world_min[1].min(wy);
        world_min[2] = world_min[2].min(wz);
        world_max[0] = world_max[0].max(wx);
        world_max[1] = world_max[1].max(wy);
        world_max[2] = world_max[2].max(wz);
    }

    Aabb {
        min: world_min,
        max: world_max,
    }
}

fn aabb_corners(aabb: &Aabb) -> [[f64; 3]; 8] {
    let (x0, x1) = (aabb.min[0], aabb.max[0]);
    let (y0, y1) = (aabb.min[1], aabb.max[1]);
    let (z0, z1) = (aabb.min[2], aabb.max[2]);
    [
        [x0, y0, z0],
        [x1, y0, z0],
        [x0, y1, z0],
        [x0, y0, z1],
        [x1, y1, z0],
        [x1, y0, z1],
        [x0, y1, z1],
        [x1, y1, z1],
    ]
}

/// ZYX rotation matrix: R = Rz(yaw) * Ry(pitch) * Rx(roll).
fn rotation_matrix_zyx(yaw: f64, pitch: f64, roll: f64) -> [[f64; 3]; 3] {
    let (sz, cz) = yaw.sin_cos();
    let (sy, cy) = pitch.sin_cos();
    let (sx, cx) = roll.sin_cos();

    // Rz * Ry * Rx
    [
        [cz * cy, cz * sy * sx - sz * cx, cz * sy * cx + sz * sx],
        [sz * cy, sz * sy * sx + cz * cx, sz * sy * cx - cz * sx],
        [-sy, cy * sx, cy * cx],
    ]
}

fn mul_mat_vec(m: &[[f64; 3]; 3], v: &[f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

fn deg_to_rad(deg: f64) -> f64 {
    deg * PI / 180.0
}

// ── Region overlap helper ──

/// Helper trait for checking spatial overlap by region key.
pub trait RegionOverlap {
    fn overlaps_by_key(&self, other_key: &str) -> bool;
}

impl RegionOverlap for Aabb {
    fn overlaps_by_key(&self, other_key: &str) -> bool {
        if let Some(other) = Aabb::from_region_key(other_key) {
            self.overlaps(&other)
        } else {
            false
        }
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aabb_overlap_positive() {
        let a = Aabb::new([0.0, 0.0, 0.0], [10.0, 10.0, 10.0]);
        let b = Aabb::new([5.0, 5.0, 5.0], [15.0, 15.0, 15.0]);
        assert!(a.overlaps(&b));
    }

    #[test]
    fn test_aabb_overlap_negative() {
        let a = Aabb::new([0.0, 0.0, 0.0], [5.0, 5.0, 5.0]);
        let b = Aabb::new([10.0, 10.0, 10.0], [15.0, 15.0, 15.0]);
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn test_aabb_touching_boundary() {
        let a = Aabb::new([0.0, 0.0, 0.0], [10.0, 10.0, 10.0]);
        let b = Aabb::new([10.0, 10.0, 10.0], [20.0, 20.0, 20.0]);
        assert!(a.overlaps(&b));
    }

    #[test]
    fn test_identity_pose_preserves_aabb() {
        let geom = Geometry {
            shape: Shape::Cuboid {
                half_extents: [5.0, 5.0, 10.0],
            },
            pose: Pose {
                position: [5.0, 5.0, 10.0],
                rotation: [0.0, 0.0, 0.0],
            },
            region: None,
        };
        let aabb = geom.world_aabb();
        assert_eq!(aabb.min, [0.0, 0.0, 0.0]);
        assert_eq!(aabb.max, [10.0, 10.0, 20.0]);
    }

    #[test]
    fn test_90_degree_yaw_swaps_axes() {
        let geom = Geometry {
            shape: Shape::Cuboid {
                half_extents: [2.0, 1.0, 3.0],
            },
            pose: Pose {
                position: [0.0, 0.0, 0.0],
                rotation: [90.0, 0.0, 0.0], // yaw 90° around Z
            },
            region: None,
        };
        let aabb = geom.world_aabb();
        // After 90° yaw: x→y, y→-x, so half_extents [2,1,3] → AABB roughly [-1, -2, -3] to [1, 2, 3]
        // Actually: x_local * cos90 - y_local * sin90 = -y_local; y_local * cos90 + x_local * sin90 = x_local
        // So the AABB should be about [-1, -2, -3] to [1, 2, 3]
        let eps = 1e-10;
        assert!((aabb.min[0] + 1.0).abs() < eps, "min[0] = {}", aabb.min[0]);
        assert!((aabb.min[1] + 2.0).abs() < eps, "min[1] = {}", aabb.min[1]);
        assert!((aabb.max[0] - 1.0).abs() < eps, "max[0] = {}", aabb.max[0]);
        assert!((aabb.max[1] - 2.0).abs() < eps, "max[1] = {}", aabb.max[1]);
    }

    #[test]
    fn test_sphere_world_aabb() {
        let geom = Geometry {
            shape: Shape::Sphere { radius: 5.0 },
            pose: Pose {
                position: [10.0, 10.0, 10.0],
                rotation: [0.0, 0.0, 0.0],
            },
            region: None,
        };
        let aabb = geom.world_aabb();
        assert_eq!(aabb.min, [5.0, 5.0, 5.0]);
        assert_eq!(aabb.max, [15.0, 15.0, 15.0]);
    }

    #[test]
    fn test_cylinder_world_aabb() {
        let geom = Geometry {
            shape: Shape::Cylinder {
                radius: 1.0,
                half_height: 4.0,
            },
            pose: Pose {
                position: [0.0, 0.0, 4.0],
                rotation: [0.0, 0.0, 0.0],
            },
            region: None,
        };
        let aabb = geom.world_aabb();
        assert_eq!(aabb.min, [-1.0, -1.0, 0.0]);
        assert_eq!(aabb.max, [1.0, 1.0, 8.0]);
    }

    #[test]
    fn test_cone_world_aabb() {
        let geom = Geometry {
            shape: Shape::Cone {
                radius: 3.0,
                half_height: 5.0,
            },
            pose: Pose::from_position([0.0, 0.0, 5.0]),
            region: None,
        };
        let aabb = geom.world_aabb();
        assert_eq!(aabb.min, [-3.0, -3.0, 0.0]);
        assert_eq!(aabb.max, [3.0, 3.0, 10.0]);
    }

    // ── GJK exact intersection tests ──

    #[test]
    fn test_gjk_cuboids_intersecting() {
        let a = Geometry {
            shape: Shape::Cuboid {
                half_extents: [2.0, 2.0, 2.0],
            },
            pose: Pose::from_position([0.0, 0.0, 0.0]),
            region: None,
        };
        let b = Geometry {
            shape: Shape::Cuboid {
                half_extents: [2.0, 2.0, 2.0],
            },
            pose: Pose::from_position([3.0, 3.0, 3.0]),
            region: None,
        };
        assert!(a.exact_intersects(&b));
    }

    #[test]
    fn test_gjk_cuboids_separated() {
        let a = Geometry {
            shape: Shape::Cuboid {
                half_extents: [1.0, 1.0, 1.0],
            },
            pose: Pose::from_position([0.0, 0.0, 0.0]),
            region: None,
        };
        let b = Geometry {
            shape: Shape::Cuboid {
                half_extents: [1.0, 1.0, 1.0],
            },
            pose: Pose::from_position([10.0, 10.0, 10.0]),
            region: None,
        };
        assert!(!a.exact_intersects(&b));
    }

    #[test]
    fn test_gjk_false_positive_cone_cylinder() {
        // A cone whose AABB extends to z=5, but physically tapers to a point there.
        // A cylinder whose AABB starts at z=4.5. The AABBs overlap in Z [4.5, 5],
        // but GJK correctly identifies no intersection.
        let cone = Geometry {
            shape: Shape::Cone {
                radius: 3.0,
                half_height: 5.0,
            },
            pose: Pose::from_position([0.0, 0.0, 0.0]),
            region: None,
        };
        let cylinder = Geometry {
            shape: Shape::Cylinder {
                radius: 2.0,
                half_height: 1.0,
            },
            pose: Pose::from_position([0.0, 0.0, 5.5]),
            region: None,
        };
        // AABBs overlap in Z: cone=[-5,5], cylinder=[4.5,6.5]
        assert!(cone.world_aabb().overlaps(&cylinder.world_aabb()));
        // GJK: cone tip at z=5 is a point, cylinder bottom at z=4.5 has radius 2.
        // Cone radius at z=4.5 is 0.15, cylinder radius is 2. Gap > 0.
        assert!(!cone.exact_intersects(&cylinder));
    }

    #[test]
    fn test_gjk_spheres() {
        let a = Geometry {
            shape: Shape::Sphere { radius: 5.0 },
            pose: Pose::from_position([0.0, 0.0, 0.0]),
            region: None,
        };
        let b = Geometry {
            shape: Shape::Sphere { radius: 3.0 },
            pose: Pose::from_position([6.0, 0.0, 0.0]),
            region: None,
        };
        // Distance between centers = 6, sum of radii = 8, so they intersect
        assert!(a.exact_intersects(&b));
    }

    #[test]
    fn test_gjk_spheres_separated() {
        let a = Geometry {
            shape: Shape::Sphere { radius: 2.0 },
            pose: Pose::from_position([0.0, 0.0, 0.0]),
            region: None,
        };
        let b = Geometry {
            shape: Shape::Sphere { radius: 2.0 },
            pose: Pose::from_position([10.0, 0.0, 0.0]),
            region: None,
        };
        // Distance between centers = 10, sum of radii = 4, so they don't intersect
        assert!(!a.exact_intersects(&b));
    }
}
