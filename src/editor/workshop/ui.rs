use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use crate::obstacle::definition::{ObstacleDef, ObstacleId, TriggerVolumeConfig};
use crate::obstacle::library::{save_obstacle_library, ObstacleLibrary};
use crate::states::{AppState, EditorMode};

use super::{PreviewObstacle, WorkshopState};

const PANEL_BG: Color = Color::srgba(0.08, 0.08, 0.08, 0.9);
const BUTTON_NORMAL: Color = Color::srgb(0.15, 0.15, 0.15);
const BUTTON_HOVERED: Color = Color::srgb(0.25, 0.25, 0.25);
const BUTTON_PRESSED: Color = Color::srgb(0.35, 0.75, 0.35);
const TOGGLE_ON: Color = Color::srgb(0.2, 0.6, 0.3);
const TOGGLE_OFF: Color = Color::srgb(0.4, 0.15, 0.15);

// Marker components
#[derive(Component)]
pub struct NodeListContainer;

#[derive(Component)]
pub struct LibraryListContainer;

#[derive(Component)]
pub struct NodeButton(pub String);

#[derive(Component)]
pub struct LibraryButton(pub String);

#[derive(Component)]
pub struct AdjustButton {
    pub field: AdjustField,
    pub delta: f32,
}

#[derive(Clone, Copy)]
pub enum AdjustField {
    OffsetX,
    OffsetY,
    OffsetZ,
    ExtentX,
    ExtentY,
    ExtentZ,
}

#[derive(Component)]
pub struct IsGateToggle;

#[derive(Component)]
pub struct HasTriggerToggle;

#[derive(Component)]
pub struct SaveButton;

#[derive(Component)]
pub struct NewButton;

#[derive(Component)]
pub struct DeleteButton;

#[derive(Component)]
pub struct BackButton;

#[derive(Component)]
pub struct SwitchToCourseEditorButton;

#[derive(Component)]
pub struct IdFieldButton;

#[derive(Component)]
pub struct IdDisplayText;

#[derive(Component)]
pub struct NodeFieldButton;

#[derive(Component)]
pub struct NodeDisplayText;

#[derive(Component)]
pub struct ValueDisplay(pub AdjustField);

#[derive(Component)]
pub struct IsGateText;

#[derive(Component)]
pub struct HasTriggerText;

pub fn build_workshop_ui(commands: &mut Commands, library: &ObstacleLibrary) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            DespawnOnExit(EditorMode::ObstacleWorkshop),
        ))
        .with_children(|root| {
            build_left_panel(root, library);
            build_right_panel(root);
        });
}

fn build_left_panel(parent: &mut ChildSpawnerCommands, library: &ObstacleLibrary) {
    parent
        .spawn((
            Node {
                width: Val::Px(260.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(8.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(PANEL_BG),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("Obstacle Workshop"),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
            ));

            panel.spawn((
                Text::new("glTF Objects"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
                Node {
                    margin: UiRect::top(Val::Px(8.0)),
                    ..default()
                },
            ));

            // Node list container (populated async when glTF loads)
            panel
                .spawn((
                    NodeListContainer,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                ))
                .with_children(|container| {
                    container.spawn((
                        Text::new("Loading glTF..."),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.5, 0.5, 0.5)),
                    ));
                });

            // Divider
            panel.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(1.0),
                    margin: UiRect::vertical(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
            ));

            panel.spawn((
                Text::new("Library"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));

            panel
                .spawn((
                    LibraryListContainer,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                ))
                .with_children(|container| {
                    if library.definitions.is_empty() {
                        container.spawn((
                            Text::new("No obstacles defined"),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.5, 0.5, 0.5)),
                        ));
                    } else {
                        let mut ids: Vec<&ObstacleId> = library.definitions.keys().collect();
                        ids.sort_by(|a, b| a.0.cmp(&b.0));
                        for id in ids {
                            spawn_library_button(container, &id.0);
                        }
                    }
                });

            panel.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });

            spawn_small_button(panel, "Back to Menu", BackButton);
            spawn_small_button(panel, "Course Editor", SwitchToCourseEditorButton);
        });
}

