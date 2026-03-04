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

/// Clips a vertical gate opening to stay above the ground plane (y=0).
/// Returns adjusted `(center_y, half_height)` that excludes below-ground portions.
/// If the opening is entirely underground, returns `(0.0, 0.0)`.
pub fn clip_opening_to_ground(center_y: f32, half_height: f32) -> (f32, f32) {
    let top = center_y + half_height;
    if top <= 0.0 {
        return (0.0, 0.0);
    }
    let bottom = center_y - half_height;
    if bottom >= 0.0 {
        return (center_y, half_height);
    }
    // Partially underground: clamp bottom to 0, keep top unchanged
    let new_half_height = top / 2.0;
    let new_center_y = new_half_height;
    (new_center_y, new_half_height)
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- clip_opening_to_ground ---

    #[test]
    fn clip_fully_above_ground() {
        // Center at y=5, half_height=3 → bottom=2, top=8 — no clipping
        let (cy, hh) = clip_opening_to_ground(5.0, 3.0);
        assert_eq!(cy, 5.0);
        assert_eq!(hh, 3.0);
    }

    #[test]
    fn clip_bottom_at_ground() {
        // Center at y=3, half_height=3 → bottom=0, top=6 — no clipping needed
        let (cy, hh) = clip_opening_to_ground(3.0, 3.0);
        assert_eq!(cy, 3.0);
        assert_eq!(hh, 3.0);
    }

    #[test]
    fn clip_partially_underground() {
        // Center at y=2, half_height=3 → bottom=-1, top=5
        // Clipped: bottom=0, top=5 → center=2.5, half_height=2.5
        let (cy, hh) = clip_opening_to_ground(2.0, 3.0);
        assert!((cy - 2.5).abs() < 1e-6);
        assert!((hh - 2.5).abs() < 1e-6);
    }

    #[test]
    fn clip_center_at_ground() {
        // Center at y=0, half_height=4 → bottom=-4, top=4
        // Clipped: bottom=0, top=4 → center=2, half_height=2
        let (cy, hh) = clip_opening_to_ground(0.0, 4.0);
        assert!((cy - 2.0).abs() < 1e-6);
        assert!((hh - 2.0).abs() < 1e-6);
    }

    #[test]
    fn clip_fully_underground() {
        // Center at y=-5, half_height=2 → top=-3 — entirely underground
        let (cy, hh) = clip_opening_to_ground(-5.0, 2.0);
        assert_eq!(cy, 0.0);
        assert_eq!(hh, 0.0);
    }

    #[test]
    fn clip_top_at_ground() {
        // Center at y=-2, half_height=2 → top=0 — just touching ground
        let (cy, hh) = clip_opening_to_ground(-2.0, 2.0);
        assert_eq!(cy, 0.0);
        assert_eq!(hh, 0.0);
    }

    #[test]
    fn clip_zero_height() {
        let (cy, hh) = clip_opening_to_ground(5.0, 0.0);
        assert_eq!(cy, 5.0);
        assert_eq!(hh, 0.0);
    }
}
