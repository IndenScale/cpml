use serde::de;
use serde::{Deserialize, Deserializer};

/// A named geometry definition with optional pose and shape.
///
/// Custom deserializer rejects unknown top-level keys so that mistakes like
/// `position: [...]` (outside a `pose:` block) or `shape: cuboid` (instead of
/// `cuboid: {half_extents: [...]}`) produce a fatal error rather than being
/// silently ignored by serde's flatten.
#[derive(Debug, Clone)]
pub struct GeometryDef {
    pub id: String,
    pub pose: Option<PoseDef>,
    pub shape: ShapeDef,
    /// Optional region ID for capacity hierarchy. Geometries in child regions
    /// see capacity contributions from all ancestor regions.
    pub region: Option<String>,
}

impl<'de> Deserialize<'de> for GeometryDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        const KNOWN_KEYS: &[&str] = &[
            "id",
            "pose",
            "region",
            // ShapeDef variant names (internally tagged, snake_case)
            "aabb",
            "cuboid",
            "cylinder",
            "sphere",
            "hemisphere",
            "cone",
            "union",
            "intersection",
            "subtract",
        ];

        // Parse into generic YAML value first so we can inspect keys.
        let value = serde_yaml::Value::deserialize(deserializer)?;

        if let serde_yaml::Value::Mapping(ref map) = value {
            for key in map.keys() {
                if let serde_yaml::Value::String(k) = key {
                    if !KNOWN_KEYS.contains(&k.as_str()) {
                        return Err(de::Error::unknown_field(k, KNOWN_KEYS));
                    }
                }
            }
        }

        // Re-parse through a helper struct that does the actual flatten-based
        // deserialization.  We avoid recursive calls by using a private helper.
        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case")]
        struct Helper {
            id: String,
            #[serde(default)]
            pose: Option<PoseDef>,
            #[serde(default)]
            region: Option<String>,
            #[serde(flatten)]
            shape: ShapeDef,
        }

        let helper: Helper = serde_yaml::from_value(value).map_err(de::Error::custom)?;

        Ok(GeometryDef {
            id: helper.id,
            pose: helper.pose,
            shape: helper.shape,
            region: helper.region,
        })
    }
}

/// Position and orientation in world space.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct PoseDef {
    #[serde(default)]
    pub position: [f64; 3],
    /// Euler angles in degrees, ZYX order. Defaults to zero (identity rotation).
    #[serde(default)]
    pub rotation: [f64; 3],
}

impl Default for PoseDef {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        }
    }
}

/// Supported geometric primitives. All shapes are defined in local space
/// and transformed to world space by the parent GeometryDef's pose.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ShapeDef {
    /// Axis-aligned box defined by min/max corners (legacy shorthand).
    /// Internally converted to Cuboid + centered pose.
    Aabb { min: [f64; 3], max: [f64; 3] },
    /// Oriented box with half-extents from local origin.
    Cuboid { half_extents: [f64; 3] },
    /// Cylinder aligned with local Z axis. Radius in XY plane, height along Z.
    Cylinder { radius: f64, half_height: f64 },
    /// Sphere centered at local origin.
    Sphere { radius: f64 },
    /// Hemisphere with flat face at Z=0, dome in +Z direction.
    Hemisphere { radius: f64 },
    /// Cone aligned with local Z axis. Base at Z=-half_height, apex at Z=+half_height.
    Cone { radius: f64, half_height: f64 },
    /// Boolean union: combined volume of all child shapes.
    Union(Vec<ShapeDef>),
    /// Boolean intersection: volume common to all child shapes.
    Intersection(Vec<ShapeDef>),
    /// Boolean subtraction: volume of `a` minus volume of `b`.
    Subtract { a: Box<ShapeDef>, b: Box<ShapeDef> },
}
