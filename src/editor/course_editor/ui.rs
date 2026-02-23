use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use std::fs;
use std::path::Path;

use crate::course::data::{CourseData, ObstacleInstance};
use crate::course::loader::{delete_course, load_course_from_file, save_course};
use crate::obstacle::definition::ObstacleId;
use crate::obstacle::library::ObstacleLibrary;
use crate::obstacle::spawning::ObstaclesGltfHandle;
use crate::states::{AppState, EditorMode};

use super::{LastEditedCourse, PendingEditorCourse, PlacedObstacle, PlacementState, TransformMode};

const PANEL_BG: Color = Color::srgba(0.08, 0.08, 0.08, 0.9);
const BUTTON_NORMAL: Color = Color::srgb(0.15, 0.15, 0.15);
const BUTTON_HOVERED: Color = Color::srgb(0.25, 0.25, 0.25);
const BUTTON_PRESSED: Color = Color::srgb(0.35, 0.75, 0.35);
const BUTTON_SELECTED: Color = Color::srgb(0.2, 0.4, 0.7);
const TOGGLE_ON: Color = Color::srgb(0.2, 0.6, 0.3);
const TOGGLE_OFF: Color = Color::srgb(0.4, 0.15, 0.15);

// --- Marker components ---

#[derive(Component)]
pub struct PaletteButton(pub ObstacleId);

#[derive(Component)]
pub struct ExistingCourseButton(pub String);

#[derive(Component)]
pub struct BackToWorkshopButton;

#[derive(Component)]
pub struct BackToMenuButton;

#[derive(Component)]
pub struct SaveCourseButton;

#[derive(Component)]
pub struct GateOrderModeButton;

#[derive(Component)]
pub struct GateOrderModeText;

#[derive(Component)]
pub struct ClearGateOrdersButton;

#[derive(Component)]
pub struct GateCountText;

#[derive(Component)]
pub struct CourseNameField;

#[derive(Component)]
pub struct CourseNameDisplayText;

#[derive(Component)]
pub struct PaletteContainer;

#[derive(Component)]
pub struct ExistingCoursesContainer;

#[derive(Component)]
pub struct DeleteCourseButton(pub String);

#[derive(Component)]
pub struct ConfirmDeleteYesButton;

#[derive(Component)]
pub struct ConfirmDeleteCancelButton;

#[derive(Resource)]
pub struct PendingCourseDelete {
    pub path: String,
    pub display_name: String,
}

#[derive(Component)]
pub struct TransformModeButton(pub TransformMode);

pub struct CourseEntry {
    pub display_name: String,
    pub path: String,
}

pub fn discover_existing_courses() -> Vec<CourseEntry> {
    let mut courses = Vec::new();
    if let Ok(entries) = fs::read_dir(Path::new("assets/courses")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("ron") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    let display_name = name.trim_end_matches(".course").to_string();
                    courses.push(CourseEntry {
                        display_name,
                        path: path.to_string_lossy().to_string(),
                    });
                }
            }
        }
    }
    courses.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    courses
}

pub fn build_course_editor_ui(
    commands: &mut Commands,
    library: &ObstacleLibrary,
    existing_courses: &[CourseEntry],
) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            DespawnOnExit(EditorMode::CourseEditor),
        ))
        .with_children(|root| {
            build_left_panel(root, library);
            build_right_panel(root, existing_courses);
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
                row_gap: Val::Px(6.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(PANEL_BG),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("Course Editor"),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
            ));

            panel.spawn((
                Text::new("Obstacle Palette"),
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

            panel
                .spawn((
                    PaletteContainer,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                ))
                .with_children(|container| {
                    if library.definitions.is_empty() {
                        container.spawn((
                            Text::new("No obstacles in library.\nGo to Obstacle Workshop first."),
                            TextFont {
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.5, 0.5, 0.5)),
                        ));
                    } else {
                        let mut ids: Vec<_> = library.definitions.keys().collect();
                        ids.sort_by(|a, b| a.0.cmp(&b.0));
                        for id in ids {
                            spawn_palette_button(container, id);
                        }
                    }
                });

            panel.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });

            spawn_divider(panel);
            spawn_small_button(panel, "Obstacle Workshop", BackToWorkshopButton);
            spawn_small_button(panel, "Back to Menu", BackToMenuButton);
        });
}

