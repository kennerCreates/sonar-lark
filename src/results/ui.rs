use bevy::prelude::*;

use crate::common::drone_identity::{resolve_drone_color, resolve_drone_name};
use crate::drone::components::{Drone, DroneConfig, DroneIdentity, RaceSeed};
use crate::drone::spawning::ReplayRequest;
use crate::palette;
use crate::pilot::{PilotConfigs, SelectedPilots};
use crate::pilot::portrait::cache::PortraitCache;
use crate::race::progress::RaceResults;
use crate::states::AppState;
use crate::ui_theme;

#[derive(Component)]
pub(crate) struct ReplayRaceButton;

#[derive(Component)]
pub(crate) struct NewRaceButton;

#[derive(Component)]
pub(crate) struct MainMenuButton;

pub fn setup_results_ui(
    mut commands: Commands,
    results: Option<Res<RaceResults>>,
    selected: Option<Res<SelectedPilots>>,
    portrait_cache: Option<Res<PortraitCache>>,
    drones: Query<(&Drone, &DroneIdentity)>,
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

            // Standings — two columns
            let half = results.standings.len().div_ceil(2);
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(24.0),
                    margin: UiRect::vertical(Val::Px(12.0)),
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                })
                .with_children(|columns| {
                    for col in 0..2 {
                        let start = col * half;
                        let end = (start + half).min(results.standings.len());
                        columns
                            .spawn(Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(2.0),
                                ..default()
                            })
                            .with_children(|container| {
                                for pos in start..end {
                                    let entry = &results.standings[pos];
                                    let drone_idx = entry.drone_index;
                                    let identity = drones.iter()
                                        .find(|(d, _)| d.index as usize == drone_idx)
                                        .map(|(_, id)| id);
                                    let name = resolve_drone_name(selected.as_deref(), drone_idx, identity);
                                    let color = resolve_drone_color(selected.as_deref(), drone_idx, identity);

                                    let is_winner = pos == 0 && entry.finished;

                                    container
                                        .spawn(Node {
                                            height: Val::Px(64.0),
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

                                            // Portrait thumbnail
                                            let portrait_handle = selected.as_ref()
                                                .and_then(|s| s.pilots.get(drone_idx))
                                                .and_then(|sel| {
                                                    portrait_cache.as_ref()
                                                        .and_then(|cache| cache.get(sel.pilot_id))
                                                });
                                            if let Some(handle) = portrait_handle {
                                                row.spawn((
                                                    ImageNode::new(handle),
                                                    Node {
                                                        width: Val::Px(64.0),
                                                        height: Val::Px(64.0),
                                                        ..default()
                                                    },
                                                ));
                                            } else {
                                                row.spawn((
                                                    Node {
                                                        width: Val::Px(64.0),
                                                        height: Val::Px(64.0),
                                                        ..default()
                                                    },
                                                    BackgroundColor(color),
                                                ));
                                            }

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
                                                (ui_theme::fmt_time(t), palette::SEA_FOAM)
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
                    ui_theme::spawn_menu_button(row, "REPLAY RACE", ReplayRaceButton, 220.0);
                    ui_theme::spawn_menu_button(row, "NEW RACE", NewRaceButton, 180.0);
                    ui_theme::spawn_menu_button(row, "MAIN MENU", MainMenuButton, 200.0);
                });
        });
}


pub fn handle_replay_button(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<ReplayRaceButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
    race_seed: Option<Res<RaceSeed>>,
    drones: Query<(&Drone, &DroneConfig)>,
    selected_pilots: Option<Res<SelectedPilots>>,
    pilot_configs: Option<Res<PilotConfigs>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            if let Some(seed) = &race_seed {
                let mut configs: Vec<(u8, DroneConfig)> =
                    drones.iter().map(|(d, c)| (d.index, c.clone())).collect();
                configs.sort_by_key(|(idx, _)| *idx);
                commands.insert_resource(ReplayRequest {
                    race_seed: seed.0,
                    drone_configs: configs.into_iter().map(|(_, c)| c).collect(),
                    selected_pilots: selected_pilots
                        .as_ref()
                        .map(|s| s.pilots.clone())
                        .unwrap_or_default(),
                    pilot_configs: pilot_configs
                        .as_ref()
                        .map(|p| p.configs.clone())
                        .unwrap_or_default(),
                });
            }
            next_state.set(AppState::Race);
        }
    }
}

pub fn handle_new_race_button(
    query: Query<&Interaction, (Changed<Interaction>, With<NewRaceButton>)>,
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

