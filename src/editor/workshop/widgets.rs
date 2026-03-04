use bevy::prelude::*;
use std::f32::consts::{PI, TAU};

use crate::camera::orbit::MainCamera;
use crate::palette;

use crate::editor::gizmos::{
    closest_point_on_axis, point_to_segment_distance, ray_intersect_plane,
    rotated_perpendicular_basis, sample_ring_screen_dist, Axis, Sign,
    RING_HIT_THRESHOLD, RING_RADIUS, RING_SAMPLES, ROTATION_STEP_DEG,
};

use crate::editor::undo::{UndoStack, WorkshopAction, WorkshopSnapshot};

use super::{
    EditTarget, MoveDragMode, MoveHoverPart, MoveWidgetState, PreviewObstacle, ResizeWidgetState,
    RotateWidgetState, TransformMode, WorkshopState, ARROW_LENGTH, HANDLE_HIT_THRESHOLD,
    HANDLE_SIZE,
};

const ARROW_HIT_THRESHOLD: f32 = 25.0;
const PLANE_INDICATOR_FRAC: f32 = 0.3;

// --- Transform Mode Keys (1/2/3) ---

pub(super) fn handle_transform_mode_keys(
    mut state: ResMut<WorkshopState>,
    mut move_widget: ResMut<MoveWidgetState>,
    mut rotate_widget: ResMut<RotateWidgetState>,
    mut resize_widget: ResMut<ResizeWidgetState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if state.editing_name {
        return;
    }

    let new_mode = if keyboard.just_pressed(KeyCode::Digit1) {
        Some(TransformMode::Move)
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        Some(TransformMode::Rotate)
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        Some(TransformMode::Resize)
    } else {
        None
    };

    if let Some(mode) = new_mode {
        state.transform_mode = mode;
        move_widget.active_drag = None;
        move_widget.hovered_part = None;
        rotate_widget.active = false;
        rotate_widget.hovered = false;
        resize_widget.active_handle = None;
        resize_widget.hovered_handle = None;
    }
}

// --- Move Widget (XZ Plane + Shift-Y, matching Course Editor) ---

fn move_arrow_origin(
    state: &WorkshopState,
    preview_query: &Query<&Transform, With<PreviewObstacle>>,
) -> Option<Vec3> {
    let entity = state.preview_entity?;
    let transform = preview_query.get(entity).ok()?;
    match state.edit_target {
        EditTarget::Model => Some(transform.translation),
        EditTarget::Trigger => {
            if state.has_trigger {
                Some(transform.translation + state.trigger_offset)
            } else {
                None
            }
        }
        EditTarget::Collision => {
            if state.has_collision {
                Some(transform.translation + state.collision_offset)
            } else {
                None
            }
        }
        EditTarget::Camera => {
            if state.has_camera {
                Some(transform.translation + state.camera_offset)
            } else {
                None
            }
        }
    }
}

pub(super) fn draw_move_arrows(
    mut gizmos: Gizmos,
    state: Res<WorkshopState>,
    widget: Res<MoveWidgetState>,
    preview_query: Query<&Transform, With<PreviewObstacle>>,
) {
    if state.transform_mode != TransformMode::Move {
        return;
    }
    let Some(origin) = move_arrow_origin(&state, &preview_query) else {
        return;
    };

    // Per-axis arrows (R/G/B)
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let is_active = matches!(widget.active_drag, Some(MoveDragMode::SingleAxis(a)) if a == axis);
        let is_hovered = matches!(widget.hovered_part, Some(MoveHoverPart::Arrow(a)) if a == axis);
        let color = axis.color(is_hovered, is_active);
        gizmos.arrow(origin, origin + axis.direction() * ARROW_LENGTH, color);
    }

    // Plane indicator square between X and Z
    let sq_active = matches!(widget.active_drag, Some(MoveDragMode::XzPlane));
    let sq_hovered = matches!(widget.hovered_part, Some(MoveHoverPart::PlaneSquare));
    let sq_brightness = if sq_active {
        1.0
    } else if sq_hovered {
        0.8
    } else {
        0.5
    };
    let Color::Srgba(sq_base) = palette::LIMON else { unreachable!() };
    let sq_color = Color::srgb(sq_base.red * sq_brightness, sq_base.green * sq_brightness, sq_base.blue * sq_brightness);
    let sq = ARROW_LENGTH * PLANE_INDICATOR_FRAC;
    gizmos.line(
        origin + Vec3::X * sq,
        origin + Vec3::X * sq + Vec3::Z * sq,
        sq_color,
    );
    gizmos.line(
        origin + Vec3::Z * sq,
        origin + Vec3::X * sq + Vec3::Z * sq,
        sq_color,
    );
}

