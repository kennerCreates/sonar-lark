use std::f32::consts::TAU;

use bevy::prelude::*;

use crate::camera::orbit::MainCamera;
use crate::editor::gizmos::{
    closest_point_on_axis, point_to_segment_distance, ray_intersect_plane,
    rotated_perpendicular_basis, yaw_quat_from_transform, Axis,
};

use super::{PlacedFilter, PlacementState, TransformMode};

// --- Widget resources ---

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum MoveDragMode {
    XzPlane,
    YAxis,
}

#[derive(Resource, Default)]
pub(super) struct MoveWidgetState {
    pub(super) active_drag: Option<MoveDragMode>,
    pub(super) hovered: bool,
    drag_anchor: Vec3,
    entity_start_pos: Vec3,
}

#[derive(Resource, Default)]
pub(super) struct RotateWidgetState {
    pub(super) active: bool,
    pub(super) hovered: bool,
    active_axis: Axis,
    drag_start_angle: f32,
    entity_start_rotation: Quat,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ScaleDragMode {
    Uniform,
    PerAxis(Axis),
}

#[derive(Resource, Default)]
pub(super) struct ScaleWidgetState {
    pub(super) active_drag: Option<ScaleDragMode>,
    pub(super) hovered_axis: Option<Axis>,
    pub(super) hovered_center: bool,
    drag_start_t: f32,
    entity_start_scale: Vec3,
}

// --- Constants ---

const ARROW_LENGTH: f32 = 3.75;
const ARROW_HIT_THRESHOLD: f32 = 25.0;

const RING_RADIUS: f32 = 3.0;
const RING_SAMPLES: usize = 32;
const RING_HIT_THRESHOLD: f32 = 15.0;

const SCALE_HANDLE_LENGTH: f32 = 3.75;
const SCALE_CUBE_SIZE: f32 = 0.45;
const SCALE_HIT_THRESHOLD: f32 = 25.0;
const SCALE_SENSITIVITY: f32 = 1.0;

const ROTATION_STEP_DEG: f32 = 5.0;

const PLANE_INDICATOR_FRAC: f32 = 0.3;

// --- Move Gizmo ---

pub(super) fn draw_move_gizmo(
    mut gizmos: Gizmos,
    state: Res<PlacementState>,
    widget: Res<MoveWidgetState>,
    placed_query: Query<&Transform, PlacedFilter>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if state.transform_mode != TransformMode::Move {
        return;
    }
    let Some(entity) = state.selected_entity else {
        return;
    };
    let Ok(transform) = placed_query.get(entity) else {
        return;
    };

    let yaw_quat = yaw_quat_from_transform(transform);
    let shift_held = keyboard.pressed(KeyCode::ShiftLeft)
        || keyboard.pressed(KeyCode::ShiftRight);
    let origin = transform.translation;

    let rot_x = Axis::X.rotated_direction(yaw_quat);
    let rot_z = Axis::Z.rotated_direction(yaw_quat);

    // XZ-plane arrows + square indicator
    let xz_active = matches!(widget.active_drag, Some(MoveDragMode::XzPlane));
    let xz_brightness = if xz_active {
        1.0
    } else if !shift_held && widget.hovered {
        0.8
    } else {
        0.5
    };
    let xz_color = Color::srgb(xz_brightness, xz_brightness, 0.0);
    gizmos.arrow(origin, origin + rot_x * ARROW_LENGTH, xz_color);
    gizmos.arrow(origin, origin + rot_z * ARROW_LENGTH, xz_color);
    // Small square between X and Z to indicate plane movement
    let sq = ARROW_LENGTH * PLANE_INDICATOR_FRAC;
    gizmos.line(origin + rot_x * sq, origin + rot_x * sq + rot_z * sq, xz_color);
    gizmos.line(origin + rot_z * sq, origin + rot_x * sq + rot_z * sq, xz_color);

    // Y-axis arrow (Shift mode)
    let y_active = matches!(widget.active_drag, Some(MoveDragMode::YAxis));
    let y_brightness = if y_active {
        1.0
    } else if shift_held && widget.hovered {
        0.8
    } else {
        0.5
    };
    let y_color = Color::srgb(0.0, y_brightness, 0.0);
    gizmos.arrow(origin, origin + Vec3::Y * ARROW_LENGTH, y_color);
}

pub(super) fn handle_move_gizmo(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    state: Res<PlacementState>,
    mut widget: ResMut<MoveWidgetState>,
    mut placed_query: Query<&mut Transform, PlacedFilter>,
    interaction_query: Query<&Interaction>,
) {
    if state.transform_mode != TransformMode::Move {
        widget.active_drag = None;
        widget.hovered = false;
        return;
    }
    let Some(entity) = state.selected_entity else {
        widget.active_drag = None;
        widget.hovered = false;
        return;
    };
    let Ok(mut transform) = placed_query.get_mut(entity) else {
        return;
    };

    let Ok(window) = windows.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let Ok((camera, camera_gt)) = camera_query.single() else { return };
    let Ok(ray) = camera.viewport_to_world(camera_gt, cursor_pos) else { return };

    let origin = transform.translation;
    let mouse_over_ui = interaction_query.iter().any(|i| *i != Interaction::None);

    if let Some(drag_mode) = widget.active_drag {
        if mouse_buttons.pressed(MouseButton::Left) {
            match drag_mode {
                MoveDragMode::XzPlane => {
                    if let Some(hit) = ray_intersect_plane(
                        ray,
                        Vec3::new(0.0, widget.entity_start_pos.y, 0.0),
                        Vec3::Y,
                    ) {
                        let delta = hit - widget.drag_anchor;
                        transform.translation = widget.entity_start_pos
                            + Vec3::new(delta.x, 0.0, delta.z);
                    }
                }
                MoveDragMode::YAxis => {
                    let t = closest_point_on_axis(ray, widget.entity_start_pos, Vec3::Y);
                    let delta = t - widget.drag_anchor.y;
                    transform.translation = widget.entity_start_pos + Vec3::Y * delta;
                }
            }
        } else {
            widget.active_drag = None;
        }
    } else {
        // Hover detection: check screen distance to all three arrows
        let yaw_quat = yaw_quat_from_transform(&transform);
        let arrows_ends = [
            origin + Axis::X.rotated_direction(yaw_quat) * ARROW_LENGTH,
            origin + Vec3::Y * ARROW_LENGTH,
            origin + Axis::Z.rotated_direction(yaw_quat) * ARROW_LENGTH,
        ];
        let mut near = false;
        for end in arrows_ends {
            let Ok(ss) = camera.world_to_viewport(camera_gt, origin) else {
                continue;
            };
            let Ok(se) = camera.world_to_viewport(camera_gt, end) else {
                continue;
            };
            if point_to_segment_distance(cursor_pos, ss, se) < ARROW_HIT_THRESHOLD {
                near = true;
                break;
            }
        }
        widget.hovered = near;

        if !mouse_over_ui && mouse_buttons.just_pressed(MouseButton::Left) && near {
            let shift_held = keyboard.pressed(KeyCode::ShiftLeft)
                || keyboard.pressed(KeyCode::ShiftRight);
            let mode = if shift_held {
                MoveDragMode::YAxis
            } else {
                MoveDragMode::XzPlane
            };
            widget.entity_start_pos = origin;
            match mode {
                MoveDragMode::XzPlane => {
                    if let Some(hit) = ray_intersect_plane(ray, origin, Vec3::Y) {
                        widget.drag_anchor = hit;
                        widget.active_drag = Some(mode);
                    }
                }
                MoveDragMode::YAxis => {
                    let t = closest_point_on_axis(ray, origin, Vec3::Y);
                    widget.drag_anchor = Vec3::new(0.0, t, 0.0);
                    widget.active_drag = Some(mode);
                }
            }
        }
    }
}

// --- Rotate Gizmo ---

fn rotation_axis_from_modifiers(keyboard: &ButtonInput<KeyCode>) -> Axis {
    if keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight) {
        Axis::X
    } else if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
        Axis::Z
    } else {
        Axis::Y
    }
}

