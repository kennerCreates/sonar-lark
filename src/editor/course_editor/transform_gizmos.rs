use std::f32::consts::TAU;

use bevy::prelude::*;

use crate::editor::gizmos::{
    closest_point_on_axis, perpendicular_basis, point_to_segment_distance, ray_intersect_plane,
    Axis,
};

use super::{PlacedFilter, PlacementState, TransformMode};

// --- Widget resources ---

#[derive(Resource, Default)]
pub(super) struct MoveWidgetState {
    pub(super) active_axis: Option<Axis>,
    pub(super) hovered_axis: Option<Axis>,
    drag_offset: f32,
}

#[derive(Resource, Default)]
pub(super) struct RotateWidgetState {
    pub(super) active_axis: Option<Axis>,
    pub(super) hovered_axis: Option<Axis>,
    drag_start_angle: f32,
    entity_start_rotation: Quat,
}

#[derive(Resource, Default)]
pub(super) struct ScaleWidgetState {
    pub(super) active_axis: Option<Axis>,
    pub(super) hovered_axis: Option<Axis>,
    drag_start_t: f32,
    entity_start_scale: Vec3,
}

// --- Constants ---

const ARROW_LENGTH: f32 = 2.5;
const ARROW_HIT_THRESHOLD: f32 = 25.0;

const RING_RADIUS: f32 = 2.0;
const RING_SAMPLES: usize = 32;
const RING_HIT_THRESHOLD: f32 = 15.0;

const SCALE_HANDLE_LENGTH: f32 = 2.5;
const SCALE_CUBE_SIZE: f32 = 0.3;
const SCALE_HIT_THRESHOLD: f32 = 25.0;
const SCALE_SENSITIVITY: f32 = 1.0;

// --- Move Gizmo ---

pub(super) fn draw_move_gizmo(
    mut gizmos: Gizmos,
    state: Res<PlacementState>,
    widget: Res<MoveWidgetState>,
    placed_query: Query<&Transform, PlacedFilter>,
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

    let origin = transform.translation;
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let is_hovered = widget.hovered_axis == Some(axis);
        let is_active = widget.active_axis == Some(axis);
        let color = axis.color(is_hovered, is_active);
        let end = origin + axis.direction() * ARROW_LENGTH;
        gizmos.arrow(origin, end, color);
    }
}