fn build_right_panel(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                width: Val::Px(280.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(6.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(PANEL_BG),
        ))
        .with_children(|panel| {
            // Obstacle ID
            panel.spawn((
                Text::new("Obstacle ID"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ));

            panel
                .spawn((
                    Button,
                    IdFieldButton,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(32.0),
                        padding: UiRect::horizontal(Val::Px(8.0)),
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.05, 0.05, 0.05)),
                    BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
                ))
                .with_children(|field| {
                    field.spawn((
                        Text::new("(type an ID)"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.5, 0.5, 0.5)),
                        IdDisplayText,
                    ));
                });

            // Object name
            panel.spawn((
                Text::new("Object Name (in glb)"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
                Node {
                    margin: UiRect::top(Val::Px(4.0)),
                    ..default()
                },
            ));

            panel
                .spawn((
                    Button,
                    NodeFieldButton,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(32.0),
                        padding: UiRect::horizontal(Val::Px(8.0)),
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.05, 0.05, 0.05)),
                    BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
                ))
                .with_children(|field| {
                    field.spawn((
                        Text::new("(type or select from list)"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.5, 0.5, 0.5)),
                        NodeDisplayText,
                    ));
                });

            spawn_divider(panel);

            // Properties
            panel.spawn((
                Text::new("Properties"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ));

            spawn_toggle_row(panel, "Is Gate", IsGateToggle, IsGateText, true);
            spawn_toggle_row(panel, "Trigger Volume", HasTriggerToggle, HasTriggerText, true);

            spawn_divider(panel);

            // Trigger volume controls
            panel.spawn((
                Text::new("Trigger Offset"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ));

            spawn_adjust_row(panel, "X", AdjustField::OffsetX, 0.0);
            spawn_adjust_row(panel, "Y", AdjustField::OffsetY, 1.0);
            spawn_adjust_row(panel, "Z", AdjustField::OffsetZ, 0.0);

            panel.spawn((
                Text::new("Trigger Half-Extents"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
                Node {
                    margin: UiRect::top(Val::Px(4.0)),
                    ..default()
                },
            ));

            spawn_adjust_row(panel, "X", AdjustField::ExtentX, 2.0);
            spawn_adjust_row(panel, "Y", AdjustField::ExtentY, 2.0);
            spawn_adjust_row(panel, "Z", AdjustField::ExtentZ, 0.5);

            panel.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });

            spawn_divider(panel);
            spawn_action_button(panel, "Save Obstacle", SaveButton, Color::srgb(0.15, 0.4, 0.15));
            spawn_action_button(panel, "New / Clear", NewButton, BUTTON_NORMAL);
            spawn_action_button(panel, "Delete", DeleteButton, Color::srgb(0.5, 0.12, 0.12));
        });
}

pub fn spawn_node_button(parent: &mut ChildSpawnerCommands, name: &str) {
    parent
        .spawn((
            Button,
            NodeButton(name.to_string()),
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(28.0),
                padding: UiRect::horizontal(Val::Px(8.0)),
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(BUTTON_NORMAL),
            BorderColor::all(Color::srgb(0.25, 0.25, 0.25)),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(name),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
        });
}

fn spawn_library_button(parent: &mut ChildSpawnerCommands, id: &str) {
    parent
        .spawn((
            Button,
            LibraryButton(id.to_string()),
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(28.0),
                padding: UiRect::horizontal(Val::Px(8.0)),
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(BUTTON_NORMAL),
            BorderColor::all(Color::srgb(0.25, 0.25, 0.25)),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(id),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.85, 0.7)),
            ));
        });
}

fn spawn_small_button(parent: &mut ChildSpawnerCommands, label: &str, marker: impl Component) {
    parent
        .spawn((
            Button,
            marker,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(32.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(BUTTON_NORMAL),
            BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
        });
}

fn spawn_toggle_row(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    toggle_marker: impl Component,
    text_marker: impl Component,
    initial: bool,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Button,
                toggle_marker,
                Node {
                    width: Val::Px(50.0),
                    height: Val::Px(26.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(if initial { TOGGLE_ON } else { TOGGLE_OFF }),
                BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new(if initial { "ON" } else { "OFF" }),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    text_marker,
                ));
            });

            row.spawn((
                Text::new(label),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
        });
}

fn spawn_adjust_row(
    parent: &mut ChildSpawnerCommands,
    axis: &str,
    field: AdjustField,
    initial: f32,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new(format!("{axis}:")),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
                Node {
                    width: Val::Px(20.0),
                    ..default()
                },
            ));

            spawn_adjust_btn(row, "-", field, -0.1);

            row.spawn((
                Text::new(format!("{initial:.1}")),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                ValueDisplay(field),
                Node {
                    width: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
            ));

            spawn_adjust_btn(row, "+", field, 0.1);
        });
}

fn spawn_adjust_btn(parent: &mut ChildSpawnerCommands, label: &str, field: AdjustField, delta: f32) {
    parent
        .spawn((
            Button,
            AdjustButton { field, delta },
            Node {
                width: Val::Px(28.0),
                height: Val::Px(24.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(BUTTON_NORMAL),
            BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
            ));
        });
}

fn spawn_action_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    marker: impl Component,
    bg: Color,
) {
    parent
        .spawn((
            Button,
            marker,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(36.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                margin: UiRect::vertical(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(bg),
            BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
            ));
        });
}

