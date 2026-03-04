use bevy::prelude::*;

use crate::palette;

// --- Axis ---

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Axis {
    #[default]
    X,
    Y,
    Z,
}

impl Axis {
    pub(crate) fn direction(self) -> Vec3 {
        match self {
            Axis::X => Vec3::X,
            Axis::Y => Vec3::Y,
            Axis::Z => Vec3::Z,
        }
    }

    pub(crate) fn rotated_direction(self, yaw_quat: Quat) -> Vec3 {
        yaw_quat * self.direction()
    }

    pub(crate) fn color(self, hovered: bool, active: bool) -> Color {
        let brightness = if active {
            1.0
        } else if hovered {
            0.8
        } else {
            0.5
        };
        let base = match self {
            Axis::X => palette::NEON_RED,
            Axis::Y => palette::GREEN,
            Axis::Z => palette::CAROLINA,
        };
        let Color::Srgba(c) = base else { unreachable!() };
        Color::srgb(c.red * brightness, c.green * brightness, c.blue * brightness)
    }
}

// --- Sign ---

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sign {
    Positive,
    Negative,
}

// --- Math helpers ---

/// Project the camera ray onto a world-space axis through `origin` and return the parameter `t`
/// such that `origin + axis_dir * t` is the closest point on the axis to the ray.
pub(crate) fn closest_point_on_axis(ray: Ray3d, origin: Vec3, axis_dir: Vec3) -> f32 {
    let ray_origin = ray.origin;
    let ray_dir = *ray.direction;
    let w = ray_origin - origin;

    let a = ray_dir.dot(ray_dir);
    let b = ray_dir.dot(axis_dir);
    let c = axis_dir.dot(axis_dir);
    let d = ray_dir.dot(w);
    let e = axis_dir.dot(w);

    let denom = a * c - b * b;
    if denom.abs() < 1e-6 {
        return 0.0;
    }

    (a * e - b * d) / denom
}

/// Distance from a point to a line segment in 2D screen space.
pub(crate) fn point_to_segment_distance(point: Vec2, seg_start: Vec2, seg_end: Vec2) -> f32 {
    let ab = seg_end - seg_start;
    let ap = point - seg_start;
    let len_sq = ab.length_squared();
    if len_sq < 1e-6 {
        return ap.length();
    }
    let t = (ap.dot(ab) / len_sq).clamp(0.0, 1.0);
    let proj = seg_start + ab * t;
    (point - proj).length()
}

/// Intersect a ray with an arbitrary plane defined by a point and normal.
/// Returns `None` if the ray is nearly parallel to the plane or behind it.
pub(crate) fn ray_intersect_plane(
    ray: Ray3d,
    plane_point: Vec3,
    plane_normal: Vec3,
) -> Option<Vec3> {
    let denom = ray.direction.dot(plane_normal);
    if denom.abs() < 1e-6 {
        return None;
    }
    let t = (plane_point - ray.origin).dot(plane_normal) / denom;
    if t < 0.0 {
        return None;
    }
    Some(ray.origin + *ray.direction * t)
}

/// Two orthogonal unit vectors that span the plane perpendicular to `axis`.
/// Used to sample rotation ring points and compute drag angles.
pub(crate) fn perpendicular_basis(axis: Axis) -> (Vec3, Vec3) {
    match axis {
        Axis::X => (Vec3::Y, Vec3::Z),
        Axis::Y => (Vec3::X, Vec3::Z),
        Axis::Z => (Vec3::X, Vec3::Y),
    }
}

/// Extract only the Y-rotation (yaw) from a transform's rotation.
pub(crate) fn yaw_quat_from_transform(transform: &Transform) -> Quat {
    let (yaw, _, _) = transform.rotation.to_euler(EulerRot::YXZ);
    Quat::from_rotation_y(yaw)
}

/// Perpendicular basis vectors rotated by yaw.
pub(crate) fn rotated_perpendicular_basis(axis: Axis, yaw_quat: Quat) -> (Vec3, Vec3) {
    let (a, b) = perpendicular_basis(axis);
    (yaw_quat * a, yaw_quat * b)
}

// --- Rotation ring constants ---

