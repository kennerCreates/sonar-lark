use std::fs;
use std::path::Path;

use bevy::prelude::*;

use crate::course::loader::SelectedCourse;
use crate::states::AppState;

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);
const SELECTED_COURSE: Color = Color::srgb(0.2, 0.4, 0.6);
const NORMAL_COURSE: Color = Color::srgb(0.1, 0.1, 0.1);
const HOVERED_COURSE: Color = Color::srgb(0.2, 0.2, 0.2);

pub struct CourseEntry {
    pub name: String,
    pub path: String,
}

#[derive(Resource, Default)]
pub struct AvailableCourses {
    pub courses: Vec<CourseEntry>,
    pub selected_index: Option<usize>,
}

#[derive(Component)]
pub(crate) struct EditorButton;

#[derive(Component)]
pub(crate) struct RaceButton;

#[derive(Component)]
pub(crate) struct CourseItem(usize);

#[derive(Component)]
pub(crate) struct RaceButtonText;

fn discover_courses() -> Vec<CourseEntry> {
    let courses_dir = Path::new("assets/courses");
    let mut courses = Vec::new();

    if let Ok(entries) = fs::read_dir(courses_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("ron") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    let display_name = name.trim_end_matches(".course").to_string();
                    courses.push(CourseEntry {
                        name: display_name,
                        path: path.to_string_lossy().to_string(),
                    });
                }
            }
        }
    }

    courses.sort_by(|a, b| a.name.cmp(&b.name));
    courses
}

pub fn setup_menu(mut commands: Commands) {
    let courses = discover_courses();
    let available = AvailableCourses {
        courses,
        selected_index: None,
    };

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
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("SONAR LARK"),
                TextFont {
                    font_size: 64.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
            ));

            // Subtitle
            parent.spawn((
                Text::new("Drone Racing Simulator"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ));

            // Course selection area
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(8.0),
                    margin: UiRect::vertical(Val::Px(20.0)),
                    ..default()
                })
                .with_children(|course_area| {
                    course_area.spawn((
                        Text::new("Select Course"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 0.7, 0.7)),
                    ));

                    if available.courses.is_empty() {
                        course_area.spawn((
                            Text::new("No courses found"),
                            TextFont {
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.5, 0.5, 0.5)),
                        ));
                    } else {
                        for (i, course) in available.courses.iter().enumerate() {
                            course_area
                                .spawn((
                                    Button,
                                    CourseItem(i),
                                    Node {
                                        width: Val::Px(300.0),
                                        height: Val::Px(40.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(2.0)),
                                        ..default()
                                    },
                                    BackgroundColor(NORMAL_COURSE),
                                    BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
                                ))
                                .with_children(|btn| {
                                    btn.spawn((
                                        Text::new(&course.name),
                                        TextFont {
                                            font_size: 18.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                                    ));
                                });
                        }
                    }
                });

            // Button row
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(20.0),
                    ..default()
                })
                .with_children(|row| {
                    spawn_menu_button(row, "Editor", EditorButton);

                    row.spawn((
                        Button,
                        RaceButton,
                        Node {
                            width: Val::Px(200.0),
                            height: Val::Px(60.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(3.0)),
                            ..default()
                        },
                        BackgroundColor(NORMAL_BUTTON),
                        BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("Race"),
                            TextFont {
                                font_size: 24.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.5, 0.5, 0.5)),
                            RaceButtonText,
                        ));
                    });
                });

            parent.spawn((
                Text::new("Select a course to enable racing"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.4, 0.4, 0.4)),
            ));
        });

    commands.insert_resource(available);
}

fn spawn_menu_button(parent: &mut ChildSpawnerCommands, label: &str, marker: impl Component) {
    parent
        .spawn((
            Button,
            marker,
            Node {
                width: Val::Px(200.0),
                height: Val::Px(60.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(3.0)),
                ..default()
            },
            BackgroundColor(NORMAL_BUTTON),
            BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
        ))
        .with_children(|btn: &mut ChildSpawnerCommands| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
            ));
        });
}

pub fn handle_course_selection(
    mut available: ResMut<AvailableCourses>,
    mut course_query: Query<
        (&Interaction, &CourseItem, &mut BackgroundColor, &mut BorderColor),
        Changed<Interaction>,
    >,
) {
    for (interaction, course_item, mut bg, mut border) in &mut course_query {
        match *interaction {
            Interaction::Pressed => {
                available.selected_index = Some(course_item.0);
            }
            Interaction::Hovered => {
                if available.selected_index != Some(course_item.0) {
                    *bg = BackgroundColor(HOVERED_COURSE);
                    *border = BorderColor::all(Color::srgb(0.5, 0.5, 0.5));
                }
            }
            Interaction::None => {
                if available.selected_index != Some(course_item.0) {
                    *bg = BackgroundColor(NORMAL_COURSE);
                    *border = BorderColor::all(Color::srgb(0.3, 0.3, 0.3));
                }
            }
        }
    }
}

pub fn update_course_highlights(
    available: Res<AvailableCourses>,
    mut course_query: Query<(&CourseItem, &mut BackgroundColor, &mut BorderColor)>,
    mut race_text_query: Query<&mut TextColor, With<RaceButtonText>>,
) {
    if !available.is_changed() {
        return;
    }

    for (course_item, mut bg, mut border) in &mut course_query {
        if available.selected_index == Some(course_item.0) {
            *bg = BackgroundColor(SELECTED_COURSE);
            *border = BorderColor::all(Color::srgb(0.4, 0.7, 1.0));
        } else {
            *bg = BackgroundColor(NORMAL_COURSE);
            *border = BorderColor::all(Color::srgb(0.3, 0.3, 0.3));
        }
    }

    let has_selection = available.selected_index.is_some();
    for mut text_color in &mut race_text_query {
        *text_color = if has_selection {
            TextColor(Color::srgb(0.9, 0.9, 0.9))
        } else {
            TextColor(Color::srgb(0.5, 0.5, 0.5))
        };
    }
}

pub fn handle_editor_button(
    query: Query<&Interaction, (Changed<Interaction>, With<EditorButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Editor);
        }
    }
}

pub fn handle_race_button(
    query: Query<&Interaction, (Changed<Interaction>, With<RaceButton>)>,
    available: Res<AvailableCourses>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            if let Some(idx) = available.selected_index {
                if let Some(course) = available.courses.get(idx) {
                    commands.insert_resource(SelectedCourse {
                        path: course.path.clone(),
                    });
                    next_state.set(AppState::Race);
                }
            }
        }
    }
}

pub fn handle_button_visuals(
    mut query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, Or<(With<EditorButton>, With<RaceButton>)>),
    >,
) {
    for (interaction, mut bg, mut border) in &mut query {
        match *interaction {
            Interaction::Pressed => {
                *bg = BackgroundColor(PRESSED_BUTTON);
                *border = BorderColor::all(Color::WHITE);
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(HOVERED_BUTTON);
                *border = BorderColor::all(Color::srgb(0.6, 0.6, 0.6));
            }
            Interaction::None => {
                *bg = BackgroundColor(NORMAL_BUTTON);
                *border = BorderColor::all(Color::srgb(0.3, 0.3, 0.3));
            }
        }
    }
}

pub fn cleanup_menu(mut commands: Commands) {
    commands.remove_resource::<AvailableCourses>();
}