fn spawn_divider(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(1.0),
            margin: UiRect::vertical(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
    ));
}

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
        if state.obstacle_id.is_empty() {
            state.obstacle_id = node_btn.0.to_lowercase().replace(' ', "_");
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

        state.obstacle_id = def.id.0.clone();
        state.node_name = def.glb_node_name.clone();
        state.is_gate = def.is_gate;
        state.has_trigger = def.trigger_volume.is_some();
        if let Some(trigger) = &def.trigger_volume {
            state.trigger_offset = trigger.offset;
            state.trigger_half_extents = trigger.half_extents;
        }

        for entity in &preview_query {
            commands.entity(entity).despawn();
        }
        state.preview_entity = None;
    }
}

pub fn handle_adjust_buttons(
    mut state: ResMut<WorkshopState>,
    query: Query<(&Interaction, &AdjustButton), Changed<Interaction>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    for (interaction, adjust) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let multiplier = if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
            10.0
        } else {
            1.0
        };
        let delta = adjust.delta * multiplier;

        match adjust.field {
            AdjustField::OffsetX => state.trigger_offset.x += delta,
            AdjustField::OffsetY => state.trigger_offset.y += delta,
            AdjustField::OffsetZ => state.trigger_offset.z += delta,
            AdjustField::ExtentX => {
                state.trigger_half_extents.x = (state.trigger_half_extents.x + delta).max(0.1)
            }
            AdjustField::ExtentY => {
                state.trigger_half_extents.y = (state.trigger_half_extents.y + delta).max(0.1)
            }
            AdjustField::ExtentZ => {
                state.trigger_half_extents.z = (state.trigger_half_extents.z + delta).max(0.1)
            }
        }
    }
}

