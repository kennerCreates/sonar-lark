use bevy::prelude::*;

use crate::course::data::PropKind;
use crate::obstacle::library::ObstacleLibrary;
use crate::palette;
use crate::states::EditorMode;
use crate::ui_theme;

use super::right_panel::{build_right_panel, spawn_palette_button};
use super::types::*;

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
            BackgroundColor(ui_theme::PANEL_BG),
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

                    ui_theme::spawn_divider(content);

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
                            BackgroundColor(ui_theme::BUTTON_NORMAL),
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
                            BackgroundColor(ui_theme::BUTTON_NORMAL),
                            BorderColor::all(palette::SAPPHIRE),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("Add Camera to Gate"),
                                TextFont {
                                    font_size: 13.0,
                                    ..default()
                                },
                                TextColor(palette::SKY),
                            ));
                        });

                    content
                        .spawn((
                            Button,
                            RemoveCameraButton,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(28.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(ui_theme::BUTTON_NORMAL),
                            BorderColor::all(palette::STEEL),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("Remove Camera"),
                                TextFont {
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(palette::PEACH),
                            ));
                        });

                    ui_theme::spawn_divider(content);

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
                            BackgroundColor(ui_theme::BUTTON_NORMAL),
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

                    ui_theme::spawn_divider(content);

                    content.spawn((
                        Text::new("Select a gate, then add a camera.\nUse Move/Rotate to aim."),
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

            ui_theme::spawn_divider(panel);
            ui_theme::spawn_panel_button(panel, "Back to Menu", BackToMenuButton);
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
            BackgroundColor(if active { ui_theme::BUTTON_SELECTED } else { ui_theme::BUTTON_NORMAL }),
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
            BackgroundColor(ui_theme::BUTTON_NORMAL),
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