pub(super) fn handle_move_widget(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut preview_query: Query<&mut Transform, With<PreviewObstacle>>,
    mut state: ResMut<WorkshopState>,
    mut widget: ResMut<MoveWidgetState>,
    interaction_query: Query<&Interaction>,
    mut undo_stack: ResMut<UndoStack<WorkshopAction>>,
) {
    if state.transform_mode != TransformMode::Move {
        widget.active_drag = None;
        widget.hovered_part = None;
        return;
    }
    let Some(preview_entity) = state.preview_entity else {
        widget.active_drag = None;
        widget.hovered_part = None;
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok((camera, camera_gt)) = camera_query.single() else {
        return;
    };
    let Ok(mut preview_transform) = preview_query.get_mut(preview_entity) else {
        return;
    };

    // Determine arrow origin based on edit target
    let origin = match state.edit_target {
        EditTarget::Model => preview_transform.translation,
        EditTarget::Trigger => {
            if !state.has_trigger {
                widget.active_drag = None;
                widget.hovered_part = None;
                return;
            }
            preview_transform.translation + state.trigger_offset
        }
        EditTarget::Collision => {
            if !state.has_collision {
                widget.active_drag = None;
                widget.hovered_part = None;
                return;
            }
            preview_transform.translation + state.collision_offset
        }
        EditTarget::Camera => {
            if !state.has_camera {
                widget.active_drag = None;
                widget.hovered_part = None;
                return;
            }
            preview_transform.translation + state.camera_offset
        }
    };

    let mouse_over_ui = interaction_query.iter().any(|i| *i != Interaction::None);

    let Ok(ray) = camera.viewport_to_world(camera_gt, cursor_pos) else {
        return;
    };

    if let Some(drag_mode) = widget.active_drag {
        if mouse_buttons.pressed(MouseButton::Left) {
            let new_offset = match drag_mode {
                MoveDragMode::XzPlane => {
                    ray_intersect_plane(
                        ray,
                        Vec3::new(0.0, widget.start_offset.y, 0.0),
                        Vec3::Y,
                    )
                    .map(|hit| {
                        let delta = hit - widget.drag_anchor;
                        widget.start_offset + Vec3::new(delta.x, 0.0, delta.z)
                    })
                }
                MoveDragMode::SingleAxis(axis) => {
                    let axis_dir = axis.direction();
                    let t = closest_point_on_axis(ray, widget.start_offset, axis_dir);
                    let anchor_t = widget.drag_anchor.dot(axis_dir);
                    Some(widget.start_offset + axis_dir * (t - anchor_t))
                }
            };
            if let Some(offset) = new_offset {
                match state.edit_target {
                    EditTarget::Model => {
                        preview_transform.translation = offset;
                        state.model_offset = offset;
                    }
                    EditTarget::Trigger => {
                        state.trigger_offset = offset;
                    }
                    EditTarget::Collision => {
                        state.collision_offset = offset;
                    }
                    EditTarget::Camera => {
                        state.camera_offset = offset;
                    }
                }
            }
        } else {
            // Drag ended — push undo
            if let Some(before) = widget.snapshot_before.take() {
                let after = WorkshopSnapshot::capture(&state);
                undo_stack.push(WorkshopAction::StateChange { before, after });
            }
            widget.active_drag = None;
        }
    } else {
        // Hover detection: find closest arrow or plane square
        let arrows = [
            (Axis::X, origin + Vec3::X * ARROW_LENGTH),
            (Axis::Y, origin + Vec3::Y * ARROW_LENGTH),
            (Axis::Z, origin + Vec3::Z * ARROW_LENGTH),
        ];

        let mut best_part: Option<MoveHoverPart> = None;
        let mut best_dist = ARROW_HIT_THRESHOLD;

        for (axis, end) in &arrows {
            let Ok(ss) = camera.world_to_viewport(camera_gt, origin) else {
                continue;
            };
            let Ok(se) = camera.world_to_viewport(camera_gt, *end) else {
                continue;
            };
            let dist = point_to_segment_distance(cursor_pos, ss, se);
            if dist < best_dist {
                best_dist = dist;
                best_part = Some(MoveHoverPart::Arrow(*axis));
            }
        }

        // Check plane indicator square (two edges between X and Z)
        let sq = ARROW_LENGTH * PLANE_INDICATOR_FRAC;
        let sq_edges = [
            (origin + Vec3::X * sq, origin + Vec3::X * sq + Vec3::Z * sq),
            (origin + Vec3::Z * sq, origin + Vec3::X * sq + Vec3::Z * sq),
        ];
        for (a, b) in &sq_edges {
            let Ok(sa) = camera.world_to_viewport(camera_gt, *a) else {
                continue;
            };
            let Ok(sb) = camera.world_to_viewport(camera_gt, *b) else {
                continue;
            };
            let dist = point_to_segment_distance(cursor_pos, sa, sb);
            if dist < best_dist {
                best_dist = dist;
                best_part = Some(MoveHoverPart::PlaneSquare);
            }
        }

        widget.hovered_part = best_part;

        if !mouse_over_ui
            && mouse_buttons.just_pressed(MouseButton::Left)
            && let Some(part) = widget.hovered_part
        {
            let mode = match part {
                MoveHoverPart::PlaneSquare => MoveDragMode::XzPlane,
                MoveHoverPart::Arrow(axis) => MoveDragMode::SingleAxis(axis),
            };

            // Capture the current offset for this edit target
            let current_offset = match state.edit_target {
                EditTarget::Model => preview_transform.translation,
                EditTarget::Trigger => state.trigger_offset,
                EditTarget::Collision => state.collision_offset,
                EditTarget::Camera => state.camera_offset,
            };

            widget.start_offset = current_offset;
            widget.snapshot_before = Some(WorkshopSnapshot::capture(&state));
            match mode {
                MoveDragMode::XzPlane => {
                    if let Some(hit) = ray_intersect_plane(ray, origin, Vec3::Y) {
                        widget.drag_anchor = hit;
                        widget.active_drag = Some(mode);
                    }
                }
                MoveDragMode::SingleAxis(axis) => {
                    let axis_dir = axis.direction();
                    let t = closest_point_on_axis(ray, origin, axis_dir);
                    widget.drag_anchor = axis_dir * t;
                    widget.active_drag = Some(mode);
                }
            }
        }
    }
}

// --- Rotate Gizmo (Trigger / Collision volumes only) ---

fn rotation_axis_from_modifiers(keyboard: &ButtonInput<KeyCode>) -> Axis {
    if keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight) {
        Axis::X
    } else if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
        Axis::Z
    } else {
        Axis::Y
    }
}

