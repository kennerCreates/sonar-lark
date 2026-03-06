use bevy::prelude::*;

use crate::palette;
use crate::pilot::roster::PilotRoster;
use crate::pilot::Pilot;
use crate::states::DevMenuPage;
use crate::ui_theme::ThemedButton;

use super::{
    ClearRosterButton, DeletePilotButton, ObstacleWorkshopButton, PaletteEditorButton,
    PilotGeneratorButton, PilotRow, RosterBackButton, RosterCountLabel, RosterListContainer,
    RosterPortraitCache, format_skill,
};

const PANEL_BG: Color = Color::srgba(0.02, 0.04, 0.08, 0.95);
const CARD_BG: Color = Color::srgba(0.05, 0.07, 0.12, 0.8);
const PORTRAIT_PX: f32 = 48.0;

pub fn build_ui(
    commands: &mut Commands,
    roster: &PilotRoster,
    ui_font: &Handle<Font>,
    portraits: &RosterPortraitCache,
) {
    let ui_font = ui_font.clone();
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
            DespawnOnExit(DevMenuPage::PilotRosterViewer),
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
                    Text::new("PILOT ROSTER"),
                    TextFont {
                        font: ui_font.clone(),
                        font_size: 24.0,
                        ..default()
                    },
                    TextColor(palette::VANILLA),
                ));

                header
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|btns| {
                        spawn_header_button(btns, "PILOT GENERATOR", PilotGeneratorButton, &ui_font);
                        spawn_header_button(btns, "OBSTACLE WORKSHOP", ObstacleWorkshopButton, &ui_font);
                        spawn_header_button(btns, "PALETTE EDITOR", PaletteEditorButton, &ui_font);
                        spawn_header_button(btns, "CLEAR", ClearRosterButton, &ui_font);
                        spawn_header_button(btns, "BACK", RosterBackButton, &ui_font);
                    });
            });

            // ── Scrollable 2-column grid ────────────────────────────────
            root.spawn((
                RosterListContainer,
                Node {
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    overflow: Overflow::scroll_y(),
                    row_gap: Val::Px(4.0),
                    column_gap: Val::Px(4.0),
                    align_content: AlignContent::FlexStart,
                    ..default()
                },
            ))
            .with_children(|container| {
                build_pilot_rows(container, roster, &ui_font, portraits);
            });

            // ── Footer ─────────────────────────────────────────────────
            root.spawn((
                RosterCountLabel,
                Text::new(format!("Roster: {} pilots", roster.pilots.len())),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
            ));
        });
}

pub fn build_pilot_rows(
    parent: &mut ChildSpawnerCommands,
    roster: &PilotRoster,
    ui_font: &Handle<Font>,
    portraits: &RosterPortraitCache,
) {
    for pilot in &roster.pilots {
        spawn_pilot_card(parent, pilot, ui_font, portraits);
    }
}

fn spawn_pilot_card(
    parent: &mut ChildSpawnerCommands,
    pilot: &Pilot,
    ui_font: &Handle<Font>,
    portraits: &RosterPortraitCache,
) {
    let pilot_id = pilot.id;
    let [r, g, b] = pilot.color_scheme.primary;

    parent
        .spawn((
            PilotRow,
            Node {
                flex_direction: FlexDirection::Row,
                // ~49.5% width to get 2 columns with gap
                width: Val::Percent(49.5),
                padding: UiRect::all(Val::Px(6.0)),
                column_gap: Val::Px(8.0),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(CARD_BG),
        ))
        .with_children(|card| {
            // Portrait thumbnail
            if let Some(handle) = portraits.portraits.get(&pilot_id) {
                card.spawn((
                    ImageNode::new(handle.clone()),
                    Node {
                        width: Val::Px(PORTRAIT_PX),
                        height: Val::Px(PORTRAIT_PX),
                        flex_shrink: 0.0,
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(palette::STEEL),
                ));
            } else {
                // Fallback: color swatch
                card.spawn((
                    Node {
                        width: Val::Px(PORTRAIT_PX),
                        height: Val::Px(PORTRAIT_PX),
                        flex_shrink: 0.0,
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(r, g, b)),
                    BorderColor::all(palette::STEEL),
                ));
            }

            // Info column
            card.spawn(Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                row_gap: Val::Px(2.0),
                ..default()
            })
            .with_children(|info| {
                // Gamertag + color swatch row
                info.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Node {
                            width: Val::Px(12.0),
                            height: Val::Px(12.0),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(r, g, b)),
                    ));
                    row.spawn((
                        Text::new(&pilot.gamertag),
                        TextFont {
                            font: ui_font.clone(),
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(palette::VANILLA),
                    ));
                });

                // Skill + personality
                let personality_str = if pilot.personality.is_empty() {
                    "None".to_string()
                } else {
                    pilot
                        .personality
                        .iter()
                        .map(|t| format!("{t:?}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                info.spawn((
                    Text::new(format!("{} | {}", format_skill(&pilot.skill), personality_str)),
                    TextFont {
                        font: ui_font.clone(),
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(palette::SAND),
                ));

                // Stats row
                info.spawn((
                    Text::new(format!(
                        "R:{} W:{} C:{}",
                        pilot.stats.races_entered, pilot.stats.wins, pilot.stats.crashes,
                    )),
                    TextFont {
                        font: ui_font.clone(),
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(palette::SIDEWALK),
                ));
            });

            // Delete button
            card.spawn((
                Button,
                ThemedButton,
                DeletePilotButton(pilot_id),
                Node {
                    width: Val::Px(24.0),
                    height: Val::Px(24.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    flex_shrink: 0.0,
                    ..default()
                },
                BackgroundColor(palette::BURGUNDY),
                BorderColor::all(palette::STEEL),
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new("X"),
                    TextFont {
                        font: ui_font.clone(),
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(palette::VANILLA),
                ));
            });
        });
}

fn spawn_header_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    marker: impl Component,
    ui_font: &Handle<Font>,
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
                    font: ui_font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));
        });
}
