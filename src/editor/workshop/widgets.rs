use bevy::prelude::*;

use crate::camera::orbit::MainCamera;

use crate::editor::gizmos::{closest_point_on_axis, point_to_segment_distance, Axis, Sign};

use super::{
    EditTarget, MoveWidgetState, PreviewObstacle, ResizeWidgetState, WorkshopState,
    ARROW_HIT_THRESHOLD, ARROW_LENGTH, HANDLE_HIT_THRESHOLD, HANDLE_SIZE,
};

// --- Move Widget (3D Axis Arrows) ---

fn move_arrow_origin(state: &WorkshopState, preview_query: &Query<&Transform, With<PreviewObstacle>>) -> Option<Vec3> {
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
    }
}

pub(super) fn draw_move_arrows(
    mut gizmos: Gizmos,
    state: Res<WorkshopState>,
    widget: Res<MoveWidgetState>,
    preview_query: Query<&Transform, With<PreviewObstacle>>,
) {
    let Some(origin) = move_arrow_origin(&state, &preview_query) else {
        return;
    };

    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let is_hovered = widget.hovered_axis == Some(axis);
        let is_active = widget.active_axis == Some(axis);
        let color = axis.color(is_hovered, is_active);

        let end = origin + axis.direction() * ARROW_LENGTH;
        gizmos.arrow(origin, end, color);
    }
}

pub(super) fn handle_move_widget(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut preview_query: Query<&mut Transform, With<PreviewObstacle>>,
    mut state: ResMut<WorkshopState>,
    mut widget: ResMut<MoveWidgetState>,
    interaction_query: Query<&Interaction>,
) {
    let Some(preview_entity) = state.preview_entity else {
        widget.hovered_axis = None;
        widget.active_axis = None;
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
                widget.hovered_axis = None;
                widget.active_axis = None;
                return;
            }
            preview_transform.translation + state.trigger_offset
        }
        EditTarget::Collision => {
            if !state.has_collision {
                widget.hovered_axis = None;
                widget.active_axis = None;
                return;
            }
            preview_transform.translation + state.collision_offset
        }
    };

    let mouse_over_ui = interaction_query.iter().any(|i| *i != Interaction::None);

    let Ok(ray) = camera.viewport_to_world(camera_gt, cursor_pos) else {
        return;
    };

    if let Some(active_axis) = widget.active_axis {
        if mouse_buttons.pressed(MouseButton::Left) {
            let axis_dir = active_axis.direction();
            let t = closest_point_on_axis(ray, origin, axis_dir);
            let delta = t - widget.drag_offset;

            match state.edit_target {
                EditTarget::Model => {
                    let new_pos = origin + axis_dir * delta;
                    preview_transform.translation = new_pos;
                    state.model_offset = new_pos;
                }
                EditTarget::Trigger => {
                    state.trigger_offset += axis_dir * delta;
                }
                EditTarget::Collision => {
                    state.collision_offset += axis_dir * delta;
                }
            }
        } else {
            widget.active_axis = None;
        }
    } else {
        let mut best_axis: Option<Axis> = None;
        let mut best_dist = ARROW_HIT_THRESHOLD;

        for axis in [Axis::X, Axis::Y, Axis::Z] {
            let end = origin + axis.direction() * ARROW_LENGTH;

            let Ok(screen_start) = camera.world_to_viewport(camera_gt, origin) else {
                continue;
            };
            let Ok(screen_end) = camera.world_to_viewport(camera_gt, end) else {
                continue;
            };

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

// --- Resize Handles for Trigger / Collision Volume ---

/// Returns the world-space center and half-extents for the active volume edit target.
fn volume_box_params(
    state: &WorkshopState,
    preview_query: &Query<&Transform, With<PreviewObstacle>>,
) -> Option<(Vec3, Vec3)> {
    let entity = state.preview_entity?;
    let transform = preview_query.get(entity).ok()?;
    match state.edit_target {
        EditTarget::Trigger if state.has_trigger => {
            Some((transform.translation + state.trigger_offset, state.trigger_half_extents))
        }
        EditTarget::Collision if state.has_collision => {
            Some((transform.translation + state.collision_offset, state.collision_half_extents))
        }
        _ => None,
    }
}

pub(super) fn draw_resize_handles(
    mut gizmos: Gizmos,
    state: Res<WorkshopState>,
    resize: Res<ResizeWidgetState>,
    preview_query: Query<&Transform, With<PreviewObstacle>>,
) {
    if !matches!(state.edit_target, EditTarget::Trigger | EditTarget::Collision) {
        return;
    }
    let Some((center, he)) = volume_box_params(&state, &preview_query) else {
        return;
    };

    for axis in [Axis::X, Axis::Y, Axis::Z] {
        for sign in [Sign::Positive, Sign::Negative] {
            let dir = axis.direction() * match sign {
                Sign::Positive => 1.0,
                Sign::Negative => -1.0,
            };
            let extent = match axis {
                Axis::X => he.x,
                Axis::Y => he.y,
                Axis::Z => he.z,
            };
            let pos = center + dir * extent;

            let is_hovered = resize.hovered_handle == Some((axis, sign));
            let is_active = resize.active_handle == Some((axis, sign));
            let base = axis.color(is_hovered, is_active);
            // Lighten the color for resize handles to distinguish from move arrows
            let color = if is_active || is_hovered {
                base
            } else {
                match axis {
                    Axis::X => Color::srgb(0.7, 0.3, 0.3),
                    Axis::Y => Color::srgb(0.3, 0.7, 0.3),
                    Axis::Z => Color::srgb(0.3, 0.3, 0.7),
                }
            };

            let transform = Transform::from_translation(pos).with_scale(Vec3::splat(HANDLE_SIZE * 2.0));
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
) {
    if !matches!(state.edit_target, EditTarget::Trigger | EditTarget::Collision) {
        resize.hovered_handle = None;
        resize.active_handle = None;
        return;
    }

    let Some((center, he)) = volume_box_params(&state, &preview_query) else {
        resize.hovered_handle = None;
        resize.active_handle = None;
        return;
    };

    let Ok(window) = windows.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let Ok((camera, camera_gt)) = camera_query.single() else { return };
    let Ok(ray) = camera.viewport_to_world(camera_gt, cursor_pos) else { return };

    let mouse_over_ui = interaction_query.iter().any(|i| *i != Interaction::None);

    if let Some((active_axis, active_sign)) = resize.active_handle {
        if mouse_buttons.pressed(MouseButton::Left) {
            let axis_dir = active_axis.direction();
            let sign_f = match active_sign {
                Sign::Positive => 1.0,
                Sign::Negative => -1.0,
            };
            // Project cursor ray onto the axis to get new extent
            let t = closest_point_on_axis(ray, center, axis_dir);
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
            resize.active_handle = None;
        }
    } else {
        // Don't process resize hover/click if move widget is active
        if move_widget.active_axis.is_some() {
            resize.hovered_handle = None;
            return;
        }

        let mut best: Option<(Axis, Sign)> = None;
        let mut best_dist = HANDLE_HIT_THRESHOLD;

        for axis in [Axis::X, Axis::Y, Axis::Z] {
            for sign in [Sign::Positive, Sign::Negative] {
                let dir = axis.direction() * match sign {
                    Sign::Positive => 1.0,
                    Sign::Negative => -1.0,
                };
                let extent = match axis {
                    Axis::X => he.x,
                    Axis::Y => he.y,
                    Axis::Z => he.z,
                };
                let handle_pos = center + dir * extent;
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
            resize.active_handle = Some((axis, sign));
        }
    }
}