/// Returns the world-space center and current rotation for the active volume edit target.
fn volume_rotate_params(
    state: &WorkshopState,
    preview_query: &Query<&Transform, With<PreviewObstacle>>,
) -> Option<(Vec3, Quat)> {
    let entity = state.preview_entity?;
    let transform = preview_query.get(entity).ok()?;
    match state.edit_target {
        EditTarget::Trigger if state.has_trigger => Some((
            transform.translation + state.trigger_offset,
            state.trigger_rotation,
        )),
        EditTarget::Collision if state.has_collision => Some((
            transform.translation + state.collision_offset,
            state.collision_rotation,
        )),
        EditTarget::Camera if state.has_camera => Some((
            transform.translation + state.camera_offset,
            state.camera_rotation,
        )),
        _ => None,
    }
}

pub(super) fn draw_rotate_gizmo(
    mut gizmos: Gizmos,
    state: Res<WorkshopState>,
    widget: Res<RotateWidgetState>,
    preview_query: Query<&Transform, With<PreviewObstacle>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if state.transform_mode != TransformMode::Rotate {
        return;
    }
    let Some((pos, current_rotation)) = volume_rotate_params(&state, &preview_query) else {
        return;
    };

    let display_axis = if widget.active {
        widget.active_axis
    } else {
        rotation_axis_from_modifiers(&keyboard)
    };

    let is_hovered = !widget.active && widget.hovered;
    let color = display_axis.color(is_hovered, widget.active);

    let yaw_quat = Quat::from_rotation_y(current_rotation.to_euler(EulerRot::YXZ).0);
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
    mut state: ResMut<WorkshopState>,
    mut widget: ResMut<RotateWidgetState>,
    preview_query: Query<&Transform, With<PreviewObstacle>>,
    interaction_query: Query<&Interaction>,
    mut undo_stack: ResMut<UndoStack<WorkshopAction>>,
) {
    if state.transform_mode != TransformMode::Rotate {
        widget.active = false;
        widget.hovered = false;
        return;
    }
    let Some((pos, current_rotation)) = volume_rotate_params(&state, &preview_query) else {
        widget.active = false;
        widget.hovered = false;
        return;
    };

    let Ok(window) = windows.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let Ok((camera, camera_gt)) = camera_query.single() else { return };
    let mouse_over_ui = interaction_query.iter().any(|i| *i != Interaction::None);

    if widget.active {
        if mouse_buttons.pressed(MouseButton::Left) {
            if let Ok(center_screen) = camera.world_to_viewport(camera_gt, pos) {
                let d = cursor_pos - center_screen;
                let current_angle = d.y.atan2(d.x);
                let mut raw_delta = current_angle - widget.drag_start_angle;
                raw_delta = (raw_delta + PI).rem_euclid(TAU) - PI;

                let axis_dir = widget.active_axis.direction();
                let cam_forward = camera_gt.forward().as_vec3();
                if cam_forward.dot(axis_dir) < 0.0 {
                    raw_delta = -raw_delta;
                }

                let step = ROTATION_STEP_DEG.to_radians();
                let snapped_delta = (raw_delta / step).round() * step;

                let new_rotation = Quat::from_axis_angle(axis_dir, snapped_delta)
                    * widget.entity_start_rotation;

                match state.edit_target {
                    EditTarget::Trigger => state.trigger_rotation = new_rotation,
                    EditTarget::Collision => state.collision_rotation = new_rotation,
                    EditTarget::Camera => state.camera_rotation = new_rotation,
                    EditTarget::Model => {}
                }
            }
        } else {
            // Drag ended — push undo
            if let Some(before) = widget.snapshot_before.take() {
                let after = WorkshopSnapshot::capture(&state);
                undo_stack.push(WorkshopAction::StateChange { before, after });
            }
            widget.active = false;
        }
    } else {
        let current_axis = rotation_axis_from_modifiers(&keyboard);
        let yaw_quat = Quat::from_rotation_y(current_rotation.to_euler(EulerRot::YXZ).0);
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

        if !mouse_over_ui
            && mouse_buttons.just_pressed(MouseButton::Left)
            && widget.hovered
            && let Ok(center_screen) = camera.world_to_viewport(camera_gt, pos)
        {
            let d = cursor_pos - center_screen;
            widget.active = true;
            widget.active_axis = current_axis;
            widget.drag_start_angle = d.y.atan2(d.x);
            widget.entity_start_rotation = current_rotation;
            widget.snapshot_before = Some(WorkshopSnapshot::capture(&state));
        }
    }
}

