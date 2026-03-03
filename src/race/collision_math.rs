use bevy::math::Vec3;

use super::collision::{GateOpening, ObstacleObb};

/// Swept segment vs OBB intersection using slab method.
/// `expansion` (drone radius) is added to half_extents at test time.
/// Returns the first hit point on the (expanded) OBB surface, or `None`.
#[allow(dead_code)]
pub fn segment_obb_intersection(
    p0: Vec3,
    p1: Vec3,
    obb: &ObstacleObb,
    expansion: f32,
) -> Option<Vec3> {
    let dir = p1 - p0;
    let delta = p0 - obb.center;

    let mut t_min = 0.0_f32;
    let mut t_max = 1.0_f32;

    for i in 0..3 {
        let axis = obb.axes[i];
        let half = obb.half_extents[i] + expansion;

        let e = axis.dot(delta);
        let f = axis.dot(dir);

        if f.abs() > 1e-8 {
            let inv_f = 1.0 / f;
            let mut t1 = (-half - e) * inv_f;
            let mut t2 = (half - e) * inv_f;

            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }

            t_min = t_min.max(t1);
            t_max = t_max.min(t2);

            if t_min > t_max {
                return None;
            }
        } else {
            // Ray is parallel to this slab — check if origin is within
            if (-half - e) > 0.0 || (half - e) < 0.0 {
                // Equivalent: |e| > half
                return None;
            }
        }
    }

    // t_min is in [0, 1] — segment intersects the OBB
    Some(p0 + t_min * dir)
}

/// Returns true if `point` is within the gate opening (infinite depth tube).
/// Projects the offset onto the opening's right and up axes; ignores the
/// depth axis entirely so any approach angle works.
#[allow(dead_code)]
pub fn point_in_gate_opening(point: Vec3, opening: &GateOpening) -> bool {
    let offset = point - opening.center;
    let x = offset.dot(opening.right).abs();
    let y = offset.dot(opening.up).abs();
    x < opening.half_width && y < opening.half_height
}
