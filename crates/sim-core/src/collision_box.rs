//! Per-car planar collision box.
//!
//! Prototype: one XZ box derived from the baked car visual AABB is shared by
//! car-to-car contact and wall/scenery probes. The box is described in the
//! vehicle's local frame (`+X` right, `-Z` forward), matching the visual mesh
//! bounds. A small skin is folded into the half extents so contacts settle
//! without visible interpenetration.

use sim_types::{Aabb, CarPhysics};

/// Skin added to each visual half extent. The collision box therefore sits a
/// centimetre or so outside the rendered body; this keeps resolved contacts
/// from visibly overlapping when the simulation settles.
pub const COLLISION_SKIN: f32 = 0.02;

/// Minimum half extent accepted from a degenerate or missing AABB.
const MIN_HALF_EXTENT: f32 = 0.05;

/// Planar collision box for one car, in the vehicle's local XZ frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollisionBox {
    /// Half extent along local X (left/right).
    pub half_width: f32,
    /// Half extent along local Z (front/back).
    pub half_length: f32,
    /// Local X centre offset. Non-zero when the visual AABB is asymmetric.
    pub center_x: f32,
    /// Local Z centre offset. Positive is toward the rear (`+Z`).
    pub center_z: f32,
}

impl CollisionBox {
    /// Builds a collision box from the baked car visual AABB.
    ///
    /// The skin is added to the visual half extents, so the effective footprint
    /// is the visual AABB plus [`COLLISION_SKIN`] on every side.
    pub fn from_aabb(bounds: &Aabb) -> Self {
        let half_width =
            ((bounds.max[0] - bounds.min[0]) * 0.5).max(MIN_HALF_EXTENT) + COLLISION_SKIN;
        let half_length =
            ((bounds.max[2] - bounds.min[2]) * 0.5).max(MIN_HALF_EXTENT) + COLLISION_SKIN;
        let center_x = (bounds.max[0] + bounds.min[0]) * 0.5;
        let center_z = (bounds.max[2] + bounds.min[2]) * 0.5;
        Self {
            half_width,
            half_length,
            center_x,
            center_z,
        }
    }

    /// Heuristic fallback used when no baked AABB has been installed.
    ///
    /// This preserves the previous wheelbase/track-derived contact footprint so
    /// callers that never set a box keep their current behaviour.
    pub fn from_car_physics(physics: &CarPhysics) -> Self {
        const LENGTH_OVERHANG: f32 = 0.9;
        const WIDTH_MARGIN: f32 = 0.25;
        const MIN_HALF_LENGTH: f32 = 1.9;
        const MAX_HALF_LENGTH: f32 = 2.3;
        const MIN_HALF_WIDTH: f32 = 0.9;
        const MAX_HALF_WIDTH: f32 = 1.1;

        let half_length = (physics.wheelbase.max(0.0) * 0.5 + LENGTH_OVERHANG + COLLISION_SKIN)
            .clamp(MIN_HALF_LENGTH, MAX_HALF_LENGTH);
        let half_track = physics.track_width.iter().copied().fold(0.0_f32, f32::max) * 0.5;
        let half_width =
            (half_track + WIDTH_MARGIN + COLLISION_SKIN).clamp(MIN_HALF_WIDTH, MAX_HALF_WIDTH);
        Self {
            half_width,
            half_length,
            center_x: 0.0,
            center_z: 0.0,
        }
    }
}
