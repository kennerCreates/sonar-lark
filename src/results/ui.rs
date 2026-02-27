use bevy::prelude::*;

use crate::drone::spawning::{DRONE_COLORS, DRONE_NAMES};
use crate::palette;
use crate::pilot::SelectedPilots;
use crate::race::progress::RaceResults;
use crate::states::AppState;

const NORMAL_BUTTON: Color = palette::INDIGO;
const HOVERED_BUTTON: Color = palette::SAPPHIRE;
const PRESSED_BUTTON: Color = palette::GREEN;

#[derive(Component)]
pub(crate) struct RaceAgainButton;

#[derive(Component)]
pub(crate) struct MainMenuButton;

pub fn setup_results_ui(
    mut commands: Commands,
    results: Option<Res<RaceResults>>,
    selected: Option<Res<SelectedPilots>>,
) {
    let Some(results) = results else {
        warn!("No RaceResults resource found, skipping results UI.");
        return;
    };

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(12.0),
                ..default()
            },
            DespawnOnExit(AppState::Results),
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("RACE RESULTS"),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));

            // Course name
            parent.spawn((
                Text::new(&results.course_name),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
            ));

            // Total race time
            let mins = (results.total_time / 60.0) as u32;
            let secs = results.total_time % 60.0;
            parent.spawn((
                Text::new(format!("Race Time: {:01}:{:05.2}", mins, secs)),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(palette::STONE),
            ));

            // Standings container
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    margin: UiRect::vertical(Val::Px(12.0)),
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                })
                .with_children(|container| {
                    for (pos, entry) in results.standings.iter().enumerate() {
                        let drone_idx = entry.drone_index;
                        let (name, color) = if let Some(ref pilots) = selected {
                            pilots
                                .pilots
                                .get(drone_idx)
                                .map(|p| (p.gamertag.as_str(), p.color))
                                .unwrap_or(("???", palette::VANILLA))
                        } else {
                            (
                                DRONE_NAMES.get(drone_idx).copied().unwrap_or("???"),
                                DRONE_COLORS
                                    .get(drone_idx)
                                    .copied()
                                    .unwrap_or(palette::VANILLA),
                            )
                        };

                        let is_winner = pos == 0 && entry.finished;

                        container
                            .spawn(Node {
                                height: Val::Px(24.0),
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(6.0),
                                padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
                                ..default()
                            })
                            .with_children(|row| {
                                // Color bar
                                row.spawn((
                                    Node {
                                        width: Val::Px(4.0),
                                        height: Val::Percent(100.0),
                                        ..default()
                                    },
                                    BackgroundColor(color),
                                ));

                                // Position + name
                                let name_color = if is_winner {
                                    palette::LIMON
                                } else if entry.crashed {
                                    palette::STONE
                                } else {
                                    palette::VANILLA
                                };
                                row.spawn((
                                    Text::new(format!("{:>2}  {}", pos + 1, name)),
                                    TextFont {
                                        font_size: 15.0,
                                        ..default()
                                    },
                                    TextColor(name_color),
                                    Node {
                                        width: Val::Px(140.0),
                                        ..default()
                                    },
                                ));

                                // Time / DNF
                                let (time_text, time_color) = if entry.finished {
                                    let t = entry.finish_time.unwrap_or(0.0);
                                    let m = (t / 60.0) as u32;
                                    let s = t % 60.0;
                                    (format!("{:01}:{:05.2}", m, s), palette::SEA_FOAM)
                                } else if entry.crashed {
                                    ("DNF".to_string(), palette::NEON_RED)
                                } else {
                                    ("---".to_string(), palette::SIDEWALK)
                                };
                                row.spawn((
                                    Text::new(time_text),
                                    TextFont {
                                        font_size: 15.0,
                                        ..default()
                                    },
                                    TextColor(time_color),
                                    Node {
                                        width: Val::Px(72.0),
                                        ..default()
                                    },
                                ));

                                // Gates passed
                                row.spawn((
                                    Text::new(format!(
                                        "{}/{}",
                                        entry.gates_passed, results.total_gates
                                    )),
                                    TextFont {
                                        font_size: 15.0,
                                        ..default()
                                    },
                                    TextColor(palette::STONE),
                                ));
                            });
                    }
                });

            // Button row
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(20.0),
                    margin: UiRect::top(Val::Px(8.0)),
                    ..default()
                })
                .with_children(|row| {
                    spawn_results_button(row, "RACE AGAIN", RaceAgainButton);
                    spawn_results_button(row, "MAIN MENU", MainMenuButton);
                });
        });
}

fn spawn_results_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    marker: impl Component,
) {
    parent
        .spawn((
            Button,
            marker,
            Node {
                width: Val::Px(200.0),
                height: Val::Px(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(3.0)),
                ..default()
            },
            BackgroundColor(NORMAL_BUTTON),
            BorderColor::all(palette::STEEL),
        ))
        .with_children(|btn: &mut ChildSpawnerCommands| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));
        });
}

pub fn handle_race_again_button(
    query: Query<&Interaction, (Changed<Interaction>, With<RaceAgainButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Race);
        }
    }
}

pub fn handle_main_menu_button(
    query: Query<&Interaction, (Changed<Interaction>, With<MainMenuButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Menu);
        }
    }
}

pub fn handle_results_button_visuals(
    mut query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (
            Changed<Interaction>,
            Or<(With<RaceAgainButton>, With<MainMenuButton>)>,
        ),
    >,
) {
    for (interaction, mut bg, mut border) in &mut query {
        match *interaction {
            Interaction::Pressed => {
                *bg = BackgroundColor(PRESSED_BUTTON);
                *border = BorderColor::all(palette::VANILLA);
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(HOVERED_BUTTON);
                *border = BorderColor::all(palette::SIDEWALK);
            }
            Interaction::None => {
                *bg = BackgroundColor(NORMAL_BUTTON);
                *border = BorderColor::all(palette::STEEL);
            }
        }
    }
}
