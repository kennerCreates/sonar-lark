use bevy::prelude::*;
use std::fs;
use std::path::Path;

use crate::course::data::PropKind;
use crate::editor::course_editor::TransformMode;
use crate::obstacle::definition::ObstacleId;
use crate::obstacle::library::ObstacleLibrary;
use crate::palette;
use crate::states::EditorMode;

use super::types::*;

pub fn discover_existing_courses() -> Vec<CourseEntry> {
    discover_existing_courses_in(Path::new("assets/courses"))
}

pub fn discover_existing_courses_in(courses_dir: &Path) -> Vec<CourseEntry> {
    let mut courses = Vec::new();
    if let Ok(entries) = fs::read_dir(courses_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("ron")
                && let Some(name) = path.file_stem().and_then(|s| s.to_str())
            {
                let display_name = name.trim_end_matches(".course").to_string();
                courses.push(CourseEntry {
                    display_name,
                    path: path.to_string_lossy().to_string(),
                });
            }
        }
    }
    courses.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    courses
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discover_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let courses = discover_existing_courses_in(dir.path());
        assert!(courses.is_empty());
    }

    #[test]
    fn discover_nonexistent_directory() {
        let courses = discover_existing_courses_in(Path::new("no_such_dir_xyz"));
        assert!(courses.is_empty());
    }

    #[test]
    fn discover_filters_ron_only() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.course.ron"), "()").unwrap();
        fs::write(dir.path().join("b.txt"), "not a course").unwrap();
        fs::write(dir.path().join("c.json"), "{}").unwrap();

        let courses = discover_existing_courses_in(dir.path());
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].display_name, "a");
    }

    #[test]
    fn discover_strips_course_suffix() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("my_track.course.ron"), "()").unwrap();

        let courses = discover_existing_courses_in(dir.path());
        assert_eq!(courses[0].display_name, "my_track");
    }

    #[test]
    fn discover_plain_ron_keeps_name() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("simple.ron"), "()").unwrap();

        let courses = discover_existing_courses_in(dir.path());
        assert_eq!(courses[0].display_name, "simple");
    }

    #[test]
    fn discover_results_sorted_alphabetically() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("charlie.course.ron"), "()").unwrap();
        fs::write(dir.path().join("alpha.course.ron"), "()").unwrap();
        fs::write(dir.path().join("bravo.course.ron"), "()").unwrap();

        let courses = discover_existing_courses_in(dir.path());
        let names: Vec<_> = courses.iter().map(|c| c.display_name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "bravo", "charlie"]);
    }

    #[test]
    fn discover_stores_full_path() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("test.course.ron"), "()").unwrap();

        let courses = discover_existing_courses_in(dir.path());
        assert!(courses[0].path.contains("test.course.ron"));
    }
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
                TextColor(palette::VANILLA),
            ));

            // --- Tab row ---
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(4.0),
                    width: Val::Percent(100.0),
                    margin: UiRect::top(Val::Px(8.0)),
                    ..default()
                })
                .with_children(|row| {
                    spawn_tab_button(row, "Obstacles", ObstacleTabButton, true);
                    spawn_tab_button(row, "Props", PropsTabButton, false);
                    spawn_tab_button(row, "Cameras", CamerasTabButton, false);
                });

            // --- Obstacle palette content (visible by default) ---
            panel
                .spawn((
                    ObstaclePaletteContent,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                ))
                .with_children(|content| {
                    content
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
                                    Text::new(
                                        "No obstacles in library.\nGo to Obstacle Workshop first.",
                                    ),
                                    TextFont {
                                        font_size: 13.0,
                                        ..default()
                                    },
                                    TextColor(palette::CHAINMAIL),
                                ));
                            } else {
                                let mut ids: Vec<_> = library.definitions.keys().collect();
                                ids.sort_by(|a, b| a.0.cmp(&b.0));
                                for id in ids {
                                    spawn_palette_button(container, id);
                                }
                            }
                        });
                });

            // --- Props palette content (hidden by default) ---
            panel
                .spawn((
                    PropPaletteContent,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        display: Display::None,
                        ..default()
                    },
                ))
                .with_children(|content| {
                    spawn_prop_palette_button(content, "Confetti Emitter", PropKind::ConfettiEmitter);
                    spawn_prop_palette_button(
                        content,
                        "Shell Burst Emitter",
                        PropKind::ShellBurstEmitter,
                    );

                    spawn_divider(content);

                    content.spawn((
                        Text::new("Color: Auto"),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(palette::SUNSHINE),
                        PropColorLabel,
                    ));

                    content
                        .spawn((
                            Button,
                            PropColorButton,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(28.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(BUTTON_NORMAL),
                            BorderColor::all(palette::STEEL),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("Cycle Color"),
                                TextFont {
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(palette::SAND),
                            ));
                        });
                });

            // --- Camera palette content (hidden by default) ---
            panel
                .spawn((
                    CameraPaletteContent,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        display: Display::None,
                        ..default()
                    },
                ))
                .with_children(|content| {
                    content
                        .spawn((
                            Button,
                            PlaceCameraButton,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(28.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(BUTTON_NORMAL),
                            BorderColor::all(palette::SAPPHIRE),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("Place Camera"),
                                TextFont {
                                    font_size: 13.0,
                                    ..default()
                                },
                                TextColor(palette::SKY),
                            ));
                        });

                    spawn_divider(content);

                    content.spawn((
                        Text::new("Primary: (select a camera)"),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(palette::CHAINMAIL),
                        CameraPrimaryLabel,
                    ));

                    content
                        .spawn((
                            Button,
                            CameraPrimaryToggle,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(28.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(BUTTON_NORMAL),
                            BorderColor::all(palette::STEEL),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("Toggle Primary"),
                                TextFont {
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(palette::SAND),
                            ));
                        });

                    spawn_divider(content);

                    content.spawn((
                        Text::new("Use Move/Rotate to aim"),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(palette::CHAINMAIL),
                    ));
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

fn spawn_tab_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    marker: impl Component,
    active: bool,
) {
    parent
        .spawn((
            Button,
            marker,
            Node {
                flex_grow: 1.0,
                height: Val::Px(30.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(if active { BUTTON_SELECTED } else { BUTTON_NORMAL }),
            BorderColor::all(palette::STEEL),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));
        });
}