fn angle_in_plane(point: Vec3, center: Vec3, ref1: Vec3, ref2: Vec3) -> f32 {
    let local = point - center;
    local.dot(ref2).atan2(local.dot(ref1))
}

#[cfg(test)]
fn angle_in_ring_plane(point: Vec3, center: Vec3, axis: Axis) -> f32 {
    use crate::editor::gizmos::perpendicular_basis;
    let (ref1, ref2) = perpendicular_basis(axis);
    angle_in_plane(point, center, ref1, ref2)
}

pub(super) fn draw_rotate_gizmo(
    mut gizmos: Gizmos,
    state: Res<PlacementState>,
    widget: Res<RotateWidgetState>,
    placed_query: Query<&Transform, PlacedFilter>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if state.transform_mode != TransformMode::Rotate {
        return;
    }
    let Some(entity) = state.selected_entity else {
        return;
    };
    let Ok(transform) = placed_query.get(entity) else {
        return;
    };

    let pos = transform.translation;
    let yaw_quat = yaw_quat_from_transform(transform);

    let display_axis = if widget.active {
        widget.active_axis
    } else {
        rotation_axis_from_modifiers(&keyboard)
    };

    let is_hovered = !widget.active && widget.hovered;
    let color = display_axis.color(is_hovered, widget.active);

    let face_quat = yaw_quat
        * match display_axis {
            Axis::X => Quat::from_rotation_arc(Vec3::Z, Vec3::X),
            Axis::Y => Quat::from_rotation_arc(Vec3::Z, Vec3::Y),
            Axis::Z => Quat::IDENTITY,
        };
    let iso = Isometry3d::new(pos, face_quat);
    gizmos.circle(iso, RING_RADIUS, color);
}

