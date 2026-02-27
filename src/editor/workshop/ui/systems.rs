use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use crate::obstacle::definition::{CollisionVolumeConfig, ObstacleDef, ObstacleId, TriggerVolumeConfig};
use crate::obstacle::library::{save_obstacle_library, ObstacleLibrary};
use crate::palette;
use crate::states::{AppState, EditorMode};

use super::build::*;
use crate::editor::workshop::{EditTarget, PreviewObstacle, WorkshopState};

// --- Interaction Systems ---

pub fn handle_node_selection(
    mut commands: Commands,
    mut state: ResMut<WorkshopState>,
    query: Query<(&Interaction, &NodeButton), Changed<Interaction>>,
    preview_query: Query<Entity, With<PreviewObstacle>>,
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
    }
}

pub fn handle_library_selection(
    mut commands: Commands,
    mut state: ResMut<WorkshopState>,
    query: Query<(&Interaction, &LibraryButton), Changed<Interaction>>,
    library: Res<ObstacleLibrary>,
    preview_query: Query<Entity, With<PreviewObstacle>>,
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
        // Stored offset is in ground-anchor space; convert to model-relative for editing.
        if let Some(trigger) = &def.trigger_volume {
            state.trigger_offset = trigger.offset - def.model_offset;
            state.trigger_half_extents = trigger.half_extents;
        }
        state.has_collision = def.collision_volume.is_some();
        if let Some(collision) = &def.collision_volume {
            state.collision_offset = collision.offset - def.model_offset;
            state.collision_half_extents = collision.half_extents;
        }

        for entity in &preview_query {
            commands.entity(entity).despawn();
        }
        state.preview_entity = None;
    }
}

pub fn handle_trigger_toggle(
    mut state: ResMut<WorkshopState>,
    query: Query<&Interaction, (Changed<Interaction>, With<HasTriggerToggle>)>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            state.has_trigger = !state.has_trigger;
        }
    }
}

pub fn handle_collision_toggle(
    mut state: ResMut<WorkshopState>,
    query: Query<&Interaction, (Changed<Interaction>, With<HasCollisionToggle>)>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            state.has_collision = !state.has_collision;
            if state.has_collision && state.collision_half_extents == Vec3::ZERO {
                state.collision_half_extents = Vec3::new(3.0, 3.0, 1.5);
            }
        }
    }
}

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

pub fn handle_name_field_focus(
    mut state: ResMut<WorkshopState>,
    query: Query<&Interaction, (Changed<Interaction>, With<NameFieldButton>)>,
    mut border: Query<&mut BorderColor, With<NameFieldButton>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            state.editing_name = true;
            if let Ok(mut b) = border.single_mut() {
                *b = BorderColor::all(palette::SKY);
            }
        }
    }
}

pub fn handle_name_text_input(
    mut state: ResMut<WorkshopState>,
    mut events: MessageReader<KeyboardInput>,
    mut border: Query<&mut BorderColor, With<NameFieldButton>>,
) {
    if !state.editing_name {
        return;
    }

    for event in events.read() {
        if !event.state.is_pressed() {
            continue;
        }

        match &event.logical_key {
            Key::Enter | Key::Escape => {
                state.editing_name = false;
                if let Ok(mut b) = border.single_mut() {
                    *b = BorderColor::all(palette::STEEL);
                }
            }
            Key::Backspace => {
                state.obstacle_name.pop();
            }
            Key::Space => {
                state.obstacle_name.push('_');
            }
            Key::Character(c) => {
                for ch in c.chars() {
                    if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                        state.obstacle_name.push(ch.to_ascii_lowercase());
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn handle_edit_target_toggle(
    mut state: ResMut<WorkshopState>,
    model_query: Query<&Interaction, (Changed<Interaction>, With<EditTargetRadioModel>)>,
    trigger_query: Query<&Interaction, (Changed<Interaction>, With<EditTargetRadioTrigger>)>,
    collision_query: Query<&Interaction, (Changed<Interaction>, With<EditTargetRadioCollision>)>,
) {
    for interaction in &model_query {
        if *interaction == Interaction::Pressed {
            state.edit_target = EditTarget::Model;
        }
    }
    for interaction in &trigger_query {
        if *interaction == Interaction::Pressed {
            state.edit_target = EditTarget::Trigger;
        }
    }
    for interaction in &collision_query {
        if *interaction == Interaction::Pressed {
            state.edit_target = EditTarget::Collision;
        }
    }
}

pub fn update_display_values(
    state: Res<WorkshopState>,
    mut name_text: Query<&mut Text, (With<NameDisplayText>, Without<HasTriggerText>, Without<HasCollisionText>)>,
    mut trigger_text: Query<&mut Text, (With<HasTriggerText>, Without<NameDisplayText>, Without<HasCollisionText>)>,
    mut collision_text: Query<&mut Text, (With<HasCollisionText>, Without<NameDisplayText>, Without<HasTriggerText>)>,
    mut bgs: ParamSet<(
        Query<&mut BackgroundColor, With<HasTriggerToggle>>,
        Query<&mut BackgroundColor, With<HasCollisionToggle>>,
        Query<&mut BackgroundColor, With<EditTargetRadioModel>>,
        Query<&mut BackgroundColor, With<EditTargetRadioTrigger>>,
        Query<&mut BackgroundColor, With<EditTargetRadioCollision>>,
    )>,
) {
    if !state.is_changed() {
        return;
    }

    if let Ok(mut text) = name_text.single_mut() {
        let display = if state.obstacle_name.is_empty() {
            if state.editing_name {
                "|".to_string()
            } else {
                "(type a name)".to_string()
            }
        } else if state.editing_name {
            format!("{}|", state.obstacle_name)
        } else {
            state.obstacle_name.clone()
        };
        **text = display;
    }

    if let Ok(mut text) = trigger_text.single_mut() {
        **text = if state.has_trigger { "ON" } else { "OFF" }.to_string();
    }
    if let Ok(mut text) = collision_text.single_mut() {
        **text = if state.has_collision { "ON" } else { "OFF" }.to_string();
    }

    if let Ok(mut bg) = bgs.p0().single_mut() {
        *bg = BackgroundColor(if state.has_trigger { TOGGLE_ON } else { TOGGLE_OFF });
    }
    if let Ok(mut bg) = bgs.p1().single_mut() {
        *bg = BackgroundColor(if state.has_collision { TOGGLE_ON } else { TOGGLE_OFF });
    }

    if let Ok(mut bg) = bgs.p2().single_mut() {
        *bg = BackgroundColor(if state.edit_target == EditTarget::Model { RADIO_ACTIVE } else { RADIO_INACTIVE });
    }
    if let Ok(mut bg) = bgs.p3().single_mut() {
        *bg = BackgroundColor(if state.edit_target == EditTarget::Trigger { RADIO_ACTIVE } else { RADIO_INACTIVE });
    }
    if let Ok(mut bg) = bgs.p4().single_mut() {
        *bg = BackgroundColor(if state.edit_target == EditTarget::Collision { RADIO_ACTIVE } else { RADIO_INACTIVE });
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
                With<SwitchToCourseEditorButton>,
            )>,
        ),
    >,
) {
    for (interaction, mut bg) in &mut query {
        match *interaction {
            Interaction::Pressed => *bg = BackgroundColor(BUTTON_PRESSED),
            Interaction::Hovered => *bg = BackgroundColor(BUTTON_HOVERED),
            Interaction::None => *bg = BackgroundColor(BUTTON_NORMAL),
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
