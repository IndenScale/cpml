# Changelog

All notable changes to CPML will be documented in this file.

## [0.1.0] — 2026-05-09

### Core Compiler Pipeline

- YAML-based declarative DSL parser for construction process modeling
- Five-stage compiler pipeline: parse → resolve → expand → keyframe → field evaluation
- `cpml check` command: full compilation with diagnostic reporting
- `cpml parse` command: raw YAML deserialization for debugging

### Field System (4 types + 1 WIP)

- **OccupancyField** — spatial collision detection with hard/soft discrimination and self-exclusion
- **CapacityField** — resource supply/consumption with SUM accumulation
- **ScalarField** — persistent value progression with Max/Min/Sum/Replace operators
- **PresenceField** — record-based state with key, type, attributes, and validity windows
- **RateField** — flow-rate field (schema support, evaluation WIP)

### Geometry & Collision

- 6 geometric primitives: AABB, Cuboid, Cylinder, Sphere, Hemisphere, Cone
- World-space pose with Euler rotation (ZYX, degrees)
- AABB-based fast spatial overlap with region-key hashing
- GJK-based exact intersection testing via parry3d
- Occlusion culling: raycast barrier check for blocked projections (confidence-weighted)

### Diagnostic System

- 5 severity levels: Debug → Info → Warning → Error → Fatal
- Blame attribution: traces field failures back to contributing projections
- Auto-level promotion: Hard collision ➔ Error, Soft collision ➔ Warning
- Per-keyframe evaluation with persistent field state across time

### Syntax Sugar

- `collision` shorthand: auto-generates hard/soft occupancy probe+projection pairs
- `structure` shorthand: auto-generates symmetric probe+projection pairs for common patterns
- `barriers` section: declare occlusion geometry for culling blocked projections

### Sample Scenarios (4 .cpml files)

- Tower crane collision (hard/soft body + swing radius overlap)
- Concrete curing scalar progression (strength buildup → upper structure check)
- Presence permit (excavation requires approved permit record)
- Resource contention (multiple activities drawing from limited power capacity)

### Technical Foundation

- Error handling: 7 error variants via `thiserror`
- Integration tests: 10 tests covering collision, scalar progression, presence check, resource contention, and occlusion
- Unit tests: 15 tests covering geometry (AABB, world-space transforms, GJK) and keyframe extraction
- Architecture Decision Records: 10 ADRs documenting design rationale
- TECH_DEBT.md: 3 of 4 architectural debt items resolved (GJK collision, occlusion culling, RateField + confidence); 1 remains (Structure/Presence interception — already fixed in earlier iteration)

## [Unreleased] — 2026-05-09 (post-v0.1.0)

### Resolved Technical Debt

- **GJK/SAT exact collision**: `Geometry::exact_intersects()` with three-phase detection (AABB → GJK → occlusion) via `parry3d-f64`
- **Occlusion culling**: `barriers` document section, `is_occluded()` raycasting in `eval_occupancy()`
- **RateField**: full field type with sliding-window rate computation, `eval_rate()`, `update_rate_state()`
- **Confidence Score**: optional `confidence: 0.0–1.0` on projections, weighted contribution in capacity/scalar/rate fields

### New Samples

- `samples/occlusion_demo.cpml` — wall barrier blocks soft collision between tower cranes
- `samples/ratefield_demo.cpml` — flow-rate tracking with confidence-weighted contributions

### New Tests

- `tests/ratefield.rs` — basic flow and confidence weighting integration tests
- Occlusion culling test in `tests/collisions.rs`