fn build_right_panel(parent: &mut ChildSpawnerCommands, existing_courses: &[CourseEntry]) {
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
            panel.spawn((
                Text::new("Course Name"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ));

            panel
                .spawn((
                    Button,
                    CourseNameField,
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
                        Text::new("new_course"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                        CourseNameDisplayText,
                    ));
                });

            spawn_action_button(
                panel,
                "Save Course",
                SaveCourseButton,
                Color::srgb(0.15, 0.4, 0.15),
            );

            spawn_divider(panel);

            panel.spawn((
                Text::new("Load Existing"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ));

            panel
                .spawn((
                    ExistingCoursesContainer,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        max_height: Val::Px(120.0),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                ))
                .with_children(|container| {
                    if existing_courses.is_empty() {
                        container.spawn((
                            Text::new("No courses found"),
                            TextFont {
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.5, 0.5, 0.5)),
                        ));
                    } else {
                        for course in existing_courses {
                            spawn_existing_course_button(
                                container,
                                &course.display_name,
                                &course.path,
                            );
                        }
                    }
                });

            spawn_divider(panel);

            panel.spawn((
                Text::new("Gate Ordering"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ));

            panel.spawn((
                Text::new("Gates: 0 (loop)"),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.4, 0.8, 1.0)),
                GateCountText,
            ));

            panel
                .spawn((
                    Button,
                    GateOrderModeButton,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(32.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(TOGGLE_OFF),
                    BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("Gate Mode: OFF"),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        GateOrderModeText,
                    ));
                });

            spawn_small_button(panel, "Clear Gate Orders", ClearGateOrdersButton);

            spawn_divider(panel);

            panel.spawn((
                Text::new("Transform Mode"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ));

            panel
                .spawn((Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(4.0),
                    width: Val::Percent(100.0),
                    ..default()
                },))
                .with_children(|row| {
                    spawn_transform_mode_button(row, "Move (G)", TransformMode::Move);
                    spawn_transform_mode_button(row, "Rotate (R)", TransformMode::Rotate);
                    spawn_transform_mode_button(row, "Scale (S)", TransformMode::Scale);
                });

            spawn_divider(panel);

            panel.spawn((
                Text::new("Del  →  delete selected"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
            ));

            panel.spawn((
                Text::new("LMB obstacle  →  select"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
            ));

            panel.spawn((
                Text::new("LMB palette + empty  →  place"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
            ));

            panel.spawn((
                Text::new("Gate mode: LMB to assign order"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
            ));

            panel.spawn((
                Text::new("F  →  flip gate direction"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
            ));
        });
}

pub fn spawn_palette_button(parent: &mut ChildSpawnerCommands, id: &ObstacleId) {
    parent
        .spawn((
            Button,
            PaletteButton(id.clone()),
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
                Text::new(&id.0),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.85, 0.7)),
            ));
        });
}

pub fn spawn_existing_course_button(
    parent: &mut ChildSpawnerCommands,
    display_name: &str,
    path: &str,
) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(2.0),
            ..default()
        })
        .with_children(|row| {
            // Load button (fills remaining space)
            row.spawn((
                Button,
                ExistingCourseButton(path.to_string()),
                Node {
                    flex_grow: 1.0,
                    height: Val::Px(26.0),
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
                    Text::new(display_name),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.8, 0.8)),
                ));
            });

            // Delete "X" button
            row.spawn((
                Button,
                DeleteCourseButton(path.to_string()),
                Node {
                    width: Val::Px(26.0),
                    height: Val::Px(26.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.35, 0.1, 0.1)),
                BorderColor::all(Color::srgb(0.4, 0.15, 0.15)),
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new("X"),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.3, 0.3)),
                ));
            });
        });
}