fn spawn_prop_palette_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    kind: PropKind,
) {
    let accent = match kind {
        PropKind::ConfettiEmitter => palette::SUNSHINE,
        PropKind::ShellBurstEmitter => palette::TANGERINE,
    };
    parent
        .spawn((
            Button,
            PropPaletteButton(kind),
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(28.0),
                padding: UiRect::horizontal(Val::Px(8.0)),
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(BUTTON_NORMAL),
            BorderColor::all(palette::SAPPHIRE),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(accent),
            ));
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
                TextColor(palette::SIDEWALK),
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
                    BackgroundColor(palette::BLACK),
                    BorderColor::all(palette::STEEL),
                ))
                .with_children(|field| {
                    field.spawn((
                        Text::new("new_course"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(palette::SAND),
                        CourseNameDisplayText,
                    ));
                });

            spawn_small_button(panel, "New Course", NewCourseButton);
            spawn_action_button(
                panel,
                "Save Course",
                SaveCourseButton,
                palette::JUNGLE,
            );

            spawn_divider(panel);

            panel.spawn((
                Text::new("Load Existing"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
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
                            TextColor(palette::CHAINMAIL),
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
                TextColor(palette::SIDEWALK),
            ));

            panel.spawn((
                Text::new("Gates: 0 (loop)"),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(palette::SKY),
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
                    BorderColor::all(palette::STEEL),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("Gate Mode: OFF"),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(palette::VANILLA),
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
                TextColor(palette::SIDEWALK),
            ));

            panel
                .spawn((Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(4.0),
                    width: Val::Percent(100.0),
                    ..default()
                },))
                .with_children(|row| {
                    spawn_transform_mode_button(row, "Move (1)", TransformMode::Move);
                    spawn_transform_mode_button(row, "Rotate (2)", TransformMode::Rotate);
                    spawn_transform_mode_button(row, "Scale (3)", TransformMode::Scale);
                });

            spawn_divider(panel);

            panel.spawn((
                Text::new("Del  →  delete selected"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(palette::CHAINMAIL),
            ));

            panel.spawn((
                Text::new("LMB obstacle  →  select"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(palette::CHAINMAIL),
            ));

            panel.spawn((
                Text::new("LMB palette + empty  →  place"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(palette::CHAINMAIL),
            ));

            panel.spawn((
                Text::new("Gate mode: LMB to assign order"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(palette::CHAINMAIL),
            ));

            panel.spawn((
                Text::new("F  →  flip gate direction"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(palette::CHAINMAIL),
            ));

            panel.spawn((
                Text::new("Shift  →  Y-move / axis-scale / Z-rotate"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(palette::CHAINMAIL),
            ));

            panel.spawn((
                Text::new("Ctrl  →  X-rotate"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(palette::CHAINMAIL),
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
            BorderColor::all(palette::SAPPHIRE),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(&id.0),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(palette::MINT),
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
                BorderColor::all(palette::SAPPHIRE),
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new(display_name),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(palette::SAND),
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
                BackgroundColor(palette::BURGUNDY),
                BorderColor::all(palette::EGGPLANT),
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new("X"),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(palette::SALMON),
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
            BorderColor::all(palette::STEEL),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(palette::SAND),
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
            BorderColor::all(palette::STEEL),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
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
        BackgroundColor(palette::STEEL),
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
            BorderColor::all(palette::STEEL),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(palette::SAND),
            ));
        });
}
