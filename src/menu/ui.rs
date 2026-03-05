use std::fs;

use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::course::discovery::{discover_courses, CourseEntry};
use crate::palette;
use crate::states::{AppState, PendingEditorCourse};
use crate::ui_theme;

const THUMBNAIL_DISPLAY_WIDTH: f32 = 192.0;
const THUMBNAIL_DISPLAY_HEIGHT: f32 = 108.0;

#[derive(Component)]
pub(crate) struct StartGameButton;

#[derive(Component)]
pub(crate) struct DevModeButton;

#[derive(Component)]
pub(crate) struct LandingRoot;

#[derive(Component)]
pub(crate) struct LocationSelectRoot;

/// Marker on the one enabled location card (Abandoned Warehouse).
#[derive(Component)]
pub(crate) struct LocationCard;

#[derive(Component)]
pub(crate) struct CourseLibraryButton;

#[derive(Component)]
pub(crate) struct CourseLibraryRoot;

#[derive(Component)]
pub(crate) struct CourseListItem(String);

#[derive(Component)]
pub(crate) struct CourseDeleteItem(String);

#[derive(Component)]
pub(crate) struct CourseLibraryBackButton;

struct LocationDef {
    name: &'static str,
    cost: u32,
    enabled: bool,
}

const LOCATIONS: &[LocationDef] = &[
    LocationDef { name: "Local Park", cost: 5, enabled: false },
    LocationDef { name: "Abandoned\nWarehouse", cost: 0, enabled: true },
    LocationDef { name: "Golf Course", cost: 50, enabled: false },
];

pub fn setup_menu(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(30.0),
                ..default()
            },
            DespawnOnExit(AppState::Menu),
            LandingRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("SONAR LARK"),
                TextFont { font_size: 64.0, ..default() },
                TextColor(palette::VANILLA),
            ));

            parent.spawn((
                Text::new("Drone Racing Simulator"),
                TextFont { font_size: 24.0, ..default() },
                TextColor(palette::SIDEWALK),
            ));

            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(16.0),
                    margin: UiRect::top(Val::Px(20.0)),
                    ..default()
                })
                .with_children(|col| {
                    ui_theme::spawn_menu_button(col, "Start Game", StartGameButton, 260.0);
                    ui_theme::spawn_disabled_menu_button(col, "Sandbox", 260.0);
                    ui_theme::spawn_disabled_menu_button(col, "Settings", 260.0);
                    ui_theme::spawn_menu_button(col, "Dev Mode", DevModeButton, 260.0);
                });
        });
}

pub fn handle_start_game_button(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<StartGameButton>)>,
    landing_query: Query<Entity, With<LandingRoot>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            for entity in &landing_query {
                commands.entity(entity).despawn();
            }
            spawn_location_select(&mut commands);
        }
    }
}

pub fn handle_dev_mode_button(
    query: Query<&Interaction, (Changed<Interaction>, With<DevModeButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::DevMenu);
        }
    }
}

fn spawn_location_select(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(30.0),
                ..default()
            },
            DespawnOnExit(AppState::Menu),
            LocationSelectRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Choose your Location..."),
                TextFont { font_size: 48.0, ..default() },
                TextColor(palette::VANILLA),
            ));

            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(40.0),
                    margin: UiRect::vertical(Val::Px(20.0)),
                    ..default()
                })
                .with_children(|row| {
                    for loc in LOCATIONS {
                        spawn_location_card(row, loc);
                    }
                });

            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(20.0),
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(40.0),
                    right: Val::Px(40.0),
                    ..default()
                })
                .with_children(|row| {
                    ui_theme::spawn_menu_button(row, "Course Library", CourseLibraryButton, 220.0);
                    ui_theme::spawn_disabled_menu_button(row, "SELECT", 200.0);
                });
        });
}

fn spawn_location_card(parent: &mut ChildSpawnerCommands, loc: &LocationDef) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|col| {
            col.spawn((
                Text::new(format!("${}", loc.cost)),
                TextFont { font_size: 20.0, ..default() },
                TextColor(if loc.enabled { palette::VANILLA } else { palette::CHAINMAIL }),
            ));

            if loc.enabled {
                col.spawn((
                    Button,
                    ui_theme::ThemedButton,
                    LocationCard,
                    Node {
                        width: Val::Px(220.0),
                        height: Val::Px(140.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(ui_theme::BUTTON_NORMAL),
                    BorderColor::all(ui_theme::BORDER_NORMAL),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new(loc.name),
                        TextFont { font_size: 22.0, ..default() },
                        TextColor(palette::VANILLA),
                        TextLayout::new_with_justify(Justify::Center),
                    ));
                });
            } else {
                col.spawn((
                    Node {
                        width: Val::Px(220.0),
                        height: Val::Px(140.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(ui_theme::BUTTON_DISABLED),
                    BorderColor::all(ui_theme::BORDER_DISABLED),
                ))
                .with_children(|card| {
                    card.spawn((
                        Text::new(loc.name),
                        TextFont { font_size: 22.0, ..default() },
                        TextColor(palette::CHAINMAIL),
                        TextLayout::new_with_justify(Justify::Center),
                    ));
                });
            }
        });
}

pub fn handle_location_card(
    query: Query<&Interaction, (Changed<Interaction>, With<LocationCard>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Editor);
        }
    }
}

pub fn handle_course_library_button(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<CourseLibraryButton>)>,
    location_root: Query<Entity, With<LocationSelectRoot>>,
    mut images: ResMut<Assets<Image>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            for entity in &location_root {
                commands.entity(entity).despawn();
            }
            spawn_course_library(&mut commands, &mut images);
        }
    }
}

