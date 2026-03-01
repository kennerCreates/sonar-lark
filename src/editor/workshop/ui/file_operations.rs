use bevy::prelude::*;

use crate::obstacle::definition::{CollisionVolumeConfig, ObstacleDef, ObstacleId, TriggerVolumeConfig};
use crate::obstacle::library::{save_obstacle_library, ObstacleLibrary};
use crate::palette;
use crate::states::{AppState, EditorMode};

use super::build::*;
use crate::editor::workshop::{PreviewObstacle, WorkshopState};

pub fn handle_save_button(
    mut commands: Commands,
    state: Res<WorkshopState>,
    query: Query<&Interaction, (Changed<Interaction>, With<SaveButton>)>,
    mut library: ResMut<ObstacleLibrary>,
    library_container: Query<Entity, With<LibraryListContainer>>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if state.obstacle_name.is_empty() || state.node_name.is_empty() {
            warn!("Cannot save: obstacle name and a selected object are required");
            return;
        }

        let trigger_volume = if state.has_trigger {
            Some(TriggerVolumeConfig {
                // Store offset in ground-anchor space so spawn_obstacle places it correctly.
                // trigger_offset is model-relative; adding model_offset converts to anchor space.
                offset: state.model_offset + state.trigger_offset,
                half_extents: state.trigger_half_extents,
                forward: Vec3::NEG_Z,
            })
        } else {
            None
        };

        let collision_volume = if state.has_collision {
            Some(CollisionVolumeConfig {
                // Store offset in ground-anchor space (model-relative + model_offset).
                offset: state.model_offset + state.collision_offset,
                half_extents: state.collision_half_extents,
            })
        } else {
            None
        };

        let def = ObstacleDef {
            id: ObstacleId(state.obstacle_name.clone()),
            glb_node_name: state.node_name.clone(),
            trigger_volume,
            is_gate: state.is_gate,
            model_offset: state.model_offset,
            collision_volume,
        };

        library.insert(def);
        save_obstacle_library(&library);
        info!("Saved obstacle '{}'", state.obstacle_name);

        rebuild_library_list(&mut commands, &library, &library_container);
    }
}

pub fn handle_new_button(
    mut commands: Commands,
    mut state: ResMut<WorkshopState>,
    query: Query<&Interaction, (Changed<Interaction>, With<NewButton>)>,
    preview_query: Query<Entity, With<PreviewObstacle>>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        for entity in &preview_query {
            commands.entity(entity).despawn();
        }

        let nodes = std::mem::take(&mut state.available_nodes);
        let nodes_loaded = state.nodes_loaded;
        *state = WorkshopState {
            available_nodes: nodes,
            nodes_loaded,
            ..default()
        };
    }
}

pub fn handle_delete_button(
    mut commands: Commands,
    mut state: ResMut<WorkshopState>,
    query: Query<&Interaction, (Changed<Interaction>, With<DeleteButton>)>,
    mut library: ResMut<ObstacleLibrary>,
    preview_query: Query<Entity, With<PreviewObstacle>>,
    library_container: Query<Entity, With<LibraryListContainer>>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if state.obstacle_name.is_empty() {
            return;
        }

        let id = ObstacleId(state.obstacle_name.clone());
        if library.definitions.remove(&id).is_some() {
            save_obstacle_library(&library);
            info!("Deleted obstacle '{}'", state.obstacle_name);

            for entity in &preview_query {
                commands.entity(entity).despawn();
            }

            let nodes = std::mem::take(&mut state.available_nodes);
            let nodes_loaded = state.nodes_loaded;
            *state = WorkshopState {
                available_nodes: nodes,
                nodes_loaded,
                ..default()
            };

            rebuild_library_list(&mut commands, &library, &library_container);
        }
    }
}

pub fn handle_back_button(
    query: Query<&Interaction, (Changed<Interaction>, With<BackButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Menu);
        }
    }
}

pub fn handle_switch_to_course_editor(
    query: Query<&Interaction, (Changed<Interaction>, With<SwitchToCourseEditorButton>)>,
    mut next_state: ResMut<NextState<EditorMode>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(EditorMode::CourseEditor);
        }
    }
}

fn rebuild_library_list(
    commands: &mut Commands,
    library: &ObstacleLibrary,
    container_query: &Query<Entity, With<LibraryListContainer>>,
) {
    let Ok(container) = container_query.single() else {
        return;
    };

    commands.entity(container).despawn_related::<Children>();
    commands.entity(container).with_children(|parent| {
        if library.definitions.is_empty() {
            parent.spawn((
                Text::new("No obstacles defined"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(palette::CHAINMAIL),
            ));
        } else {
            let mut ids: Vec<&ObstacleId> = library.definitions.keys().collect();
            ids.sort_by(|a, b| a.0.cmp(&b.0));
            for id in ids {
                spawn_library_button(parent, &id.0);
            }
        }
    });
}
