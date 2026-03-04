use bevy::prelude::*;

use crate::editor::undo::{UndoStack, WorkshopAction};
use crate::obstacle::definition::ObstacleId;
use crate::obstacle::library::ObstacleLibrary;
use crate::ui_theme;

use super::build::*;
use crate::editor::workshop::{CollisionVolumeData, PreviewObstacle, WorkshopState};

pub fn handle_node_selection(
    mut commands: Commands,
    mut state: ResMut<WorkshopState>,
    query: Query<(&Interaction, &NodeButton), Changed<Interaction>>,
    preview_query: Query<Entity, With<PreviewObstacle>>,
    mut undo_stack: ResMut<UndoStack<WorkshopAction>>,
) {
    for (interaction, node_btn) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        state.node_name = node_btn.0.clone();
        if state.obstacle_name.is_empty() {
            state.obstacle_name = node_btn.0.to_lowercase().replace(' ', "_");
        }

        for entity in &preview_query {
            commands.entity(entity).despawn();
        }
        state.preview_entity = None;
        undo_stack.clear();
    }
}

pub fn handle_library_selection(
    mut commands: Commands,
    mut state: ResMut<WorkshopState>,
    query: Query<(&Interaction, &LibraryButton), Changed<Interaction>>,
    library: Res<ObstacleLibrary>,
    preview_query: Query<Entity, With<PreviewObstacle>>,
    mut undo_stack: ResMut<UndoStack<WorkshopAction>>,
) {
    for (interaction, lib_btn) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let id = ObstacleId(lib_btn.0.clone());
        let Some(def) = library.get(&id) else {
            continue;
        };

        state.obstacle_name = def.id.0.clone();
        state.node_name = def.glb_node_name.clone();
        state.is_gate = def.is_gate;
        state.has_trigger = def.trigger_volume.is_some();
        state.model_offset = def.model_offset;
        state.model_rotation = def.model_rotation;
        // Stored offset is in ground-anchor space; convert to model-relative for editing.
        if let Some(trigger) = &def.trigger_volume {
            state.trigger_offset = trigger.offset - def.model_offset;
            state.trigger_half_extents = trigger.half_extents;
            state.trigger_rotation = trigger.rotation;
        }
        state.has_collision = !def.collision_volumes.is_empty();
        state.collision_volumes = def
            .collision_volumes
            .iter()
            .map(|c| CollisionVolumeData {
                offset: c.offset - def.model_offset,
                half_extents: c.half_extents,
                rotation: c.rotation,
            })
            .collect();
        state.active_collision_idx = 0;
        state.load_active_from_vec();

        state.has_camera = def.default_camera.is_some();
        if let Some(cam) = &def.default_camera {
            state.camera_offset = cam.offset - def.model_offset;
            state.camera_rotation = cam.rotation;
        } else {
            state.camera_offset = Vec3::new(0.0, 2.0, -5.0);
            state.camera_rotation = Quat::IDENTITY;
        }

        for entity in &preview_query {
            commands.entity(entity).despawn();
        }
        state.preview_entity = None;
        undo_stack.clear();
    }
}

pub fn handle_button_hover(
    mut query: Query<
        (&Interaction, &mut BackgroundColor),
        (
            Changed<Interaction>,
            Or<(
                With<NodeButton>,
                With<LibraryButton>,
                With<BackButton>,
            )>,
        ),
    >,
) {
    for (interaction, mut bg) in &mut query {
        ui_theme::apply_button_bg(interaction, &mut bg);
    }
}
