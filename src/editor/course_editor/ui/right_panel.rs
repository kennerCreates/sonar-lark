use bevy::prelude::*;

use crate::editor::course_editor::TransformMode;
use crate::obstacle::definition::ObstacleId;
use crate::palette;

use super::types::*;

pub fn build_right_panel(parent: &mut ChildSpawnerCommands, existing_courses: &[CourseEntry]) {
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

pub fn spawn_small_button(parent: &mut ChildSpawnerCommands, label: &str, marker: impl Component) {
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

pub fn spawn_divider(parent: &mut ChildSpawnerCommands) {
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