// --- Resize Handles for Trigger / Collision Volume ---

/// Returns the world-space center, half-extents, and rotation for the active volume edit target.
fn volume_box_params(
    state: &WorkshopState,
    preview_query: &Query<&Transform, With<PreviewObstacle>>,
) -> Option<(Vec3, Vec3, Quat)> {
    let entity = state.preview_entity?;
    let transform = preview_query.get(entity).ok()?;
    match state.edit_target {
        EditTarget::Trigger if state.has_trigger => Some((
            transform.translation + state.trigger_offset,
            state.trigger_half_extents,
            state.trigger_rotation,
        )),
        EditTarget::Collision if state.has_collision => Some((
            transform.translation + state.collision_offset,
            state.collision_half_extents,
            state.collision_rotation,
        )),
        _ => None,
    }
}

pub(super) fn draw_resize_handles(
    mut gizmos: Gizmos,
    state: Res<WorkshopState>,
    resize: Res<ResizeWidgetState>,
    preview_query: Query<&Transform, With<PreviewObstacle>>,
) {
    if state.transform_mode != TransformMode::Resize {
        return;
    }
    if !matches!(state.edit_target, EditTarget::Trigger | EditTarget::Collision) {
        return;
    }
    let Some((center, he, rotation)) = volume_box_params(&state, &preview_query) else {
        return;
    };

    for axis in [Axis::X, Axis::Y, Axis::Z] {
        for sign in [Sign::Positive, Sign::Negative] {
            let local_dir = axis.direction()
                * match sign {
                    Sign::Positive => 1.0,
                    Sign::Negative => -1.0,
                };
            let world_dir = rotation * local_dir;
            let extent = match axis {
                Axis::X => he.x,
                Axis::Y => he.y,
                Axis::Z => he.z,
            };
            let pos = center + world_dir * extent;

            let is_hovered = resize.hovered_handle == Some((axis, sign));
            let is_active = resize.active_handle == Some((axis, sign));
            let base = axis.color(is_hovered, is_active);
            let color = if is_active || is_hovered {
                base
            } else {
                axis.color(false, false)
            };

            let transform =
                Transform::from_translation(pos).with_scale(Vec3::splat(HANDLE_SIZE * 2.0));
            gizmos.cube(transform, color);
        }
    }
}

