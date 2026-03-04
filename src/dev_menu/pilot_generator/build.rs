use bevy::prelude::*;

use crate::palette;
use crate::states::DevMenuPage;
use crate::ui_theme::ThemedButton;

use super::{
    AcceptButton, DroneColorSwatch, GamertagLabel, GenBackButton, ObstacleWorkshopButton,
    PaletteEditorButton, PersonalityLabel, PilotGeneratorState, PreviewImage,
    RerollGamertagButton, RerollPersonalityButton, RerollPortraitButton, RosterCountLabel,
    format_personality,
};

const PANEL_BG: Color = Color::srgba(0.02, 0.04, 0.08, 0.95);

pub fn build_ui(
    commands: &mut Commands,
    state: &PilotGeneratorState,
    roster_count: usize,
    preview_handle: Option<Handle<Image>>,
) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            DespawnOnExit(DevMenuPage::PilotGenerator),
        ))
        .with_children(|root| {
            // ── Header row ─────────────────────────────────────────────
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                width: Val::Percent(100.0),
                ..default()
            })
            .with_children(|header| {
                header.spawn((
                    Text::new("PILOT GENERATOR"),
                    TextFont {
                        font_size: 24.0,
                        ..default()
                    },
                    TextColor(palette::VANILLA),
                ));

                // Right side nav buttons
                header
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|btns| {
                        spawn_header_button(btns, "OBSTACLE WORKSHOP", ObstacleWorkshopButton);
                        spawn_header_button(btns, "PALETTE EDITOR", PaletteEditorButton);
                        spawn_header_button(btns, "BACK", GenBackButton);
                    });
            });

            // ── Main content row ───────────────────────────────────────
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(24.0),
                flex_grow: 1.0,
                ..default()
            })
            .with_children(|main| {
                // Left column: portrait preview
                main.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(8.0),
                    width: Val::Px(520.0),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|left| {
                    let mut preview_cmd = left.spawn((
                        PreviewImage,
                        Node {
                            width: Val::Px(512.0),
                            height: Val::Px(512.0),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BorderColor::all(palette::STEEL),
                        BackgroundColor(palette::SMOKY_BLACK),
                    ));
                    if let Some(handle) = preview_handle {
                        preview_cmd.insert(ImageNode::new(handle));
                    }
                });

                // Right column: pilot info + action buttons
                main.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(12.0),
                    padding: UiRect::top(Val::Px(8.0)),
                    ..default()
                })
                .with_children(|right| {
                    // Gamertag
                    right
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(8.0),
                            align_items: AlignItems::Baseline,
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                Text::new("Gamertag:"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(palette::SIDEWALK),
                            ));
                            row.spawn((
                                GamertagLabel,
                                Text::new(state.candidate.gamertag.clone()),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(palette::VANILLA),
                            ));
                        });

                    // Personality
                    right
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(8.0),
                            align_items: AlignItems::Baseline,
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                Text::new("Personality:"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(palette::SIDEWALK),
                            ));
                            row.spawn((
                                PersonalityLabel,
                                Text::new(format_personality(&state.candidate.personality)),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(palette::VANILLA),
                            ));
                        });

                    // Drone color swatch
                    right
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(8.0),
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                Text::new("Drone Color:"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(palette::SIDEWALK),
                            ));
                            let [r, g, b] = state.candidate.color_scheme.primary;
                            row.spawn((
                                DroneColorSwatch,
                                Node {
                                    width: Val::Px(32.0),
                                    height: Val::Px(32.0),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(r, g, b)),
                                BorderColor::all(palette::STEEL),
                            ));
                        });

                    // Spacer
                    right.spawn(Node {
                        height: Val::Px(16.0),
                        ..default()
                    });

                    // Action buttons
                    spawn_action_button(
                        right,
                        "REROLL PORTRAIT",
                        RerollPortraitButton,
                        palette::INDIGO,
                    );
                    spawn_action_button(
                        right,
                        "REROLL NAME",
                        RerollGamertagButton,
                        palette::INDIGO,
                    );
                    spawn_action_button(
                        right,
                        "REROLL PERSONALITY",
                        RerollPersonalityButton,
                        palette::INDIGO,
                    );
                    spawn_action_button(right, "ACCEPT", AcceptButton, palette::FROG);
                });
            });

            // ── Footer ─────────────────────────────────────────────────
            root.spawn((
                RosterCountLabel,
                Text::new(format!("Roster: {} pilots", roster_count)),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
            ));
        });
}

fn spawn_header_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    marker: impl Component,
) {
    parent
        .spawn((
            Button,
            ThemedButton,
            marker,
            Node {
                height: Val::Px(36.0),
                padding: UiRect::horizontal(Val::Px(12.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(palette::INDIGO),
            BorderColor::all(palette::STEEL),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
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
            ThemedButton,
            marker,
            Node {
                width: Val::Px(220.0),
                height: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
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
