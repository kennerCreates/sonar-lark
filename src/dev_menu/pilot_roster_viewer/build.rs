use bevy::prelude::*;

use crate::palette;
use crate::pilot::roster::PilotRoster;
use crate::pilot::Pilot;
use crate::states::DevMenuPage;
use crate::ui_theme::ThemedButton;

use super::{
    DeletePilotButton, ObstacleWorkshopButton, PaletteEditorButton, PilotGeneratorButton,
    PilotRow, RosterBackButton, RosterCountLabel, RosterListContainer,
    format_skill,
};

const PANEL_BG: Color = Color::srgba(0.02, 0.04, 0.08, 0.95);

pub fn build_ui(
    commands: &mut Commands,
    roster: &PilotRoster,
    ui_font: &Handle<Font>,
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
                        spawn_header_button(btns, "BACK", RosterBackButton, &ui_font);
                    });
            });

            // ── Column headers ─────────────────────────────────────────
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                width: Val::Percent(100.0),
                padding: UiRect::horizontal(Val::Px(8.0)),
                column_gap: Val::Px(8.0),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|row| {
                spawn_column_header(row, "", 32.0, &ui_font);   // color swatch
                spawn_column_header(row, "GAMERTAG", 180.0, &ui_font);
                spawn_column_header(row, "SKILL", 120.0, &ui_font);
                spawn_column_header(row, "PERSONALITY", 160.0, &ui_font);
                spawn_column_header(row, "RACES", 50.0, &ui_font);
                spawn_column_header(row, "WINS", 50.0, &ui_font);
                spawn_column_header(row, "CRASHES", 60.0, &ui_font);
                spawn_column_header(row, "", 80.0, &ui_font);   // delete button
            });

            // ── Scrollable list ────────────────────────────────────────
            root.spawn((
                RosterListContainer,
                Node {
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::scroll_y(),
                    row_gap: Val::Px(2.0),
                    ..default()
                },
            ))
            .with_children(|container| {
                build_pilot_rows(container, roster, &ui_font);
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
) {
    for pilot in &roster.pilots {
        spawn_pilot_row(parent, pilot, ui_font);
    }
}

fn spawn_pilot_row(
    parent: &mut ChildSpawnerCommands,
    pilot: &Pilot,
    ui_font: &Handle<Font>,
) {
    let pilot_id = pilot.id;
    let [r, g, b] = pilot.color_scheme.primary;

    parent
        .spawn((
            PilotRow,
            Node {
                flex_direction: FlexDirection::Row,
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(8.0)),
                column_gap: Val::Px(8.0),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.07, 0.12, 0.8)),
        ))
        .with_children(|row| {
            // Color swatch
            row.spawn((
                Node {
                    width: Val::Px(24.0),
                    height: Val::Px(24.0),
                    border: UiRect::all(Val::Px(1.0)),
                    flex_shrink: 0.0,
                    ..default()
                },
                BackgroundColor(Color::srgb(r, g, b)),
                BorderColor::all(palette::STEEL),
            ));

            // Gamertag
            row.spawn((
                Node {
                    width: Val::Px(180.0),
                    ..default()
                },
                Text::new(&pilot.gamertag),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));

            // Skill
            row.spawn((
                Node {
                    width: Val::Px(120.0),
                    ..default()
                },
                Text::new(format_skill(&pilot.skill)),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(palette::SAND),
            ));

            // Personality
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
            row.spawn((
                Node {
                    width: Val::Px(160.0),
                    ..default()
                },
                Text::new(personality_str),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(palette::SAND),
            ));

            // Races
            row.spawn((
                Node {
                    width: Val::Px(50.0),
                    ..default()
                },
                Text::new(pilot.stats.races_entered.to_string()),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
            ));

            // Wins
            row.spawn((
                Node {
                    width: Val::Px(50.0),
                    ..default()
                },
                Text::new(pilot.stats.wins.to_string()),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
            ));

            // Crashes
            row.spawn((
                Node {
                    width: Val::Px(60.0),
                    ..default()
                },
                Text::new(pilot.stats.crashes.to_string()),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
            ));

            // Delete button
            row.spawn((
                Button,
                ThemedButton,
                DeletePilotButton(pilot_id),
                Node {
                    width: Val::Px(80.0),
                    height: Val::Px(28.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(palette::BURGUNDY),
                BorderColor::all(palette::STEEL),
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new("DELETE"),
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

fn spawn_column_header(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    width: f32,
    ui_font: &Handle<Font>,
) {
    parent.spawn((
        Node {
            width: Val::Px(width),
            ..default()
        },
        Text::new(label),
        TextFont {
            font: ui_font.clone(),
            font_size: 11.0,
            ..default()
        },
        TextColor(palette::CHAINMAIL),
    ));
}