fn spawn_course_library(commands: &mut Commands, images: &mut Assets<Image>) {
    let courses = discover_courses();

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
            DespawnOnExit(AppState::Menu),
            CourseLibraryRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Course Library"),
                TextFont { font_size: 48.0, ..default() },
                TextColor(palette::VANILLA),
            ));

            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    justify_content: JustifyContent::Center,
                    column_gap: Val::Px(12.0),
                    row_gap: Val::Px(12.0),
                    margin: UiRect::vertical(Val::Px(10.0)),
                    max_height: Val::Px(500.0),
                    max_width: Val::Px(900.0),
                    overflow: Overflow::scroll_y(),
                    ..default()
                })
                .with_children(|list| {
                    if courses.is_empty() {
                        list.spawn((
                            Text::new("No courses found"),
                            TextFont { font_size: 18.0, ..default() },
                            TextColor(palette::CHAINMAIL),
                        ));
                    } else {
                        for course in &courses {
                            spawn_course_list_item(list, course, images);
                        }
                    }
                });

            parent
                .spawn(Node {
                    margin: UiRect::top(Val::Px(20.0)),
                    ..default()
                })
                .with_children(|row| {
                    ui_theme::spawn_menu_button(row, "Back", CourseLibraryBackButton, 200.0);
                });
        });
}

fn spawn_course_list_item(
    parent: &mut ChildSpawnerCommands,
    course: &CourseEntry,
    images: &mut Assets<Image>,
) {
    let thumbnail_handle = course
        .thumbnail_path
        .as_ref()
        .and_then(|path| load_thumbnail_image(path, images));

    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|col| {
            // Thumbnail or placeholder as the clickable button
            col.spawn((
                Button,
                ui_theme::ThemedButton,
                CourseListItem(course.path.clone()),
                Node {
                    width: Val::Px(THUMBNAIL_DISPLAY_WIDTH + 4.0),
                    height: Val::Px(THUMBNAIL_DISPLAY_HEIGHT + 4.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(ui_theme::BUTTON_NORMAL),
                BorderColor::all(ui_theme::BORDER_NORMAL),
            ))
            .with_children(|btn| {
                if let Some(handle) = thumbnail_handle {
                    btn.spawn((
                        ImageNode::new(handle),
                        Node {
                            width: Val::Px(THUMBNAIL_DISPLAY_WIDTH),
                            height: Val::Px(THUMBNAIL_DISPLAY_HEIGHT),
                            ..default()
                        },
                    ));
                } else {
                    btn.spawn((
                        Text::new(&course.name),
                        TextFont { font_size: 16.0, ..default() },
                        TextColor(palette::CHAINMAIL),
                    ));
                }
            });

            // Delete button below thumbnail
            col.spawn((
                Button,
                ui_theme::ThemedButton,
                CourseDeleteItem(course.path.clone()),
                Node {
                    width: Val::Px(THUMBNAIL_DISPLAY_WIDTH + 4.0),
                    height: Val::Px(24.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(ui_theme::BUTTON_NORMAL),
                BorderColor::all(ui_theme::BORDER_NORMAL),
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new("Delete"),
                    TextFont { font_size: 12.0, ..default() },
                    TextColor(palette::CHAINMAIL),
                ));
            });
        });
}

/// Load a PNG from disk into a Bevy Image asset.
fn load_thumbnail_image(path: &str, images: &mut Assets<Image>) -> Option<Handle<Image>> {
    let bytes = fs::read(path).ok()?;
    let decoded = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let (width, height) = decoded.dimensions();
    let bevy_image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        decoded.into_raw(),
        TextureFormat::Rgba8UnormSrgb,
        default(),
    );
    Some(images.add(bevy_image))
}

pub fn handle_course_list_item(
    mut commands: Commands,
    query: Query<(&Interaction, &CourseListItem), Changed<Interaction>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, item) in &query {
        if *interaction == Interaction::Pressed {
            commands.insert_resource(PendingEditorCourse {
                path: item.0.clone(),
            });
            next_state.set(AppState::Editor);
        }
    }
}

pub fn handle_course_delete_item(
    mut commands: Commands,
    query: Query<(&Interaction, &CourseDeleteItem), Changed<Interaction>>,
    library_root: Query<Entity, With<CourseLibraryRoot>>,
    mut images: ResMut<Assets<Image>>,
) {
    for (interaction, item) in &query {
        if *interaction == Interaction::Pressed {
            let path = std::path::Path::new(&item.0);
            match fs::remove_file(path) {
                Ok(()) => info!("Deleted course: {}", item.0),
                Err(e) => error!("Failed to delete course '{}': {e}", item.0),
            }
            // Also delete the thumbnail if it exists
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .trim_end_matches(".course");
            let thumb_path = path.with_file_name(format!("{name}.png"));
            let _ = fs::remove_file(thumb_path);

            // Refresh the library view
            for entity in &library_root {
                commands.entity(entity).despawn();
            }
            spawn_course_library(&mut commands, &mut images);
            return;
        }
    }
}

pub fn handle_course_library_back(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<CourseLibraryBackButton>)>,
    library_root: Query<Entity, With<CourseLibraryRoot>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            for entity in &library_root {
                commands.entity(entity).despawn();
            }
            spawn_location_select(&mut commands);
        }
    }
}

pub fn cleanup_menu(mut _commands: Commands) {}