fn spawn_small_button(parent: &mut ChildSpawnerCommands, label: &str, marker: impl Component) {
    parent
        .spawn((
            Button,
            marker,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(30.0),
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
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
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

fn spawn_transform_mode_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    mode: TransformMode,
) {
    parent
        .spawn((
            Button,
            TransformModeButton(mode),
            Node {
                flex_grow: 1.0,
                height: Val::Px(30.0),
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
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
        });
}

// --- Interaction Systems ---

pub fn handle_palette_selection(
    mut commands: Commands,
    mut state: ResMut<PlacementState>,
    query: Query<(&Interaction, &PaletteButton), Changed<Interaction>>,
    library: Res<ObstacleLibrary>,
    gltf_handle: Option<Res<ObstaclesGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    node_assets: Res<Assets<bevy::gltf::GltfNode>>,
    mesh_assets: Res<Assets<bevy::gltf::GltfMesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (interaction, btn) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let Some(def) = library.get(&btn.0) else {
            continue;
        };
        let Some(handle) = &gltf_handle else {
            warn!("glTF not loaded yet, cannot place obstacle");
            continue;
        };

        let transform = Transform::from_translation(Vec3::ZERO);
        let spawned = crate::obstacle::spawning::spawn_obstacle(
            &mut commands,
            &gltf_assets,
            &node_assets,
            &mesh_assets,
            &mut materials,
            handle,
            &def.id,
            &def.glb_node_name,
            transform,
            def.model_offset,
            def.trigger_volume.as_ref(),
            None,
            false,
        );

        if let Some(entity) = spawned {
            commands.entity(entity).insert(PlacedObstacle {
                obstacle_id: btn.0.clone(),
                gate_order: None,
                gate_forward_flipped: false,
            });
            state.selected_entity = Some(entity);
            state.selected_palette_id = None;
        } else {
            warn!(
                "Failed to spawn obstacle '{}' (node '{}')",
                def.id.0, def.glb_node_name
            );
        }
    }
}

pub fn handle_back_to_workshop(
    query: Query<&Interaction, (Changed<Interaction>, With<BackToWorkshopButton>)>,
    mut next_mode: ResMut<NextState<EditorMode>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_mode.set(EditorMode::ObstacleWorkshop);
        }
    }
}

pub fn handle_back_to_menu(
    query: Query<&Interaction, (Changed<Interaction>, With<BackToMenuButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Menu);
        }
    }
}

fn rebuild_courses_list(commands: &mut Commands, container: Entity) {
    let courses = discover_existing_courses();
    commands.entity(container).despawn_related::<Children>();
    commands.entity(container).with_children(|parent| {
        if courses.is_empty() {
            parent.spawn((
                Text::new("No courses found"),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
            ));
        } else {
            for course in &courses {
                spawn_existing_course_button(parent, &course.display_name, &course.path);
            }
        }
    });
}

fn spawn_delete_confirmation(commands: &mut Commands, container: Entity, display_name: &str) {
    commands.entity(container).despawn_related::<Children>();
    commands.entity(container).with_children(|parent| {
        parent.spawn((
            Text::new(format!("Delete \"{display_name}\"?")),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.6, 0.6)),
        ));
        parent
            .spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(4.0),
                width: Val::Percent(100.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    Button,
                    ConfirmDeleteYesButton,
                    Node {
                        flex_grow: 1.0,
                        height: Val::Px(28.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.5, 0.1, 0.1)),
                    BorderColor::all(Color::srgb(0.6, 0.2, 0.2)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("Yes"),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.8, 0.8)),
                    ));
                });

                row.spawn((
                    Button,
                    ConfirmDeleteCancelButton,
                    Node {
                        flex_grow: 1.0,
                        height: Val::Px(28.0),
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
                        Text::new("Cancel"),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                    ));
                });
            });
    });
}

pub fn handle_save_button(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<SaveCourseButton>)>,
    state: Res<PlacementState>,
    placed_query: Query<(&PlacedObstacle, &Transform)>,
    existing_courses_container: Query<Entity, With<ExistingCoursesContainer>>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if state.course_name.is_empty() {
            warn!("Cannot save: course name is empty");
            continue;
        }

        let instances: Vec<ObstacleInstance> = placed_query
            .iter()
            .map(|(placed, transform)| ObstacleInstance {
                obstacle_id: placed.obstacle_id.clone(),
                translation: transform.translation,
                rotation: transform.rotation,
                scale: transform.scale,
                gate_order: placed.gate_order,
                gate_forward_flipped: placed.gate_forward_flipped,
            })
            .collect();

        let course = CourseData {
            name: state.course_name.clone(),
            instances,
        };

        let path_str = format!("assets/courses/{}.course.ron", state.course_name);
        let path = std::path::Path::new(&path_str);
        match save_course(&course, path) {
            Ok(()) => {
                info!(
                    "Saved course '{}' ({} obstacles) to {}",
                    state.course_name,
                    course.instances.len(),
                    path_str
                );
                commands.insert_resource(LastEditedCourse {
                    path: path_str.clone(),
                });
            }
            Err(e) => {
                error!("Failed to save course: {e}");
                continue;
            }
        }

        if let Ok(container) = existing_courses_container.single() {
            rebuild_courses_list(&mut commands, container);
        }
    }
}