pub(super) fn handle_resize_widget(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut state: ResMut<WorkshopState>,
    mut resize: ResMut<ResizeWidgetState>,
    move_widget: Res<MoveWidgetState>,
    preview_query: Query<&Transform, With<PreviewObstacle>>,
    interaction_query: Query<&Interaction>,
    mut undo_stack: ResMut<UndoStack<WorkshopAction>>,
) {
    if state.transform_mode != TransformMode::Resize {
        resize.hovered_handle = None;
        resize.active_handle = None;
        return;
    }
    if !matches!(state.edit_target, EditTarget::Trigger | EditTarget::Collision) {
        resize.hovered_handle = None;
        resize.active_handle = None;
        return;
    }

    let Some((center, he, rotation)) = volume_box_params(&state, &preview_query) else {
        resize.hovered_handle = None;
        resize.active_handle = None;
        return;
    };

    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok((camera, camera_gt)) = camera_query.single() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_gt, cursor_pos) else {
        return;
    };

    let mouse_over_ui = interaction_query.iter().any(|i| *i != Interaction::None);

    if let Some((active_axis, active_sign)) = resize.active_handle {
        if mouse_buttons.pressed(MouseButton::Left) {
            let world_axis_dir = rotation * active_axis.direction();
            let sign_f = match active_sign {
                Sign::Positive => 1.0,
                Sign::Negative => -1.0,
            };
            let t = closest_point_on_axis(ray, center, world_axis_dir);
            let new_extent = (t * sign_f).max(0.1);
            let target_he = match state.edit_target {
                EditTarget::Collision => &mut state.collision_half_extents,
                _ => &mut state.trigger_half_extents,
            };
            match active_axis {
                Axis::X => target_he.x = new_extent,
                Axis::Y => target_he.y = new_extent,
                Axis::Z => target_he.z = new_extent,
            }
        } else {
            // Drag ended — push undo
            if let Some(before) = resize.snapshot_before.take() {
                let after = WorkshopSnapshot::capture(&state);
                undo_stack.push(WorkshopAction::StateChange { before, after });
            }
            resize.active_handle = None;
        }
    } else {
        // Don't process resize hover/click if move widget is active
        if move_widget.active_drag.is_some() {
            resize.hovered_handle = None;
            return;
        }

        let mut best: Option<(Axis, Sign)> = None;
        let mut best_dist = HANDLE_HIT_THRESHOLD;

        for axis in [Axis::X, Axis::Y, Axis::Z] {
            for sign in [Sign::Positive, Sign::Negative] {
                let local_dir = axis.direction()
                    * match sign {
                        Sign::Positive => 1.0,
                        Sign::Negative => -1.0,
                    };
                let world_dir = rotation * local_dir;
                let extent = match axis {
                    Axis::X => he.x,
                    Axis::Y => he.y,
                    Axis::Z => he.z,
                };
                let handle_pos = center + world_dir * extent;
                let Ok(screen_pos) = camera.world_to_viewport(camera_gt, handle_pos) else {
                    continue;
                };
                let dist = (cursor_pos - screen_pos).length();
                if dist < best_dist {
                    best_dist = dist;
                    best = Some((axis, sign));
                }
            }
        }

        resize.hovered_handle = best;

        if !mouse_over_ui
            && mouse_buttons.just_pressed(MouseButton::Left)
            && let Some((axis, sign)) = best
        {
            resize.snapshot_before = Some(WorkshopSnapshot::capture(&state));
            resize.active_handle = Some((axis, sign));
        }
    }
}
