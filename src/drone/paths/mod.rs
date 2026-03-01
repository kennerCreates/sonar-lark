mod generation;
mod start_positions;

pub use generation::{
    adaptive_approach_offset, generate_drone_race_path, generate_race_path,
};
pub use start_positions::compute_start_positions;

use bevy::math::cubic_splines::CubicCurve;
use bevy::prelude::*;

use crate::course::data::CourseData;
use crate::obstacle::library::ObstacleLibrary;

pub struct RacePath {
    pub spline: CubicCurve<Vec3>,
    pub gate_positions: Vec<Vec3>,
    pub gate_forwards: Vec<Vec3>,
}

/// Extract gate positions, forwards, and 2D half-extents from course data, sorted by gate_order.
/// Half-extents are (width, height) of the trigger volume in world space, scaled by instance scale.
pub(super) fn extract_sorted_gates(
    course: &CourseData,
    library: &ObstacleLibrary,
) -> (Vec<Vec3>, Vec<Vec3>, Vec<Vec2>) {
    let mut gates: Vec<(u32, Vec3, Vec3, Vec2)> = course
        .instances
        .iter()
        .filter_map(|inst| {
            inst.gate_order.map(|order| {
                let tv = library
                    .get(&inst.obstacle_id)
                    .and_then(|def| def.trigger_volume.as_ref());
                // Use the trigger volume center as the fly-through target,
                // not the obstacle origin (which is at ground level).
                let fly_through_offset = tv
                    .map(|tv| inst.rotation * (tv.offset * inst.scale))
                    .unwrap_or(Vec3::ZERO);
                let local_fwd = tv.map(|tv| tv.forward).unwrap_or(Vec3::NEG_Z);
                let world_fwd = inst.rotation
                    * if inst.gate_forward_flipped { -local_fwd } else { local_fwd };
                let half_extents_2d = tv
                    .map(|tv| Vec2::new(tv.half_extents.x * inst.scale.x, tv.half_extents.y * inst.scale.y))
                    .unwrap_or(Vec2::new(3.0, 3.0));
                (order, inst.translation + fly_through_offset, world_fwd, half_extents_2d)
            })
        })
        .collect();
    gates.sort_by_key(|(order, _, _, _)| *order);
    let positions = gates.iter().map(|(_, pos, _, _)| *pos).collect();
    let forwards = gates.iter().map(|(_, _, fwd, _)| *fwd).collect();
    let extents = gates.iter().map(|(_, _, _, ext)| *ext).collect();
    (positions, forwards, extents)
}