pub fn handle_delete_button(
    mut commands: Commands,
    query: Query<(&Interaction, &DeleteCourseButton), Changed<Interaction>>,
    existing_courses_container: Query<Entity, With<ExistingCoursesContainer>>,
    pending: Option<Res<PendingCourseDelete>>,
) {
    if pending.is_some() {
        return;
    }

    for (interaction, btn) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let path = &btn.0;
        let display_name = Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(path)
            .trim_end_matches(".course")
            .to_string();

        commands.insert_resource(PendingCourseDelete {
            path: path.clone(),
            display_name: display_name.clone(),
        });

        if let Ok(container) = existing_courses_container.single() {
            spawn_delete_confirmation(&mut commands, container, &display_name);
        }

        break;
    }
}

pub fn handle_confirm_delete(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<ConfirmDeleteYesButton>)>,
    pending: Option<Res<PendingCourseDelete>>,
    mut state: ResMut<PlacementState>,
    placed_query: Query<Entity, With<PlacedObstacle>>,
    existing_courses_container: Query<Entity, With<ExistingCoursesContainer>>,
    last_edited: Option<Res<LastEditedCourse>>,
) {
    let Some(pending) = pending else { return };

    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let path = Path::new(&pending.path);
        match delete_course(path) {
            Ok(()) => {
                info!("Deleted course '{}'", pending.display_name);

                // If we deleted the currently loaded course, clear the editor
                if state.course_name == pending.display_name {
                    for entity in &placed_query {
                        commands.entity(entity).despawn();
                    }
                    state.selected_entity = None;
                    state.selected_palette_id = None;
                    state.course_name = "new_course".to_string();
                    state.next_gate_order = 0;
                }

                // If the deleted course was the last edited, remove that resource
                if let Some(last) = &last_edited {
                    if last.path == pending.path {
                        commands.remove_resource::<LastEditedCourse>();
                    }
                }
            }
            Err(e) => {
                error!("Failed to delete course: {e}");
            }
        }

        commands.remove_resource::<PendingCourseDelete>();

        if let Ok(container) = existing_courses_container.single() {
            rebuild_courses_list(&mut commands, container);
        }

        break;
    }
}

pub fn handle_cancel_delete(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<ConfirmDeleteCancelButton>)>,
    pending: Option<Res<PendingCourseDelete>>,
    existing_courses_container: Query<Entity, With<ExistingCoursesContainer>>,
) {
    let Some(_pending) = pending else { return };

    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        commands.remove_resource::<PendingCourseDelete>();

        if let Ok(container) = existing_courses_container.single() {
            rebuild_courses_list(&mut commands, container);
        }

        break;
    }
}

/// Shared logic: despawn existing obstacles, load course data, spawn obstacles.
#[allow(clippy::too_many_arguments)]
fn load_course_into_editor(
    commands: &mut Commands,
    state: &mut PlacementState,
    placed_query: &Query<Entity, With<PlacedObstacle>>,
    library: &ObstacleLibrary,
    gltf_handle: &ObstaclesGltfHandle,
    gltf_assets: &Assets<bevy::gltf::Gltf>,
    node_assets: &Assets<bevy::gltf::GltfNode>,
    mesh_assets: &Assets<bevy::gltf::GltfMesh>,
    materials: &mut Assets<StandardMaterial>,
    course: &CourseData,
) {
    for entity in placed_query {
        commands.entity(entity).despawn();
    }

    state.selected_entity = None;
    state.selected_palette_id = None;
    state.course_name = course.name.clone();
    state.next_gate_order = course
        .instances
        .iter()
        .filter_map(|i| i.gate_order)
        .max()
        .map(|m| m + 1)
        .unwrap_or(0);

    for instance in &course.instances {
        let Some(def) = library.get(&instance.obstacle_id) else {
            warn!(
                "Unknown obstacle '{}' in course file, skipping",
                instance.obstacle_id.0
            );
            continue;
        };

        let transform = Transform {
            translation: instance.translation,
            rotation: instance.rotation,
            scale: instance.scale,
        };

        let spawned = crate::obstacle::spawning::spawn_obstacle(
            commands,
            gltf_assets,
            node_assets,
            mesh_assets,
            materials,
            gltf_handle,
            &def.id,
            &def.glb_node_name,
            transform,
            def.model_offset,
            def.trigger_volume.as_ref(),
            None,
            instance.gate_forward_flipped,
        );

        if let Some(entity) = spawned {
            commands.entity(entity).insert(PlacedObstacle {
                obstacle_id: instance.obstacle_id.clone(),
                gate_order: instance.gate_order,
                gate_forward_flipped: instance.gate_forward_flipped,
            });
        } else {
            warn!(
                "Failed to spawn '{}' (node '{}') from loaded course",
                instance.obstacle_id.0, def.glb_node_name
            );
        }
    }

    info!(
        "Loaded course '{}' for editing ({} obstacles)",
        course.name,
        course.instances.len()
    );
}