pub(super) fn handle_move_gizmo(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    state: Res<PlacementState>,
    mut widget: ResMut<MoveWidgetState>,
    mut placed_query: Query<&mut Transform, PlacedFilter>,
    interaction_query: Query<&Interaction>,
) {
    if state.transform_mode != TransformMode::Move {
        widget.active_axis = None;
        widget.hovered_axis = None;
        return;
    }
    let Some(entity) = state.selected_entity else {
        widget.active_axis = None;
        widget.hovered_axis = None;
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

    if let Some(active_axis) = widget.active_axis {
        if mouse_buttons.pressed(MouseButton::Left) {
            let axis_dir = active_axis.direction();
            let t = closest_point_on_axis(ray, origin, axis_dir);
            let delta = t - widget.drag_offset;
            transform.translation = origin + axis_dir * delta;
        } else {
            widget.active_axis = None;
        }
    } else {
        let mut best_axis: Option<Axis> = None;
        let mut best_dist = ARROW_HIT_THRESHOLD;

        for axis in [Axis::X, Axis::Y, Axis::Z] {
            let end = origin + axis.direction() * ARROW_LENGTH;
            let Ok(screen_start) = camera.world_to_viewport(camera_gt, origin) else { continue };
            let Ok(screen_end) = camera.world_to_viewport(camera_gt, end) else { continue };
            let dist = point_to_segment_distance(cursor_pos, screen_start, screen_end);
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
            let axis_dir = axis.direction();
            let t = closest_point_on_axis(ray, origin, axis_dir);
            widget.active_axis = Some(axis);
            widget.drag_offset = t;
        }
    }
}

// --- Rotate Gizmo ---

pub(super) fn draw_rotate_gizmo(
    mut gizmos: Gizmos,
    state: Res<PlacementState>,
    widget: Res<RotateWidgetState>,
    placed_query: Query<&Transform, PlacedFilter>,
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

    let ring_orientations = [
        (Axis::X, Quat::from_rotation_arc(Vec3::Z, Vec3::X)),
        (Axis::Y, Quat::from_rotation_arc(Vec3::Z, Vec3::Y)),
        (Axis::Z, Quat::IDENTITY),
    ];

    for (axis, face_quat) in ring_orientations {
        let is_hovered = widget.hovered_axis == Some(axis);
        let is_active = widget.active_axis == Some(axis);
        let color = axis.color(is_hovered, is_active);
        let iso = Isometry3d::new(pos, face_quat);
        gizmos.circle(iso, RING_RADIUS, color);
    }
}

pub(super) fn handle_rotate_gizmo(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    state: Res<PlacementState>,
    mut widget: ResMut<RotateWidgetState>,
    mut placed_query: Query<&mut Transform, PlacedFilter>,
    interaction_query: Query<&Interaction>,
) {
    if state.transform_mode != TransformMode::Rotate {
        widget.active_axis = None;
        widget.hovered_axis = None;
        return;
    }
    let Some(entity) = state.selected_entity else {
        widget.active_axis = None;
        widget.hovered_axis = None;
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
    let mouse_over_ui = interaction_query.iter().any(|i| *i != Interaction::None);

    if let Some(active_axis) = widget.active_axis {
        if mouse_buttons.pressed(MouseButton::Left) {
            let axis_dir = active_axis.direction();
            if let Some(hit) = ray_intersect_plane(ray, pos, axis_dir) {
                let current_angle = angle_in_ring_plane(hit, pos, active_axis);
                let delta = current_angle - widget.drag_start_angle;
                transform.rotation =
                    widget.entity_start_rotation * Quat::from_axis_angle(axis_dir, delta);
            }
        } else {
            widget.active_axis = None;
        }
    } else {
        let mut best_axis: Option<Axis> = None;
        let mut best_dist = RING_HIT_THRESHOLD;

        for axis in [Axis::X, Axis::Y, Axis::Z] {
            let (ref1, ref2) = perpendicular_basis(axis);
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
            if min_dist < best_dist {
                best_dist = min_dist;
                best_axis = Some(axis);
            }
        }

        widget.hovered_axis = best_axis;

        if !mouse_over_ui
            && mouse_buttons.just_pressed(MouseButton::Left)
            && let Some(axis) = best_axis
        {
            let axis_dir = axis.direction();
            if let Some(hit) = ray_intersect_plane(ray, pos, axis_dir) {
                widget.active_axis = Some(axis);
                widget.drag_start_angle = angle_in_ring_plane(hit, pos, axis);
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

    let origin = transform.translation;
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let is_hovered = widget.hovered_axis == Some(axis);
        let is_active = widget.active_axis == Some(axis);
        let color = axis.color(is_hovered, is_active);
        let tip = origin + axis.direction() * SCALE_HANDLE_LENGTH;
        gizmos.line(origin, tip, color);
        let cube_transform =
            Transform::from_translation(tip).with_scale(Vec3::splat(SCALE_CUBE_SIZE));
        gizmos.cube(cube_transform, color);
    }
}

pub(super) fn handle_scale_gizmo(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    state: Res<PlacementState>,
    mut widget: ResMut<ScaleWidgetState>,
    mut placed_query: Query<&mut Transform, PlacedFilter>,
    interaction_query: Query<&Interaction>,
) {
    if state.transform_mode != TransformMode::Scale {
        widget.active_axis = None;
        widget.hovered_axis = None;
        return;
    }
    let Some(entity) = state.selected_entity else {
        widget.active_axis = None;
        widget.hovered_axis = None;
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

    if let Some(active_axis) = widget.active_axis {
        if mouse_buttons.pressed(MouseButton::Left) {
            let axis_dir = active_axis.direction();
            let current_t = closest_point_on_axis(ray, origin, axis_dir);
            let delta = current_t - widget.drag_start_t;
            match active_axis {
                Axis::X => {
                    transform.scale.x =
                        (widget.entity_start_scale.x + delta * SCALE_SENSITIVITY).max(0.01)
                }
                Axis::Y => {
                    transform.scale.y =
                        (widget.entity_start_scale.y + delta * SCALE_SENSITIVITY).max(0.01)
                }
                Axis::Z => {
                    transform.scale.z =
                        (widget.entity_start_scale.z + delta * SCALE_SENSITIVITY).max(0.01)
                }
            }
        } else {
            widget.active_axis = None;
        }
    } else {
        let mut best_axis: Option<Axis> = None;
        let mut best_dist = SCALE_HIT_THRESHOLD;

        for axis in [Axis::X, Axis::Y, Axis::Z] {
            let tip = origin + axis.direction() * SCALE_HANDLE_LENGTH;
            let Ok(screen_start) = camera.world_to_viewport(camera_gt, origin) else { continue };
            let Ok(screen_end) = camera.world_to_viewport(camera_gt, tip) else { continue };
            let dist = point_to_segment_distance(cursor_pos, screen_start, screen_end);
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
            let axis_dir = axis.direction();
            widget.active_axis = Some(axis);
            widget.drag_start_t = closest_point_on_axis(ray, origin, axis_dir);
            widget.entity_start_scale = transform.scale;
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

fn angle_in_ring_plane(point: Vec3, center: Vec3, axis: Axis) -> f32 {
    let local = point - center;
    let (ref1, ref2) = perpendicular_basis(axis);
    local.dot(ref2).atan2(local.dot(ref1))
}
