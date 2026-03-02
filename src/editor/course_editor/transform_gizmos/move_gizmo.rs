use bevy::prelude::*;

use crate::camera::orbit::MainCamera;
use crate::editor::gizmos::{
    closest_point_on_axis, point_to_segment_distance, ray_intersect_plane,
    yaw_quat_from_transform, Axis,
};

use crate::editor::course_editor::{EditorSelection, EditorTransform, PlacedFilter, TransformMode};

use super::{
    MoveDragMode, MoveWidgetState, ARROW_HIT_THRESHOLD, ARROW_LENGTH, PLANE_INDICATOR_FRAC,
};

pub(in crate::editor::course_editor) fn draw_move_gizmo(
    mut gizmos: Gizmos,
    transform_state: Res<EditorTransform>,
    selection: Res<EditorSelection>,
    widget: Res<MoveWidgetState>,
    placed_query: Query<(&Transform, &GlobalTransform), PlacedFilter>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if transform_state.mode != TransformMode::Move {
        return;
    }
    let Some(entity) = selection.entity else {
        return;
    };
    let Ok((transform, global_transform)) = placed_query.get(entity) else {
        return;
    };

    let yaw_quat = yaw_quat_from_transform(transform);
    let shift_held = keyboard.pressed(KeyCode::ShiftLeft)
        || keyboard.pressed(KeyCode::ShiftRight);
    let origin = global_transform.translation();

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

/// Convert a world-space position to local space if the entity has a parent.
fn to_local_position(
    world_pos: Vec3,
    entity: Entity,
    child_of_query: &Query<&ChildOf>,
    parent_gt_query: &Query<&GlobalTransform>,
) -> Vec3 {
    if let Ok(child_of) = child_of_query.get(entity)
        && let Ok(parent_gt) = parent_gt_query.get(child_of.parent())
    {
        return parent_gt.affine().inverse().transform_point3(world_pos);
    }
    world_pos
}

pub(in crate::editor::course_editor) fn handle_move_gizmo(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    transform_state: Res<EditorTransform>,
    selection: Res<EditorSelection>,
    mut widget: ResMut<MoveWidgetState>,
    mut placed_query: Query<(&mut Transform, &GlobalTransform), PlacedFilter>,
    interaction_query: Query<&Interaction>,
    child_of_query: Query<&ChildOf>,
    parent_gt_query: Query<&GlobalTransform>,
) {
    if transform_state.mode != TransformMode::Move {
        widget.active_drag = None;
        widget.hovered = false;
        return;
    }
    let Some(entity) = selection.entity else {
        widget.active_drag = None;
        widget.hovered = false;
        return;
    };
    let Ok((mut transform, global_transform)) = placed_query.get_mut(entity) else {
        return;
    };

    let Ok(window) = windows.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let Ok((camera, camera_gt)) = camera_query.single() else { return };
    let Ok(ray) = camera.viewport_to_world(camera_gt, cursor_pos) else { return };

    let origin = global_transform.translation();
    let mouse_over_ui = interaction_query.iter().any(|i| *i != Interaction::None);

    if let Some(drag_mode) = widget.active_drag {
        if mouse_buttons.pressed(MouseButton::Left) {
            let new_world_pos = match drag_mode {
                MoveDragMode::XzPlane => {
                    ray_intersect_plane(
                        ray,
                        Vec3::new(0.0, widget.entity_start_pos.y, 0.0),
                        Vec3::Y,
                    )
                    .map(|hit| {
                        let delta = hit - widget.drag_anchor;
                        widget.entity_start_pos + Vec3::new(delta.x, 0.0, delta.z)
                    })
                }
                MoveDragMode::YAxis => {
                    let t = closest_point_on_axis(ray, widget.entity_start_pos, Vec3::Y);
                    let delta = t - widget.drag_anchor.y;
                    Some(widget.entity_start_pos + Vec3::Y * delta)
                }
            };
            if let Some(world_pos) = new_world_pos {
                transform.translation =
                    to_local_position(world_pos, entity, &child_of_query, &parent_gt_query);
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