pub(crate) const RING_RADIUS: f32 = 3.0;
pub(crate) const RING_SAMPLES: usize = 32;
pub(crate) const RING_HIT_THRESHOLD: f32 = 15.0;
pub(crate) const ROTATION_STEP_DEG: f32 = 5.0;

/// Minimum screen-space distance from the cursor to the sampled ring.
pub(crate) fn sample_ring_screen_dist(
    camera: &Camera,
    camera_gt: &GlobalTransform,
    cursor_pos: Vec2,
    center: Vec3,
    ref1: Vec3,
    ref2: Vec3,
    radius: f32,
    n: usize,
) -> f32 {
    use std::f32::consts::TAU;
    let mut min_dist = f32::MAX;
    for i in 0..n {
        let angle = i as f32 * TAU / n as f32;
        let world_pt = center + (ref1 * angle.cos() + ref2 * angle.sin()) * radius;
        if let Ok(screen_pt) = camera.world_to_viewport(camera_gt, world_pt) {
            let d = (cursor_pos - screen_pt).length();
            if d < min_dist {
                min_dist = d;
            }
        }
    }
    min_dist
}

#[cfg(test)]
mod tests {
    use super::*;
    // --- perpendicular_basis ---

    #[test]
    fn perpendicular_basis_x() {
        let (a, b) = perpendicular_basis(Axis::X);
        assert_eq!(a, Vec3::Y);
        assert_eq!(b, Vec3::Z);
    }

    #[test]
    fn perpendicular_basis_y() {
        let (a, b) = perpendicular_basis(Axis::Y);
        assert_eq!(a, Vec3::X);
        assert_eq!(b, Vec3::Z);
    }

    #[test]
    fn perpendicular_basis_z() {
        let (a, b) = perpendicular_basis(Axis::Z);
        assert_eq!(a, Vec3::X);
        assert_eq!(b, Vec3::Y);
    }

    #[test]
    fn perpendicular_basis_orthogonal_to_axis() {
        for axis in [Axis::X, Axis::Y, Axis::Z] {
            let (a, b) = perpendicular_basis(axis);
            let dir = axis.direction();
            assert!(a.dot(dir).abs() < 1e-6, "ref1 not perpendicular to {dir}");
            assert!(b.dot(dir).abs() < 1e-6, "ref2 not perpendicular to {dir}");
            assert!(a.dot(b).abs() < 1e-6, "ref1 and ref2 not orthogonal");
        }
    }

    // --- point_to_segment_distance ---