pub(super) fn handle_rotate_gizmo(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    state: Res<PlacementState>,
    mut widget: ResMut<RotateWidgetState>,
    mut placed_query: Query<&mut Transform, PlacedFilter>,
    interaction_query: Query<&Interaction>,
) {
    if state.transform_mode != TransformMode::Rotate {
        widget.active = false;
        widget.hovered = false;
        return;
    }
    let Some(entity) = state.selected_entity else {
        widget.active = false;
        widget.hovered = false;
        return;
    };
    let Ok(mut transform) = placed_query.get_mut(entity) else {
        return;
    };

    let Ok(window) = windows.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let Ok((camera, camera_gt)) = camera_query.single() else { return };
    let Ok(ray) = camera.viewport_to_world(camera_gt, cursor_pos) else { return };

    let pos = transform.translation;
    let yaw_quat = yaw_quat_from_transform(&transform);
    let mouse_over_ui = interaction_query.iter().any(|i| *i != Interaction::None);

    if widget.active {
        if mouse_buttons.pressed(MouseButton::Left) {
            let axis_dir = widget.active_axis.rotated_direction(yaw_quat);
            if let Some(hit) = ray_intersect_plane(ray, pos, axis_dir) {
                let (ref1, ref2) =
                    rotated_perpendicular_basis(widget.active_axis, yaw_quat);
                let current_angle = angle_in_plane(hit, pos, ref1, ref2);
                let raw_delta = current_angle - widget.drag_start_angle;

                let step = ROTATION_STEP_DEG.to_radians();
                let snapped_delta = (raw_delta / step).round() * step;

                transform.rotation = Quat::from_axis_angle(axis_dir, snapped_delta)
                    * widget.entity_start_rotation;
            }
        } else {
            widget.active = false;
        }
    } else {
        let current_axis = rotation_axis_from_modifiers(&keyboard);
        let (ref1, ref2) = rotated_perpendicular_basis(current_axis, yaw_quat);
        let min_dist = sample_ring_screen_dist(
            camera,
            camera_gt,
            cursor_pos,
            pos,
            ref1,
            ref2,
            RING_RADIUS,
            RING_SAMPLES,
        );
        widget.hovered = min_dist < RING_HIT_THRESHOLD;

        if !mouse_over_ui && mouse_buttons.just_pressed(MouseButton::Left) && widget.hovered
        {
            let axis_dir = current_axis.rotated_direction(yaw_quat);
            if let Some(hit) = ray_intersect_plane(ray, pos, axis_dir) {
                widget.active = true;
                widget.active_axis = current_axis;
                widget.drag_start_angle = angle_in_plane(hit, pos, ref1, ref2);
                widget.entity_start_rotation = transform.rotation;
            }
        }
    }
}

// --- Scale Gizmo ---

