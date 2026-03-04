use bevy::prelude::*;

use crate::camera::orbit::MainCamera;
use crate::editor::gizmos::{
    closest_point_on_axis, point_to_segment_distance, ray_intersect_plane,
    yaw_quat_from_transform, Axis,
};
use crate::palette;

use crate::editor::course_editor::{EditorSelection, EditorTransform, PlacedFilter, TransformMode};
use crate::editor::undo::{CourseEditorAction, UndoStack};

use super::{
    MoveDragMode, MoveHoverPart, MoveWidgetState, ARROW_HIT_THRESHOLD, ARROW_LENGTH,
    PLANE_INDICATOR_FRAC,
};

pub(in crate::editor::course_editor) fn draw_move_gizmo(
    mut gizmos: Gizmos,
    transform_state: Res<EditorTransform>,
    selection: Res<EditorSelection>,
    widget: Res<MoveWidgetState>,
    placed_query: Query<(&Transform, &GlobalTransform), PlacedFilter>,
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
    let origin = global_transform.translation();
    let rot_x = Axis::X.rotated_direction(yaw_quat);
    let rot_z = Axis::Z.rotated_direction(yaw_quat);

    // Per-axis arrows (R/G/B)
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let is_active = matches!(widget.active_drag, Some(MoveDragMode::SingleAxis(a)) if a == axis);
        let is_hovered = matches!(widget.hovered_part, Some(MoveHoverPart::Arrow(a)) if a == axis);
        let color = axis.color(is_hovered, is_active);
        let end = match axis {
            Axis::Y => origin + Vec3::Y * ARROW_LENGTH,
            _ => origin + axis.rotated_direction(yaw_quat) * ARROW_LENGTH,
        };
        gizmos.arrow(origin, end, color);
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
    gizmos.line(origin + rot_x * sq, origin + rot_x * sq + rot_z * sq, sq_color);
    gizmos.line(origin + rot_z * sq, origin + rot_x * sq + rot_z * sq, sq_color);
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
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    transform_state: Res<EditorTransform>,
    selection: Res<EditorSelection>,
    mut widget: ResMut<MoveWidgetState>,
    mut placed_query: Query<(&mut Transform, &GlobalTransform), PlacedFilter>,
    interaction_query: Query<&Interaction>,
    child_of_query: Query<&ChildOf>,
    parent_gt_query: Query<&GlobalTransform>,
    mut undo_stack: ResMut<UndoStack<CourseEditorAction>>,
) {
    if transform_state.mode != TransformMode::Move {
        widget.active_drag = None;
        widget.hovered_part = None;
        return;
    }
    let Some(entity) = selection.entity else {
        widget.active_drag = None;
        widget.hovered_part = None;
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
    let yaw_quat = yaw_quat_from_transform(&transform);
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
                MoveDragMode::SingleAxis(axis) => {
                    let axis_dir = if axis == Axis::Y {
                        Vec3::Y
                    } else {
                        axis.rotated_direction(yaw_quat)
                    };
                    let t = closest_point_on_axis(ray, widget.entity_start_pos, axis_dir);
                    let anchor_t = widget.drag_anchor.dot(axis_dir);
                    Some(widget.entity_start_pos + axis_dir * (t - anchor_t))
                }
            };
            if let Some(world_pos) = new_world_pos {
                transform.translation =
                    to_local_position(world_pos, entity, &child_of_query, &parent_gt_query);
            }
        } else {
            // Drag ended — push undo if transform changed
            if let Some(entity) = selection.entity
                && let Ok((current_transform, _)) = placed_query.get(entity)
                && *current_transform != widget.entity_start_transform
            {
                undo_stack.push(CourseEditorAction::TransformChange {
                    entity,
                    before: widget.entity_start_transform,
                    after: *current_transform,
                });
            }
            widget.active_drag = None;
        }
    } else {
        // Hover detection: find closest arrow or plane square
        let rot_x = Axis::X.rotated_direction(yaw_quat);
        let rot_z = Axis::Z.rotated_direction(yaw_quat);
        let arrows = [
            (Axis::X, origin + rot_x * ARROW_LENGTH),
            (Axis::Y, origin + Vec3::Y * ARROW_LENGTH),
            (Axis::Z, origin + rot_z * ARROW_LENGTH),
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
            (origin + rot_x * sq, origin + rot_x * sq + rot_z * sq),
            (origin + rot_z * sq, origin + rot_x * sq + rot_z * sq),
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
            widget.entity_start_pos = origin;
            widget.entity_start_transform = *transform;
            match mode {
                MoveDragMode::XzPlane => {
                    if let Some(hit) = ray_intersect_plane(ray, origin, Vec3::Y) {
                        widget.drag_anchor = hit;
                        widget.active_drag = Some(mode);
                    }
                }
                MoveDragMode::SingleAxis(axis) => {
                    let axis_dir = if axis == Axis::Y {
                        Vec3::Y
                    } else {
                        axis.rotated_direction(yaw_quat)
                    };
                    let t = closest_point_on_axis(ray, origin, axis_dir);
                    widget.drag_anchor = axis_dir * t;
                    widget.active_drag = Some(mode);
                }
            }
        }
    }
}