pub fn handle_is_gate_toggle(
    mut state: ResMut<WorkshopState>,
    query: Query<&Interaction, (Changed<Interaction>, With<IsGateToggle>)>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            state.is_gate = !state.is_gate;
        }
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

        if state.obstacle_id.is_empty() || state.node_name.is_empty() {
            warn!("Cannot save: obstacle ID and object name are required");
            return;
        }

        let trigger_volume = if state.has_trigger {
            Some(TriggerVolumeConfig {
                offset: state.trigger_offset,
                half_extents: state.trigger_half_extents,
            })
        } else {
            None
        };

        let def = ObstacleDef {
            id: ObstacleId(state.obstacle_id.clone()),
            glb_node_name: state.node_name.clone(),
            trigger_volume,
            is_gate: state.is_gate,
        };

        library.insert(def);
        save_obstacle_library(&library);
        info!("Saved obstacle '{}'", state.obstacle_id);

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

        if state.obstacle_id.is_empty() {
            return;
        }

        let id = ObstacleId(state.obstacle_id.clone());
        if library.definitions.remove(&id).is_some() {
            save_obstacle_library(&library);
            info!("Deleted obstacle '{}'", state.obstacle_id);

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

pub fn handle_id_field_focus(
    mut state: ResMut<WorkshopState>,
    id_field_query: Query<&Interaction, (Changed<Interaction>, With<IdFieldButton>)>,
    mut id_field_border: Query<&mut BorderColor, With<IdFieldButton>>,
) {
    for interaction in &id_field_query {
        if *interaction == Interaction::Pressed {
            state.editing_id = true;
            if let Ok(mut border) = id_field_border.single_mut() {
                *border = BorderColor::all(Color::srgb(0.4, 0.7, 1.0));
            }
        }
    }
}

pub fn handle_id_text_input(
    mut state: ResMut<WorkshopState>,
    mut events: MessageReader<KeyboardInput>,
    mut id_field_border: Query<&mut BorderColor, With<IdFieldButton>>,
) {
    if !state.editing_id {
        return;
    }

    for event in events.read() {
        if !event.state.is_pressed() {
            continue;
        }

        match &event.logical_key {
            Key::Enter | Key::Escape => {
                state.editing_id = false;
                if let Ok(mut border) = id_field_border.single_mut() {
                    *border = BorderColor::all(Color::srgb(0.3, 0.3, 0.3));
                }
            }
            Key::Backspace => {
                state.obstacle_id.pop();
            }
            Key::Space => {
                state.obstacle_id.push('_');
            }
            Key::Character(c) => {
                for ch in c.chars() {
                    if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                        state.obstacle_id.push(ch.to_ascii_lowercase());
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn handle_node_field_focus(
    mut state: ResMut<WorkshopState>,
    query: Query<&Interaction, (Changed<Interaction>, With<NodeFieldButton>)>,
    mut border: Query<&mut BorderColor, With<NodeFieldButton>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            state.editing_node = true;
            if let Ok(mut b) = border.single_mut() {
                *b = BorderColor::all(Color::srgb(0.4, 0.7, 1.0));
            }
        }
    }
}

pub fn handle_node_text_input(
    mut commands: Commands,
    mut state: ResMut<WorkshopState>,
    mut events: MessageReader<KeyboardInput>,
    mut border: Query<&mut BorderColor, With<NodeFieldButton>>,
    preview_query: Query<Entity, With<PreviewObstacle>>,
) {
    if !state.editing_node {
        return;
    }

    let mut name_changed = false;
    for event in events.read() {
        if !event.state.is_pressed() {
            continue;
        }

        match &event.logical_key {
            Key::Enter | Key::Escape => {
                state.editing_node = false;
                if let Ok(mut b) = border.single_mut() {
                    *b = BorderColor::all(Color::srgb(0.3, 0.3, 0.3));
                }
                name_changed = true;
            }
            Key::Backspace => {
                if state.node_name.pop().is_some() {
                    name_changed = true;
                }
            }
            Key::Character(c) => {
                for ch in c.chars() {
                    if ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '.' || ch == ' ' {
                        state.node_name.push(ch);
                    }
                }
            }
            _ => {}
        }
    }

    if name_changed && !state.editing_node {
        for entity in &preview_query {
            commands.entity(entity).despawn();
        }
        state.preview_entity = None;

        if state.obstacle_id.is_empty() && !state.node_name.is_empty() {
            state.obstacle_id = state.node_name.to_lowercase().replace(' ', "_");
        }
    }
}

pub fn update_display_values(
    state: Res<WorkshopState>,
    mut id_text: Query<&mut Text, (With<IdDisplayText>, Without<NodeDisplayText>)>,
    mut node_text: Query<&mut Text, (With<NodeDisplayText>, Without<IdDisplayText>)>,
    mut value_displays: Query<
        (&mut Text, &ValueDisplay),
        (Without<IdDisplayText>, Without<NodeDisplayText>, Without<IsGateText>, Without<HasTriggerText>),
    >,
    mut gate_text: Query<&mut Text, (With<IsGateText>, Without<HasTriggerText>, Without<IdDisplayText>, Without<NodeDisplayText>, Without<ValueDisplay>)>,
    mut trigger_text: Query<&mut Text, (With<HasTriggerText>, Without<IsGateText>, Without<IdDisplayText>, Without<NodeDisplayText>, Without<ValueDisplay>)>,
    mut gate_bg: Query<&mut BackgroundColor, (With<IsGateToggle>, Without<HasTriggerToggle>)>,
    mut trigger_bg: Query<&mut BackgroundColor, (With<HasTriggerToggle>, Without<IsGateToggle>)>,
) {
    if !state.is_changed() {
        return;
    }

    if let Ok(mut text) = id_text.single_mut() {
        let display = if state.obstacle_id.is_empty() {
            if state.editing_id {
                "|".to_string()
            } else {
                "(type an ID)".to_string()
            }
        } else if state.editing_id {
            format!("{}|", state.obstacle_id)
        } else {
            state.obstacle_id.clone()
        };
        **text = display;
    }

    if let Ok(mut text) = node_text.single_mut() {
        let display = if state.node_name.is_empty() {
            if state.editing_node {
                "|".to_string()
            } else {
                "(type or select from list)".to_string()
            }
        } else if state.editing_node {
            format!("{}|", state.node_name)
        } else {
            state.node_name.clone()
        };
        **text = display;
    }

    for (mut text, display) in &mut value_displays {
        let val = match display.0 {
            AdjustField::OffsetX => state.trigger_offset.x,
            AdjustField::OffsetY => state.trigger_offset.y,
            AdjustField::OffsetZ => state.trigger_offset.z,
            AdjustField::ExtentX => state.trigger_half_extents.x,
            AdjustField::ExtentY => state.trigger_half_extents.y,
            AdjustField::ExtentZ => state.trigger_half_extents.z,
        };
        **text = format!("{val:.1}");
    }

    if let Ok(mut text) = gate_text.single_mut() {
        **text = if state.is_gate { "ON" } else { "OFF" }.to_string();
    }
    if let Ok(mut bg) = gate_bg.single_mut() {
        *bg = BackgroundColor(if state.is_gate { TOGGLE_ON } else { TOGGLE_OFF });
    }

    if let Ok(mut text) = trigger_text.single_mut() {
        **text = if state.has_trigger { "ON" } else { "OFF" }.to_string();
    }
    if let Ok(mut bg) = trigger_bg.single_mut() {
        *bg = BackgroundColor(if state.has_trigger { TOGGLE_ON } else { TOGGLE_OFF });
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
                With<AdjustButton>,
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
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
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
