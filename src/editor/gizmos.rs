use bevy::prelude::*;

// --- Axis ---

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Axis {
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

    pub(crate) fn color(self, hovered: bool, active: bool) -> Color {
        let brightness = if active {
            1.0
        } else if hovered {
            0.8
        } else {
            0.5
        };
        match self {
            Axis::X => Color::srgb(brightness, 0.0, 0.0),
            Axis::Y => Color::srgb(0.0, brightness, 0.0),
            Axis::Z => Color::srgb(0.0, 0.0, brightness),
        }
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