pub(super) fn draw_scale_gizmo(
    mut gizmos: Gizmos,
    state: Res<PlacementState>,
    widget: Res<ScaleWidgetState>,
    placed_query: Query<&Transform, PlacedFilter>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if state.transform_mode != TransformMode::Scale {
        return;
    }
    let Some(entity) = state.selected_entity else {
        return;
    };
    let Ok(transform) = placed_query.get(entity) else {
        return;
    };

    let yaw_quat = yaw_quat_from_transform(transform);
    let shift_held = keyboard.pressed(KeyCode::ShiftLeft)
        || keyboard.pressed(KeyCode::ShiftRight);
    let origin = transform.translation;

    // Per-axis lines and cubes
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let is_per_axis_active = matches!(
            widget.active_drag,
            Some(ScaleDragMode::PerAxis(a)) if a == axis
        );
        let is_hovered = shift_held && widget.hovered_axis == Some(axis);
        let color = axis.color(is_hovered, is_per_axis_active);
        let dir = axis.rotated_direction(yaw_quat);
        let tip = origin + dir * SCALE_HANDLE_LENGTH;
        gizmos.line(origin, tip, color);
        let cube_tf = Transform::from_translation(tip)
            .with_rotation(yaw_quat)
            .with_scale(Vec3::splat(SCALE_CUBE_SIZE));
        gizmos.cube(cube_tf, color);
    }

    // Center cube for uniform scale
    let uniform_active = matches!(widget.active_drag, Some(ScaleDragMode::Uniform));
    let uniform_hovered = !shift_held && widget.hovered_center;
    let center_brightness = if uniform_active {
        1.0
    } else if uniform_hovered {
        0.8
    } else {
        0.4
    };
    let center_color = Color::srgb(center_brightness, center_brightness, center_brightness);
    let center_tf =
        Transform::from_translation(origin).with_scale(Vec3::splat(SCALE_CUBE_SIZE * 1.5));
    gizmos.cube(center_tf, center_color);
}

pub(super) fn handle_scale_gizmo(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    state: Res<PlacementState>,
    mut widget: ResMut<ScaleWidgetState>,
    mut placed_query: Query<&mut Transform, PlacedFilter>,
    interaction_query: Query<&Interaction>,
) {
    if state.transform_mode != TransformMode::Scale {
        widget.active_drag = None;
        widget.hovered_axis = None;
        widget.hovered_center = false;
        return;
    }
    let Some(entity) = state.selected_entity else {
        widget.active_drag = None;
        widget.hovered_axis = None;
        widget.hovered_center = false;
        return;
    };
    let Ok(mut transform) = placed_query.get_mut(entity) else {
        return;
    };

    let Ok(window) = windows.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let Ok((camera, camera_gt)) = camera_query.single() else { return };
    let Ok(ray) = camera.viewport_to_world(camera_gt, cursor_pos) else { return };

    let origin = transform.translation;
    let yaw_quat = yaw_quat_from_transform(&transform);
    let mouse_over_ui = interaction_query.iter().any(|i| *i != Interaction::None);

    if let Some(drag_mode) = widget.active_drag {
        if mouse_buttons.pressed(MouseButton::Left) {
            match drag_mode {
                ScaleDragMode::Uniform => {
                    let cam_right = *camera_gt.right();
                    let current_t = closest_point_on_axis(ray, origin, cam_right);
                    let delta = (current_t - widget.drag_start_t) * SCALE_SENSITIVITY;
                    transform.scale =
                        (widget.entity_start_scale + Vec3::splat(delta)).max(Vec3::splat(0.01));
                }
                ScaleDragMode::PerAxis(axis) => {
                    let axis_dir = axis.rotated_direction(yaw_quat);
                    let current_t = closest_point_on_axis(ray, origin, axis_dir);
                    let delta = current_t - widget.drag_start_t;
                    match axis {
                        Axis::X => {
                            transform.scale.x = (widget.entity_start_scale.x
                                + delta * SCALE_SENSITIVITY)
                                .max(0.01);
                        }
                        Axis::Y => {
                            transform.scale.y = (widget.entity_start_scale.y
                                + delta * SCALE_SENSITIVITY)
                                .max(0.01);
                        }
                        Axis::Z => {
                            transform.scale.z = (widget.entity_start_scale.z
                                + delta * SCALE_SENSITIVITY)
                                .max(0.01);
                        }
                    }
                }
            }
        } else {
            widget.active_drag = None;
        }
    } else {
        let shift_held = keyboard.pressed(KeyCode::ShiftLeft)
            || keyboard.pressed(KeyCode::ShiftRight);

        if shift_held {
            // Per-axis hover: find closest axis handle
            widget.hovered_center = false;
            let mut best_axis: Option<Axis> = None;
            let mut best_dist = SCALE_HIT_THRESHOLD;
            for axis in [Axis::X, Axis::Y, Axis::Z] {
                let tip = origin + axis.rotated_direction(yaw_quat) * SCALE_HANDLE_LENGTH;
                let Ok(ss) = camera.world_to_viewport(camera_gt, origin) else {
                    continue;
                };
                let Ok(se) = camera.world_to_viewport(camera_gt, tip) else {
                    continue;
                };
                let dist = point_to_segment_distance(cursor_pos, ss, se);
                if dist < best_dist {
                    best_dist = dist;
                    best_axis = Some(axis);
                }
            }
            widget.hovered_axis = best_axis;

            if !mouse_over_ui
                && mouse_buttons.just_pressed(MouseButton::Left)
                && let Some(axis) = best_axis
            {
                let axis_dir = axis.rotated_direction(yaw_quat);
                widget.active_drag = Some(ScaleDragMode::PerAxis(axis));
                widget.drag_start_t = closest_point_on_axis(ray, origin, axis_dir);
                widget.entity_start_scale = transform.scale;
            }
        } else {
            // Uniform hover: check distance to center
            widget.hovered_axis = None;
            let Ok(screen_origin) = camera.world_to_viewport(camera_gt, origin) else {
                return;
            };
            let dist = (cursor_pos - screen_origin).length();
            widget.hovered_center = dist < SCALE_HIT_THRESHOLD;

            if !mouse_over_ui
                && mouse_buttons.just_pressed(MouseButton::Left)
                && widget.hovered_center
            {
                let cam_right = *camera_gt.right();
                widget.active_drag = Some(ScaleDragMode::Uniform);
                widget.drag_start_t = closest_point_on_axis(ray, origin, cam_right);
                widget.entity_start_scale = transform.scale;
            }
        }
    }
}

