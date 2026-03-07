use std::path::Path;

use bevy::prelude::*;

use crate::course::data::PropKind;
use crate::obstacle::definition::ObstacleId;
use crate::obstacle::library::ObstacleLibrary;
use crate::palette;
use crate::states::EditorMode;
use crate::ui_theme;

use super::right_panel::build_right_panel;
use super::types::*;

pub fn load_obstacle_thumbnails(
    asset_server: &AssetServer,
    library: &ObstacleLibrary,
) -> ObstacleThumbnails {
    let mut images = std::collections::HashMap::new();
    for id in library.definitions.keys() {
        let path_str = format!("library/thumbnails/{}.png", id.0);
        let full_path = format!("assets/{path_str}");
        if Path::new(&full_path).exists() {
            let handle: Handle<Image> = asset_server.load(path_str);
            images.insert(id.clone(), handle);
        }
    }
    ObstacleThumbnails { images }
}

const THUMB_CELL_SIZE: f32 = 112.0;

pub fn build_course_editor_ui(
    commands: &mut Commands,
    library: &ObstacleLibrary,
    font: &Handle<Font>,
    thumbnails: &ObstacleThumbnails,
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
            build_left_panel(root, library, font, thumbnails);
            build_right_panel(root, font);
        });
}

fn build_left_panel(parent: &mut ChildSpawnerCommands, library: &ObstacleLibrary, font: &Handle<Font>, thumbnails: &ObstacleThumbnails) {
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
                    font: font.clone(),
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
                    spawn_tab_button(row, "Obstacles", ObstacleTabButton, true, font);
                    spawn_tab_button(row, "Props", PropsTabButton, false, font);
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
                                flex_direction: FlexDirection::Row,
                                flex_wrap: FlexWrap::Wrap,
                                column_gap: Val::Px(4.0),
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
                                        font: font.clone(),
                                        font_size: 13.0,
                                        ..default()
                                    },
                                    TextColor(palette::CHAINMAIL),
                                ));
                            } else {
                                let mut ids: Vec<_> = library.definitions.keys().collect();
                                ids.sort_by(|a, b| a.0.cmp(&b.0));
                                for id in ids {
                                    spawn_palette_card(container, id, font, library, thumbnails);
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
                    spawn_prop_palette_button(content, "Confetti Emitter", PropKind::ConfettiEmitter, font);
                    spawn_prop_palette_button(
                        content,
                        "Shell Burst Emitter",
                        PropKind::ShellBurstEmitter,
                        font,
                    );

                    ui_theme::spawn_divider(content);

                    content.spawn((
                        Text::new("Color: Auto"),
                        TextFont {
                            font: font.clone(),
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
                                    font: font.clone(),
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(palette::SAND),
                            ));
                        });
                });

            panel.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });

            ui_theme::spawn_divider(panel);
            ui_theme::spawn_panel_button(panel, "Back to Menu", BackToMenuButton, font);
        });
}

fn spawn_tab_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    marker: impl Component,
    active: bool,
    font: &Handle<Font>,
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
                    font: font.clone(),
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
    font: &Handle<Font>,
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
                    font: font.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(accent),
            ));
        });
}

fn spawn_palette_card(
    parent: &mut ChildSpawnerCommands,
    id: &ObstacleId,
    font: &Handle<Font>,
    library: &ObstacleLibrary,
    thumbnails: &ObstacleThumbnails,
) {
    let cost = crate::course::data::gate_cost(&id.0, library);
    let cost_label = if cost > 0 {
        format!("${cost}")
    } else {
        String::new()
    };

    parent
        .spawn((
            Button,
            PaletteButton(id.clone()),
            Node {
                width: Val::Px(THUMB_CELL_SIZE),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(3.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(ui_theme::BUTTON_NORMAL),
            BorderColor::all(palette::SAPPHIRE),
        ))
        .with_children(|card| {
            if let Some(image_handle) = thumbnails.images.get(id) {
                card.spawn((
                    ImageNode::new(image_handle.clone()),
                    Node {
                        width: Val::Px(THUMB_CELL_SIZE - 8.0),
                        height: Val::Px(THUMB_CELL_SIZE - 8.0),
                        ..default()
                    },
                ));
            } else {
                // Fallback: colored box with name
                card.spawn((
                    Node {
                        width: Val::Px(THUMB_CELL_SIZE - 8.0),
                        height: Val::Px(THUMB_CELL_SIZE - 8.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(palette::INDIGO),
                ))
                .with_children(|fallback| {
                    fallback.spawn((
                        Text::new(&id.0),
                        TextFont {
                            font: font.clone(),
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(palette::CHAINMAIL),
                    ));
                });
            }

            // Name label
            card.spawn((
                Text::new(&id.0),
                TextFont {
                    font: font.clone(),
                    font_size: 10.0,
                    ..default()
                },
                TextColor(palette::MINT),
                Node {
                    margin: UiRect::top(Val::Px(2.0)),
                    ..default()
                },
            ));

            // Cost label
            if !cost_label.is_empty() {
                card.spawn((
                    Text::new(cost_label),
                    TextFont {
                        font: font.clone(),
                        font_size: 10.0,
                        ..default()
                    },
                    TextColor(palette::SUNSHINE),
                ));
            }
        });
}