pub fn handle_load_button(
    mut commands: Commands,
    query: Query<(&Interaction, &ExistingCourseButton), Changed<Interaction>>,
    mut state: ResMut<PlacementState>,
    placed_query: Query<Entity, With<PlacedObstacle>>,
    library: Res<ObstacleLibrary>,
    gltf_handle: Option<Res<ObstaclesGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    node_assets: Res<Assets<bevy::gltf::GltfNode>>,
    mesh_assets: Res<Assets<bevy::gltf::GltfMesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (interaction, btn) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let path = std::path::Path::new(&btn.0);
        let course = match load_course_from_file(path) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to load course: {e}");
                continue;
            }
        };

        let Some(handle) = &gltf_handle else {
            warn!("glTF not loaded yet, cannot spawn loaded course obstacles");
            continue;
        };

        load_course_into_editor(
            &mut commands,
            &mut state,
            &placed_query,
            &library,
            handle,
            &gltf_assets,
            &node_assets,
            &mesh_assets,
            &mut materials,
            &course,
        );

        commands.insert_resource(LastEditedCourse {
            path: btn.0.clone(),
        });
    }
}

/// Polls for a `PendingEditorCourse` and loads it once glTF assets are ready.
pub fn auto_load_pending_course(
    mut commands: Commands,
    pending: Option<Res<PendingEditorCourse>>,
    mut state: ResMut<PlacementState>,
    placed_query: Query<Entity, With<PlacedObstacle>>,
    library: Res<ObstacleLibrary>,
    gltf_handle: Option<Res<ObstaclesGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    node_assets: Res<Assets<bevy::gltf::GltfNode>>,
    mesh_assets: Res<Assets<bevy::gltf::GltfMesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(pending) = pending else { return };
    let Some(handle) = &gltf_handle else { return };
    // Wait until the glTF asset is actually loaded
    if gltf_assets.get(&handle.0).is_none() {
        return;
    }

    let path = std::path::Path::new(&pending.path);
    let course = match load_course_from_file(path) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to auto-load course: {e}");
            commands.remove_resource::<PendingEditorCourse>();
            return;
        }
    };

    load_course_into_editor(
        &mut commands,
        &mut state,
        &placed_query,
        &library,
        handle,
        &gltf_assets,
        &node_assets,
        &mesh_assets,
        &mut materials,
        &course,
    );

    commands.insert_resource(LastEditedCourse {
        path: pending.path.clone(),
    });
    commands.remove_resource::<PendingEditorCourse>();
}

pub fn handle_gate_order_toggle(
    mut state: ResMut<PlacementState>,
    query: Query<&Interaction, (Changed<Interaction>, With<GateOrderModeButton>)>,
    placed_query: Query<&PlacedObstacle>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            state.gate_order_mode = !state.gate_order_mode;
            if state.gate_order_mode {
                // Continue from max existing gate order so the user can add gates
                // incrementally without losing previous assignments.
                state.next_gate_order = placed_query
                    .iter()
                    .filter_map(|p| p.gate_order)
                    .max()
                    .map(|m| m + 1)
                    .unwrap_or(0);
                state.selected_entity = None;
            }
        }
    }
}

pub fn handle_clear_gate_orders_button(
    mut state: ResMut<PlacementState>,
    query: Query<&Interaction, (Changed<Interaction>, With<ClearGateOrdersButton>)>,
    mut placed_query: Query<&mut PlacedObstacle>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        for mut placed in &mut placed_query {
            placed.gate_order = None;
        }
        state.next_gate_order = 0;
        info!("Cleared all gate orders");
    }
}

