use bevy::prelude::*;

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
        Color::srgb(0.2, 1.0, 0.2)
    } else {
        Color::srgb(1.0, 0.8, 0.2)
    };

    let center = preview_pos + state.trigger_offset;
    let size = state.trigger_half_extents * 2.0;
    let transform = Transform::from_translation(center).with_scale(size);

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

    let color = Color::srgb(1.0, 0.4, 0.1);

    let center = preview_pos + state.collision_offset;
    let size = state.collision_half_extents * 2.0;
    let transform = Transform::from_translation(center).with_scale(size);

    gizmos.cube(transform, color);
}

// --- Ground Center Gizmo ---

pub(super) fn draw_ground_gizmo(mut gizmos: Gizmos, state: Res<WorkshopState>) {
    if state.preview_entity.is_none() {
        return;
    }

    // Fixed at the world origin — this is the obstacle's ground center
    // that will be used as the placement anchor in the course editor.
    let ground_pos = Vec3::ZERO;
    let magenta = Color::srgb(1.0, 0.0, 1.0);

    let iso = Isometry3d::new(ground_pos, Quat::IDENTITY);
    gizmos.circle(iso, 0.5, magenta);

    let cross_size = 0.4;
    gizmos.line(
        ground_pos + Vec3::new(-cross_size, 0.0, 0.0),
        ground_pos + Vec3::new(cross_size, 0.0, 0.0),
        magenta,
    );
    gizmos.line(
        ground_pos + Vec3::new(0.0, 0.0, -cross_size),
        ground_pos + Vec3::new(0.0, 0.0, cross_size),
        magenta,
    );
}
