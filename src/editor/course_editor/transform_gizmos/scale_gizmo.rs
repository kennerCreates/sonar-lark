use bevy::prelude::*;

use crate::camera::orbit::MainCamera;
use crate::editor::gizmos::{
    closest_point_on_axis, point_to_segment_distance, yaw_quat_from_transform, Axis,
};

use crate::editor::course_editor::{
    EditorSelection, EditorTransform, PlacedCamera, PlacedFilter, TransformMode,
};

use super::{
    ScaleDragMode, ScaleWidgetState, SCALE_CUBE_SIZE, SCALE_HANDLE_LENGTH, SCALE_HIT_THRESHOLD,
    SCALE_SENSITIVITY,
};

pub(in crate::editor::course_editor) fn draw_scale_gizmo(
    mut gizmos: Gizmos,
    transform_state: Res<EditorTransform>,
    selection: Res<EditorSelection>,
    widget: Res<ScaleWidgetState>,
    placed_query: Query<&Transform, PlacedFilter>,
    camera_check: Query<(), With<PlacedCamera>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if transform_state.mode != TransformMode::Scale {
        return;
    }
    let Some(entity) = selection.entity else {
        return;
    };
    // Skip scale gizmo for cameras
    if camera_check.get(entity).is_ok() {
        return;
    }
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

pub(in crate::editor::course_editor) fn handle_scale_gizmo(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    transform_state: Res<EditorTransform>,
    selection: Res<EditorSelection>,
    mut widget: ResMut<ScaleWidgetState>,
    mut placed_query: Query<&mut Transform, PlacedFilter>,
    interaction_query: Query<&Interaction>,
    camera_check: Query<(), With<PlacedCamera>>,
) {
    if transform_state.mode != TransformMode::Scale {
        widget.active_drag = None;
        widget.hovered_axis = None;
        widget.hovered_center = false;
        return;
    }
    let Some(entity) = selection.entity else {
        widget.active_drag = None;
        widget.hovered_axis = None;
        widget.hovered_center = false;
        return;
    };
    // Skip scale gizmo for cameras
    if camera_check.get(entity).is_ok() {
        widget.active_drag = None;
        widget.hovered_axis = None;
        widget.hovered_center = false;
        return;
    }
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