// --- Helpers ---

fn sample_ring_screen_dist(
    camera: &Camera,
    camera_gt: &GlobalTransform,
    cursor_pos: Vec2,
    center: Vec3,
    ref1: Vec3,
    ref2: Vec3,
    radius: f32,
    n: usize,
) -> f32 {
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
    use std::f32::consts::{FRAC_PI_2, PI};

    #[test]
    fn angle_along_ref1_is_zero() {
        let angle = angle_in_ring_plane(Vec3::new(5.0, 0.0, 0.0), Vec3::ZERO, Axis::Y);
        assert!(angle.abs() < 1e-5);
    }

    #[test]
    fn angle_along_ref2_is_half_pi() {
        let angle = angle_in_ring_plane(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Axis::Y);
        assert!((angle - FRAC_PI_2).abs() < 1e-5);
    }

    #[test]
    fn angle_along_neg_ref1_is_pi() {
        let angle = angle_in_ring_plane(Vec3::new(-5.0, 0.0, 0.0), Vec3::ZERO, Axis::Y);
        assert!((angle - PI).abs() < 1e-5);
    }

    #[test]
    fn angle_with_offset_center() {
        let center = Vec3::new(10.0, 20.0, 30.0);
        let point = center + Vec3::new(5.0, 0.0, 0.0);
        let angle = angle_in_ring_plane(point, center, Axis::Y);
        assert!(angle.abs() < 1e-5);
    }

    #[test]
    fn angle_x_axis_uses_yz_plane() {
        let angle = angle_in_ring_plane(Vec3::new(0.0, 3.0, 0.0), Vec3::ZERO, Axis::X);
        assert!(angle.abs() < 1e-5);
        let angle = angle_in_ring_plane(Vec3::new(0.0, 0.0, 3.0), Vec3::ZERO, Axis::X);
        assert!((angle - FRAC_PI_2).abs() < 1e-5);
    }

    #[test]
    fn angle_z_axis_uses_xy_plane() {
        let angle = angle_in_ring_plane(Vec3::new(3.0, 0.0, 0.0), Vec3::ZERO, Axis::Z);
        assert!(angle.abs() < 1e-5);
        let angle = angle_in_ring_plane(Vec3::new(0.0, 3.0, 0.0), Vec3::ZERO, Axis::Z);
        assert!((angle - FRAC_PI_2).abs() < 1e-5);
    }

    #[test]
    fn angle_ignores_axis_component() {
        let a1 = angle_in_ring_plane(Vec3::new(5.0, 0.0, 0.0), Vec3::ZERO, Axis::Y);
        let a2 = angle_in_ring_plane(Vec3::new(5.0, 100.0, 0.0), Vec3::ZERO, Axis::Y);
        assert!((a1 - a2).abs() < 1e-5);
    }
}
