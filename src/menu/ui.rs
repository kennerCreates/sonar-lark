use bevy::prelude::*;

use crate::course::loader::SelectedCourse;
use crate::states::{LastEditedCourse, PendingEditorCourse};
use crate::palette;
use crate::states::AppState;
use crate::ui_theme;

use super::discover::{discover_courses, CourseEntry};

const SELECTED_COURSE: Color = palette::TEAL;
const NORMAL_COURSE: Color = palette::SMOKY_BLACK;
const HOVERED_COURSE: Color = palette::INDIGO;

const MIN_RACEABLE_GATES: usize = 3;

#[derive(Resource, Default)]
pub struct AvailableCourses {
    pub courses: Vec<CourseEntry>,
    pub selected_index: Option<usize>,
}

#[derive(Component)]
pub(crate) struct StartGameButton;

#[derive(Component)]
pub(crate) struct DevModeButton;

#[derive(Component)]
pub(crate) struct LandingRoot;

#[derive(Component)]
pub(crate) struct GameMenuRoot;

#[derive(Component)]
pub(crate) struct EditorButton;

#[derive(Component)]
pub(crate) struct RaceButton;

#[derive(Component)]
pub(crate) struct CourseItem(usize);

#[derive(Component)]
pub(crate) struct RaceButtonText;

#[derive(Component)]
pub(crate) struct HintText;

pub fn setup_menu(mut commands: Commands) {
    // Landing screen: title + two buttons
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
            // Title
            parent.spawn((
                Text::new("SONAR LARK"),
                TextFont {
                    font_size: 64.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));

            // Subtitle
            parent.spawn((
                Text::new("Drone Racing Simulator"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
            ));

            // Button column
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
            // Despawn landing screen
            for entity in &landing_query {
                commands.entity(entity).despawn();
            }
            // Spawn game menu
            spawn_game_menu(&mut commands);
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

fn spawn_game_menu(commands: &mut Commands) {
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
            GameMenuRoot,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("SONAR LARK"),
                TextFont {
                    font_size: 64.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));

            // Subtitle
            parent.spawn((
                Text::new("Drone Racing Tycoon"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
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
                        TextColor(palette::SIDEWALK),
                    ));

                    if available.courses.is_empty() {
                        course_area.spawn((
                            Text::new("No courses found"),
                            TextFont {
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(palette::CHAINMAIL),
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
                                    BorderColor::all(palette::STEEL),
                                ))
                                .with_children(|btn| {
                                    btn.spawn((
                                        Text::new(&course.name),
                                        TextFont {
                                            font_size: 18.0,
                                            ..default()
                                        },
                                        TextColor(palette::SAND),
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
                    ui_theme::spawn_menu_button(row, "Editor", EditorButton, 200.0);

                    row.spawn((
                        Button,
                        ui_theme::ThemedButton,
                        RaceButton,
                        Node {
                            width: Val::Px(200.0),
                            height: Val::Px(60.0),
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
                            Text::new("Race"),
                            TextFont {
                                font_size: 24.0,
                                ..default()
                            },
                            TextColor(palette::CHAINMAIL),
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
                TextColor(palette::STONE),
                HintText,
            ));
        });

    commands.insert_resource(available);
}


pub fn handle_course_selection(
    available: Option<ResMut<AvailableCourses>>,
    mut course_query: Query<
        (&Interaction, &CourseItem, &mut BackgroundColor, &mut BorderColor),
        Changed<Interaction>,
    >,
) {
    let Some(mut available) = available else { return };
    for (interaction, course_item, mut bg, mut border) in &mut course_query {
        match *interaction {
            Interaction::Pressed => {
                available.selected_index = Some(course_item.0);
            }
            Interaction::Hovered => {
                if available.selected_index != Some(course_item.0) {
                    *bg = BackgroundColor(HOVERED_COURSE);
                    *border = BorderColor::all(palette::CHAINMAIL);
                }
            }
            Interaction::None => {
                if available.selected_index != Some(course_item.0) {
                    *bg = BackgroundColor(NORMAL_COURSE);
                    *border = BorderColor::all(palette::STEEL);
                }
            }
        }
    }
}

pub fn update_course_highlights(
    available: Option<Res<AvailableCourses>>,
    mut course_query: Query<(&CourseItem, &mut BackgroundColor, &mut BorderColor)>,
    mut race_text_query: Query<&mut TextColor, (With<RaceButtonText>, Without<HintText>)>,
    mut hint_query: Query<(&mut Text, &mut TextColor), (With<HintText>, Without<RaceButtonText>)>,
) {
    let Some(available) = available else { return };
    if !available.is_changed() {
        return;
    }

    for (course_item, mut bg, mut border) in &mut course_query {
        if available.selected_index == Some(course_item.0) {
            *bg = BackgroundColor(SELECTED_COURSE);
            *border = BorderColor::all(palette::SKY);
        } else {
            *bg = BackgroundColor(NORMAL_COURSE);
            *border = BorderColor::all(palette::STEEL);
        }
    }

    let selected_course = available
        .selected_index
        .and_then(|idx| available.courses.get(idx));
    let raceable = selected_course.is_some_and(|c| c.gate_count >= MIN_RACEABLE_GATES);

    for mut text_color in &mut race_text_query {
        *text_color = if raceable {
            TextColor(palette::VANILLA)
        } else {
            TextColor(palette::CHAINMAIL)
        };
    }

    for (mut text, mut color) in &mut hint_query {
        match selected_course {
            Some(course) if course.gate_count < MIN_RACEABLE_GATES => {
                **text = format!(
                    "Course has {} gate{} — needs at least {} to race",
                    course.gate_count,
                    if course.gate_count == 1 { "" } else { "s" },
                    MIN_RACEABLE_GATES,
                );
                *color = TextColor(palette::BRONZE);
            }
            Some(_) => {
                **text = "Ready to race!".to_string();
                *color = TextColor(palette::MINT);
            }
            None => {
                **text = "Select a course to enable racing".to_string();
                *color = TextColor(palette::STONE);
            }
        }
    }
}

pub fn handle_editor_button(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<EditorButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
    last_edited: Option<Res<LastEditedCourse>>,
    available: Option<Res<AvailableCourses>>,
) {
    let Some(available) = available else { return };
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            if let Some(course) = available
                .selected_index
                .and_then(|idx| available.courses.get(idx))
            {
                commands.insert_resource(PendingEditorCourse {
                    path: course.path.clone(),
                });
            } else if let Some(ref last) = last_edited {
                commands.insert_resource(PendingEditorCourse {
                    path: last.path.clone(),
                });
            }
            next_state.set(AppState::Editor);
        }
    }
}

pub fn handle_race_button(
    query: Query<&Interaction, (Changed<Interaction>, With<RaceButton>)>,
    available: Option<Res<AvailableCourses>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let Some(available) = available else { return };
    for interaction in &query {
        if *interaction == Interaction::Pressed
            && let Some(idx) = available.selected_index
            && let Some(course) = available.courses.get(idx)
            && course.gate_count >= MIN_RACEABLE_GATES
        {
            commands.insert_resource(SelectedCourse {
                path: course.path.clone(),
            });
            next_state.set(AppState::Race);
        }
    }
}


pub fn cleanup_menu(mut commands: Commands) {
    commands.remove_resource::<AvailableCourses>();
}
