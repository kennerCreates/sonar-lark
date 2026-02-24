use std::fs;
use std::path::Path;

use bevy::prelude::*;

use crate::course::loader::{load_course_from_file, SelectedCourse};
use crate::editor::course_editor::{LastEditedCourse, PendingEditorCourse};
use crate::states::AppState;

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);
const SELECTED_COURSE: Color = Color::srgb(0.2, 0.4, 0.6);
const NORMAL_COURSE: Color = Color::srgb(0.1, 0.1, 0.1);
const HOVERED_COURSE: Color = Color::srgb(0.2, 0.2, 0.2);

const MIN_RACEABLE_GATES: usize = 3;

pub struct CourseEntry {
    pub name: String,
    pub path: String,
    pub gate_count: usize,
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

#[derive(Component)]
pub(crate) struct HintText;

fn discover_courses() -> Vec<CourseEntry> {
    discover_courses_in(Path::new("assets/courses"))
}

fn discover_courses_in(courses_dir: &Path) -> Vec<CourseEntry> {
    let mut courses = Vec::new();

    if let Ok(entries) = fs::read_dir(courses_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("ron") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    let display_name = name.trim_end_matches(".course").to_string();
                    let gate_count = load_course_from_file(&path)
                        .map(|c| {
                            c.instances
                                .iter()
                                .filter(|i| i.gate_order.is_some())
                                .count()
                        })
                        .unwrap_or(0);
                    courses.push(CourseEntry {
                        name: display_name,
                        path: path.to_string_lossy().to_string(),
                        gate_count,
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
                                        Text::new(format!(
                                            "{}  ({} gate{})",
                                            course.name,
                                            course.gate_count,
                                            if course.gate_count == 1 { "" } else { "s" }
                                        )),
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
                HintText,
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
    mut race_text_query: Query<&mut TextColor, (With<RaceButtonText>, Without<HintText>)>,
    mut hint_query: Query<(&mut Text, &mut TextColor), (With<HintText>, Without<RaceButtonText>)>,
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

    let selected_course = available
        .selected_index
        .and_then(|idx| available.courses.get(idx));
    let raceable = selected_course.is_some_and(|c| c.gate_count >= MIN_RACEABLE_GATES);

    for mut text_color in &mut race_text_query {
        *text_color = if raceable {
            TextColor(Color::srgb(0.9, 0.9, 0.9))
        } else {
            TextColor(Color::srgb(0.5, 0.5, 0.5))
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
                *color = TextColor(Color::srgb(0.8, 0.5, 0.2));
            }
            Some(_) => {
                **text = "Ready to race!".to_string();
                *color = TextColor(Color::srgb(0.4, 0.8, 0.4));
            }
            None => {
                **text = "Select a course to enable racing".to_string();
                *color = TextColor(Color::srgb(0.4, 0.4, 0.4));
            }
        }
    }
}

pub fn handle_editor_button(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<EditorButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
    last_edited: Option<Res<LastEditedCourse>>,
    available: Res<AvailableCourses>,
) {
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
    available: Res<AvailableCourses>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            if let Some(idx) = available.selected_index {
                if let Some(course) = available.courses.get(idx) {
                    if course.gate_count >= MIN_RACEABLE_GATES {
                        commands.insert_resource(SelectedCourse {
                            path: course.path.clone(),
                        });
                        next_state.set(AppState::Race);
                    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_course_ron() -> &'static str {
        r#"(name: "empty", instances: [])"#
    }

    fn course_ron_with_gates(gate_count: usize) -> String {
        let mut instances = String::new();
        for i in 0..gate_count {
            if i > 0 {
                instances.push_str(", ");
            }
            instances.push_str(&format!(
                r#"(obstacle_id: ("gate"), translation: (0.0, 0.0, {z}.0), rotation: (0.0, 0.0, 0.0, 1.0), scale: (1.0, 1.0, 1.0), gate_order: Some({i}), gate_forward_flipped: false)"#,
                z = -(i as i32) * 20,
                i = i,
            ));
        }
        format!(r#"(name: "test", instances: [{instances}])"#)
    }

    #[test]
    fn discover_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let courses = discover_courses_in(dir.path());
        assert!(courses.is_empty());
    }

    #[test]
    fn discover_nonexistent_directory() {
        let courses = discover_courses_in(Path::new("this/does/not/exist"));
        assert!(courses.is_empty());
    }

    #[test]
    fn discover_filters_ron_only() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("track.course.ron"), empty_course_ron()).unwrap();
        fs::write(dir.path().join("readme.txt"), "ignore me").unwrap();
        fs::write(dir.path().join("notes.md"), "ignore me too").unwrap();

        let courses = discover_courses_in(dir.path());
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].name, "track");
    }

    #[test]
    fn discover_strips_course_suffix() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("mountain.course.ron"), empty_course_ron()).unwrap();
        fs::write(dir.path().join("simple.ron"), empty_course_ron()).unwrap();

        let courses = discover_courses_in(dir.path());
        assert_eq!(courses.len(), 2);
        assert_eq!(courses[0].name, "mountain");
        assert_eq!(courses[1].name, "simple");
    }

    #[test]
    fn discover_results_sorted_alphabetically() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("zebra.course.ron"), empty_course_ron()).unwrap();
        fs::write(dir.path().join("alpha.course.ron"), empty_course_ron()).unwrap();
        fs::write(dir.path().join("middle.course.ron"), empty_course_ron()).unwrap();

        let courses = discover_courses_in(dir.path());
        let names: Vec<&str> = courses.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "middle", "zebra"]);
    }

    #[test]
    fn discover_stores_full_path() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.course.ron");
        fs::write(&file_path, empty_course_ron()).unwrap();

        let courses = discover_courses_in(dir.path());
        assert_eq!(courses.len(), 1);
        assert!(courses[0].path.contains("test.course.ron"));
    }

    #[test]
    fn discover_counts_gates() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("no_gates.course.ron"), empty_course_ron()).unwrap();
        fs::write(
            dir.path().join("two_gates.course.ron"),
            course_ron_with_gates(2),
        )
        .unwrap();
        fs::write(
            dir.path().join("five_gates.course.ron"),
            course_ron_with_gates(5),
        )
        .unwrap();

        let courses = discover_courses_in(dir.path());
        assert_eq!(courses.len(), 3);
        // Sorted: five_gates, no_gates, two_gates
        assert_eq!(courses[0].name, "five_gates");
        assert_eq!(courses[0].gate_count, 5);
        assert_eq!(courses[1].name, "no_gates");
        assert_eq!(courses[1].gate_count, 0);
        assert_eq!(courses[2].name, "two_gates");
        assert_eq!(courses[2].gate_count, 2);
    }

    #[test]
    fn discover_invalid_ron_gets_zero_gates() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("bad.course.ron"), "not valid ron").unwrap();

        let courses = discover_courses_in(dir.path());
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].name, "bad");
        assert_eq!(courses[0].gate_count, 0);
    }
}
