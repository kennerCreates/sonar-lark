use bevy::prelude::*;

use crate::palette;

use super::{PreviewObstacle, TriggerGizmoGroup, WorkshopState};

pub(super) fn draw_trigger_gizmo(
    mut gizmos: Gizmos<TriggerGizmoGroup>,
    state: Res<WorkshopState>,
    preview_query: Query<&Transform, With<PreviewObstacle>>,
) {
    if !state.has_trigger || state.node_name.is_empty() {
        return;
    }

    let preview_pos = state
        .preview_entity
        .and_then(|e| preview_query.get(e).ok())
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    let color = if state.is_gate {
        palette::GREEN
    } else {
        palette::SUNSHINE
    };

    let center = preview_pos + state.trigger_offset;
    let size = state.trigger_half_extents * 2.0;
    let transform = Transform {
        translation: center,
        rotation: state.trigger_rotation,
        scale: size,
    };

    gizmos.cube(transform, color);
}

pub(super) fn draw_collision_gizmo(
    mut gizmos: Gizmos<TriggerGizmoGroup>,
    state: Res<WorkshopState>,
    preview_query: Query<&Transform, With<PreviewObstacle>>,
) {
    if !state.has_collision || state.node_name.is_empty() {
        return;
    }

    let preview_pos = state
        .preview_entity
        .and_then(|e| preview_query.get(e).ok())
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    // Draw non-active shapes in a dim color
    for (i, vol) in state.collision_volumes.iter().enumerate() {
        if i == state.active_collision_idx {
            continue;
        }
        let center = preview_pos + vol.offset;
        let size = vol.half_extents * 2.0;
        let transform = Transform {
            translation: center,
            rotation: vol.rotation,
            scale: size,
        };
        gizmos.cube(transform, palette::CLAY);
    }

    // Draw the active shape in bright orange (uses working-copy fields)
    let center = preview_pos + state.collision_offset;
    let size = state.collision_half_extents * 2.0;
    let transform = Transform {
        translation: center,
        rotation: state.collision_rotation,
        scale: size,
    };
    gizmos.cube(transform, palette::TANGERINE);
}

pub(super) fn draw_camera_gizmo(
    mut gizmos: Gizmos<TriggerGizmoGroup>,
    state: Res<WorkshopState>,
    preview_query: Query<&Transform, With<PreviewObstacle>>,
) {
    if !state.has_camera || state.node_name.is_empty() {
        return;
    }

    let preview_pos = state
        .preview_entity
        .and_then(|e| preview_query.get(e).ok())
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    let pos = preview_pos + state.camera_offset;
    let rotation = state.camera_rotation;
    let forward = rotation * Vec3::NEG_Z;
    let right = rotation * Vec3::X;
    let up = rotation * Vec3::Y;
    let color = palette::SKY;

    let iso = Isometry3d::new(pos, Quat::IDENTITY);
    gizmos.sphere(iso, 0.35, color);

    // Frustum wireframe (16:9 aspect, matching course editor)
    let dist = 2.0;
    let half_h = 0.6;
    let half_w = half_h * 1.778;

    let center = pos + forward * dist;
    let corners = [
        center + right * half_w + up * half_h,
        center - right * half_w + up * half_h,
        center - right * half_w - up * half_h,
        center + right * half_w - up * half_h,
    ];

    for i in 0..4 {
        gizmos.line(corners[i], corners[(i + 1) % 4], color);
        gizmos.line(pos, corners[i], color);
    }

    // Up indicator
    gizmos.arrow(pos, pos + up * 0.8, color);
}

// --- Ground Center Gizmo ---

pub(super) fn draw_ground_gizmo(mut gizmos: Gizmos, state: Res<WorkshopState>) {
    if state.preview_entity.is_none() {
        return;
    }

    // Fixed at the world origin — this is the obstacle's ground center
    // that will be used as the placement anchor in the course editor.
    let ground_pos = Vec3::ZERO;
    let iso = Isometry3d::new(ground_pos, Quat::IDENTITY);
    gizmos.circle(iso, 0.5, palette::MAGENTA);

    let cross_size = 0.4;
    gizmos.line(
        ground_pos + Vec3::new(-cross_size, 0.0, 0.0),
        ground_pos + Vec3::new(cross_size, 0.0, 0.0),
        palette::MAGENTA,
    );
    gizmos.line(
        ground_pos + Vec3::new(0.0, 0.0, -cross_size),
        ground_pos + Vec3::new(0.0, 0.0, cross_size),
        palette::MAGENTA,
    );
}
