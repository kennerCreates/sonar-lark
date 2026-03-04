use bevy::prelude::*;

use crate::camera::orbit::MainCamera;
use std::f32::consts::{PI, TAU};

use crate::editor::gizmos::{
    rotated_perpendicular_basis, sample_ring_screen_dist, yaw_quat_from_transform, Axis,
    RING_HIT_THRESHOLD, RING_RADIUS, RING_SAMPLES, ROTATION_STEP_DEG,
};

use crate::editor::course_editor::{EditorSelection, EditorTransform, PlacedFilter, TransformMode};

use super::RotateWidgetState;

fn rotation_axis_from_modifiers(keyboard: &ButtonInput<KeyCode>) -> Axis {
    if keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight) {
        Axis::X
    } else if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
        Axis::Z
    } else {
        Axis::Y
    }
}

#[cfg(test)]
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

pub(in crate::editor::course_editor) fn draw_rotate_gizmo(
    mut gizmos: Gizmos,
    transform_state: Res<EditorTransform>,
    selection: Res<EditorSelection>,
    widget: Res<RotateWidgetState>,
    placed_query: Query<(&Transform, &GlobalTransform), PlacedFilter>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if transform_state.mode != TransformMode::Rotate {
        return;
    }
    let Some(entity) = selection.entity else {
        return;
    };
    let Ok((transform, global_transform)) = placed_query.get(entity) else {
        return;
    };

    let pos = global_transform.translation();
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

pub(in crate::editor::course_editor) fn handle_rotate_gizmo(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    transform_state: Res<EditorTransform>,
    selection: Res<EditorSelection>,
    mut widget: ResMut<RotateWidgetState>,
    mut placed_query: Query<(&mut Transform, &GlobalTransform), PlacedFilter>,
    interaction_query: Query<&Interaction>,
    child_of_query: Query<&ChildOf>,
    parent_gt_query: Query<&GlobalTransform>,
) {
    if transform_state.mode != TransformMode::Rotate {
        widget.active = false;
        widget.hovered = false;
        return;
    }
    let Some(entity) = selection.entity else {
        widget.active = false;
        widget.hovered = false;
        return;
    };
    let Ok((mut transform, global_transform)) = placed_query.get_mut(entity) else {
        return;
    };

    let Ok(window) = windows.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let Ok((camera, camera_gt)) = camera_query.single() else { return };
    let pos = global_transform.translation();
    let yaw_quat = yaw_quat_from_transform(&transform);
    let mouse_over_ui = interaction_query.iter().any(|i| *i != Interaction::None);

    if widget.active {
        if mouse_buttons.pressed(MouseButton::Left) {
            if let Ok(center_screen) = camera.world_to_viewport(camera_gt, pos) {
                let d = cursor_pos - center_screen;
                let current_angle = d.y.atan2(d.x);
                let mut raw_delta = current_angle - widget.drag_start_angle;
                // Normalize to [-PI, PI] to handle wrapping at ±180°.
                raw_delta = (raw_delta + PI).rem_euclid(TAU) - PI;

                let axis_dir = widget.active_axis.rotated_direction(widget.start_yaw_quat);
                let cam_forward = camera_gt.forward().as_vec3();
                if cam_forward.dot(axis_dir) < 0.0 {
                    raw_delta = -raw_delta;
                }

                let step = ROTATION_STEP_DEG.to_radians();
                let snapped_delta = (raw_delta / step).round() * step;

                let world_rotation = Quat::from_axis_angle(axis_dir, snapped_delta)
                    * widget.entity_start_rotation;

                // Convert world rotation to local if entity has a parent
                if let Ok(child_of) = child_of_query.get(entity) {
                    if let Ok(parent_gt) = parent_gt_query.get(child_of.parent()) {
                        let parent_rot = parent_gt.to_scale_rotation_translation().1;
                        transform.rotation = parent_rot.inverse() * world_rotation;
                    } else {
                        transform.rotation = world_rotation;
                    }
                } else {
                    transform.rotation = world_rotation;
                }
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
            && let Ok(center_screen) = camera.world_to_viewport(camera_gt, pos)
        {
            let d = cursor_pos - center_screen;
            widget.active = true;
            widget.active_axis = current_axis;
            widget.drag_start_angle = d.y.atan2(d.x);
            widget.start_yaw_quat = yaw_quat;
            // Store the world-space rotation at drag start
            let (_, world_rot, _) = global_transform.to_scale_rotation_translation();
            widget.entity_start_rotation = world_rot;
        }
    }
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