pub fn handle_name_field_focus(
    mut state: ResMut<PlacementState>,
    query: Query<&Interaction, (Changed<Interaction>, With<CourseNameField>)>,
    mut border: Query<&mut BorderColor, With<CourseNameField>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            state.editing_name = true;
            if let Ok(mut b) = border.single_mut() {
                *b = BorderColor::all(Color::srgb(0.4, 0.7, 1.0));
            }
        }
    }
}

pub fn handle_name_text_input(
    mut state: ResMut<PlacementState>,
    mut events: MessageReader<KeyboardInput>,
    mut border: Query<&mut BorderColor, With<CourseNameField>>,
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
                    *b = BorderColor::all(Color::srgb(0.3, 0.3, 0.3));
                }
            }
            Key::Backspace => {
                state.course_name.pop();
            }
            Key::Space => {
                state.course_name.push('_');
            }
            Key::Character(c) => {
                for ch in c.chars() {
                    if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                        state.course_name.push(ch.to_ascii_lowercase());
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn update_display_values(
    state: Res<PlacementState>,
    mut name_text: Query<
        &mut Text,
        (With<CourseNameDisplayText>, Without<GateOrderModeText>),
    >,
    mut gate_mode_text: Query<
        &mut Text,
        (With<GateOrderModeText>, Without<CourseNameDisplayText>),
    >,
    mut gate_mode_bg: Query<
        &mut BackgroundColor,
        (With<GateOrderModeButton>, Without<PaletteButton>),
    >,
    mut palette_bgs: Query<
        (&PaletteButton, &mut BackgroundColor),
        Without<GateOrderModeButton>,
    >,
) {
    if !state.is_changed() {
        return;
    }

    if let Ok(mut text) = name_text.single_mut() {
        **text = if state.course_name.is_empty() {
            if state.editing_name {
                "|".to_string()
            } else {
                "(type a name)".to_string()
            }
        } else if state.editing_name {
            format!("{}|", state.course_name)
        } else {
            state.course_name.clone()
        };
    }

    if let Ok(mut text) = gate_mode_text.single_mut() {
        **text = if state.gate_order_mode {
            "Gate Mode: ON".to_string()
        } else {
            "Gate Mode: OFF".to_string()
        };
    }

    if let Ok(mut bg) = gate_mode_bg.single_mut() {
        *bg = BackgroundColor(if state.gate_order_mode {
            TOGGLE_ON
        } else {
            TOGGLE_OFF
        });
    }

    for (btn, mut bg) in &mut palette_bgs {
        *bg = BackgroundColor(
            if state.selected_palette_id.as_ref() == Some(&btn.0) {
                BUTTON_SELECTED
            } else {
                BUTTON_NORMAL
            },
        );
    }
}

pub fn handle_button_hover(
    mut query: Query<
        (&Interaction, &mut BackgroundColor),
        (
            Changed<Interaction>,
            Or<(
                With<BackToWorkshopButton>,
                With<BackToMenuButton>,
                With<ExistingCourseButton>,
                With<ClearGateOrdersButton>,
                With<DeleteCourseButton>,
                With<ConfirmDeleteYesButton>,
                With<ConfirmDeleteCancelButton>,
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

pub fn handle_transform_mode_buttons(
    mut state: ResMut<PlacementState>,
    query: Query<(&Interaction, &TransformModeButton), Changed<Interaction>>,
) {
    for (interaction, btn) in &query {
        if *interaction == Interaction::Pressed {
            state.transform_mode = btn.0;
        }
    }
}

pub fn update_transform_mode_ui(
    state: Res<PlacementState>,
    mut buttons: Query<(&TransformModeButton, &mut BackgroundColor)>,
) {
    if !state.is_changed() {
        return;
    }
    for (btn, mut bg) in &mut buttons {
        *bg = BackgroundColor(if btn.0 == state.transform_mode {
            BUTTON_SELECTED
        } else {
            BUTTON_NORMAL
        });
    }
}

pub fn update_gate_count_display(
    placed_query: Query<&PlacedObstacle>,
    mut text_query: Query<(&mut Text, &mut TextColor), With<GateCountText>>,
) {
    let gate_count = placed_query.iter().filter(|p| p.gate_order.is_some()).count();
    if let Ok((mut text, mut color)) = text_query.single_mut() {
        **text = format!("Gates: {gate_count} (loop)");
        *color = if gate_count >= 2 {
            TextColor(Color::srgb(0.4, 0.8, 1.0))
        } else {
            TextColor(Color::srgb(0.8, 0.4, 0.2))
        };
    }
}