    #[test]
    fn point_to_segment_at_midpoint() {
        let dist = point_to_segment_distance(
            Vec2::new(5.0, 3.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
        );
        assert!((dist - 3.0).abs() < 1e-5);
    }

    #[test]
    fn point_to_segment_at_start() {
        let dist = point_to_segment_distance(
            Vec2::new(-1.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
        );
        assert!((dist - 1.0).abs() < 1e-5);
    }

    #[test]
    fn point_to_segment_at_end() {
        let dist = point_to_segment_distance(
            Vec2::new(11.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
        );
        assert!((dist - 1.0).abs() < 1e-5);
    }

    #[test]
    fn point_to_segment_degenerate() {
        let dist = point_to_segment_distance(
            Vec2::new(3.0, 4.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 0.0),
        );
        assert!((dist - 5.0).abs() < 1e-5);
    }

    #[test]
    fn point_to_segment_on_segment() {
        let dist = point_to_segment_distance(
            Vec2::new(5.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
        );
        assert!(dist < 1e-5);
    }

    // --- ray_intersect_plane ---

    fn ray(origin: Vec3, direction: Vec3) -> Ray3d {
        Ray3d {
            origin,
            direction: Dir3::new(direction).unwrap(),
        }
    }

    #[test]
    fn ray_plane_perpendicular_hit() {
        let r = ray(Vec3::new(0.0, 5.0, 0.0), Vec3::NEG_Y);
        let hit = ray_intersect_plane(r, Vec3::ZERO, Vec3::Y).unwrap();
        assert!((hit - Vec3::ZERO).length() < 1e-5);
    }

    #[test]
    fn ray_plane_offset_hit() {
        let r = ray(Vec3::new(3.0, 10.0, 7.0), Vec3::NEG_Y);
        let hit = ray_intersect_plane(r, Vec3::ZERO, Vec3::Y).unwrap();
        assert!((hit.x - 3.0).abs() < 1e-5);
        assert!(hit.y.abs() < 1e-5);
        assert!((hit.z - 7.0).abs() < 1e-5);
    }

    #[test]
    fn ray_plane_parallel_returns_none() {
        let r = ray(Vec3::new(0.0, 5.0, 0.0), Vec3::X);
        assert!(ray_intersect_plane(r, Vec3::ZERO, Vec3::Y).is_none());
    }

    #[test]
    fn ray_plane_behind_returns_none() {
        let r = ray(Vec3::new(0.0, 5.0, 0.0), Vec3::Y);
        assert!(ray_intersect_plane(r, Vec3::ZERO, Vec3::Y).is_none());
    }

    #[test]
    fn ray_plane_angled_plane() {
        let r = ray(Vec3::ZERO, Vec3::X);
        let plane_pt = Vec3::new(10.0, 0.0, 0.0);
        let plane_normal = Vec3::NEG_X;
        let hit = ray_intersect_plane(r, plane_pt, plane_normal).unwrap();
        assert!((hit.x - 10.0).abs() < 1e-5);
    }

    // --- closest_point_on_axis ---

    #[test]
    fn closest_point_on_axis_direct() {
        // Ray pointing at the Y axis from the side
        let r = ray(Vec3::new(5.0, 3.0, 0.0), Vec3::NEG_X);
        let t = closest_point_on_axis(r, Vec3::ZERO, Vec3::Y);
        // Closest point on Y axis to this ray should be at y=3
        assert!((t - 3.0).abs() < 1e-4);
    }

    #[test]
    fn closest_point_on_axis_at_origin() {
        let r = ray(Vec3::new(5.0, 0.0, 0.0), Vec3::NEG_X);
        let t = closest_point_on_axis(r, Vec3::ZERO, Vec3::Y);
        assert!(t.abs() < 1e-4);
    }

    #[test]
    fn closest_point_on_axis_parallel_returns_zero() {
        let r = ray(Vec3::new(0.0, 5.0, 0.0), Vec3::Y);
        let t = closest_point_on_axis(r, Vec3::ZERO, Vec3::Y);
        assert_eq!(t, 0.0);
    }

    #[test]
    fn closest_point_on_axis_with_offset_origin() {
        let r = ray(Vec3::new(10.0, 7.0, 0.0), Vec3::NEG_X);
        let origin = Vec3::new(0.0, 2.0, 0.0);
        let t = closest_point_on_axis(r, origin, Vec3::Y);
        // Ray is at y=7, axis origin at y=2, so closest on axis is at t=5
        assert!((t - 5.0).abs() < 1e-4);
    }

    #[test]
    fn closest_point_on_axis_z_axis() {
        let r = ray(Vec3::new(3.0, 0.0, 8.0), Vec3::NEG_X);
        let t = closest_point_on_axis(r, Vec3::ZERO, Vec3::Z);
        assert!((t - 8.0).abs() < 1e-4);
    }

    // --- rotated_direction ---

    #[test]
    fn rotated_direction_90_y() {
        let yaw = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let rx = Axis::X.rotated_direction(yaw);
        assert!((rx - Vec3::NEG_Z).length() < 1e-5);
        let rz = Axis::Z.rotated_direction(yaw);
        assert!((rz - Vec3::X).length() < 1e-5);
        let ry = Axis::Y.rotated_direction(yaw);
        assert!((ry - Vec3::Y).length() < 1e-5);
    }

    // --- rotated_perpendicular_basis ---

    #[test]
    fn rotated_perpendicular_basis_identity() {
        let (a, b) = rotated_perpendicular_basis(Axis::Y, Quat::IDENTITY);
        assert!((a - Vec3::X).length() < 1e-5);
        assert!((b - Vec3::Z).length() < 1e-5);
    }

    // --- yaw_quat_from_transform ---

    #[test]
    fn yaw_quat_extracts_y_only() {
        let rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_4)
            * Quat::from_rotation_x(0.5);
        let tf = Transform::from_rotation(rotation);
        let yaw_q = yaw_quat_from_transform(&tf);
        let up = yaw_q * Vec3::Y;
        assert!((up - Vec3::Y).length() < 1e-5, "Yaw quat should not tilt Y axis");
    }
}
