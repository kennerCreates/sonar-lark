use bevy::prelude::*;

use crate::common::drone_identity::{resolve_drone_color, resolve_drone_name, DRONE_COUNT};
use crate::course::loader::SelectedCourse;
use crate::drone::components::{Drone, DroneConfig, DroneIdentity, RaceSeed};
use crate::drone::spawning::ReplayRequest;
use crate::palette;
use crate::pilot::{PilotConfigs, SelectedPilots};
use crate::pilot::portrait::cache::PortraitCache;
use crate::race::progress::RaceProgress;
use crate::race::timing::RaceClock;
use crate::menu::ui::SkipToLocationSelect;
use crate::states::AppState;
use crate::ui_theme::{self, UiFont};

#[derive(Component)]
pub(crate) struct ReplayRaceButton;

#[derive(Component)]
pub(crate) struct NewRaceButton;

#[derive(Component)]
pub(crate) struct ResultsRaceTime;

/// Tags a name+position text in a results row. Index is the row position (0..11).
#[derive(Component)]
pub(crate) struct RsNameText(usize);

/// Tags a time/status text in a results row.
#[derive(Component)]
pub(crate) struct RsTimeText(usize);

/// Tags a gates-passed text in a results row.
#[derive(Component)]
pub(crate) struct RsGatesText(usize);

/// Tags the color bar in a results row.
#[derive(Component)]
pub(crate) struct RsColorBar(usize);

/// Tags the portrait slot in a results row.
#[derive(Component)]
pub(crate) struct RsPortrait(usize);

const DRONE_N: usize = DRONE_COUNT as usize;

pub fn setup_results_ui(
    mut commands: Commands,
    progress: Option<Res<RaceProgress>>,
    selected_course: Option<Res<SelectedCourse>>,
    selected: Option<Res<SelectedPilots>>,
    portrait_cache: Option<Res<PortraitCache>>,
    drones: Query<(&Drone, &DroneIdentity)>,
    font: Res<UiFont>,
) {
    if progress.is_none() {
        warn!("No RaceProgress resource found, skipping results UI.");
        return;
    }

    let course_name = selected_course
        .map(|s| {
            std::path::Path::new(&s.path)
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .trim_end_matches(".course")
                .to_string()
        })
        .unwrap_or_else(|| "Unknown".to_string());

    let ui_font = font.0.clone();
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
                    font: ui_font.clone(),
                    font_size: 48.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));

            // Course name
            parent.spawn((
                Text::new(course_name),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 20.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
            ));

            // Total race time (updated live)
            parent.spawn((
                ResultsRaceTime,
                Text::new("Race Time: --:--.--"),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(palette::STONE),
            ));

            // Standings — two columns, DRONE_N rows total
            let half = DRONE_N.div_ceil(2);
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
                        let end = (start + half).min(DRONE_N);
                        columns
                            .spawn(Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(2.0),
                                ..default()
                            })
                            .with_children(|container| {
                                for pos in start..end {
                                    spawn_result_row(
                                        container, pos, &ui_font,
                                        selected.as_deref(), portrait_cache.as_deref(), &drones,
                                    );
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
                    ui_theme::spawn_menu_button(row, "REPLAY RACE", ReplayRaceButton, 220.0, &ui_font);
                    ui_theme::spawn_menu_button(row, "NEW RACE", NewRaceButton, 180.0, &ui_font);
                });
        });
}

fn spawn_result_row(
    container: &mut ChildSpawnerCommands,
    pos: usize,
    ui_font: &Handle<Font>,
    selected: Option<&SelectedPilots>,
    portrait_cache: Option<&PortraitCache>,
    drones: &Query<(&Drone, &DroneIdentity)>,
) {
    // Use drone index 0 as placeholder — update_results_ui will fill real data
    let identity = drones.iter()
        .find(|(d, _)| d.index as usize == pos)
        .map(|(_, id)| id);
    let init_name = resolve_drone_name(selected, pos, identity);
    let init_color = resolve_drone_color(selected, pos, identity);

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
                RsColorBar(pos),
                Node {
                    width: Val::Px(4.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(init_color),
            ));

            // Portrait thumbnail
            let portrait_handle = selected
                .and_then(|s| s.pilots.get(pos))
                .and_then(|sel| portrait_cache.and_then(|cache| cache.get(sel.pilot_id)));
            if let Some(handle) = portrait_handle {
                row.spawn((
                    RsPortrait(pos),
                    ImageNode::new(handle),
                    Node {
                        width: Val::Px(64.0),
                        height: Val::Px(64.0),
                        ..default()
                    },
                ));
            } else {
                row.spawn((
                    RsPortrait(pos),
                    Node {
                        width: Val::Px(64.0),
                        height: Val::Px(64.0),
                        ..default()
                    },
                    BackgroundColor(init_color),
                ));
            }

            // Position + name
            row.spawn((
                RsNameText(pos),
                Text::new(format!("{:>2}  {}", pos + 1, init_name)),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 15.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
                Node {
                    width: Val::Px(140.0),
                    ..default()
                },
            ));

            // Time / DNF
            row.spawn((
                RsTimeText(pos),
                Text::new("---"),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 15.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
                Node {
                    width: Val::Px(72.0),
                    ..default()
                },
            ));

            // Gates passed
            row.spawn((
                RsGatesText(pos),
                Text::new("0/0"),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 15.0,
                    ..default()
                },
                TextColor(palette::STONE),
            ));
        });
}

/// Updates the results UI every frame from live RaceProgress data.
pub(crate) fn update_results_ui(
    progress: Option<Res<RaceProgress>>,
    clock: Option<Res<RaceClock>>,
    selected: Option<Res<SelectedPilots>>,
    portrait_cache: Option<Res<PortraitCache>>,
    drones: Query<(&Drone, &DroneIdentity)>,
    mut race_time_text: Query<&mut Text, With<ResultsRaceTime>>,
    mut name_texts: Query<
        (&RsNameText, &mut Text, &mut TextColor),
        (Without<RsTimeText>, Without<RsGatesText>, Without<ResultsRaceTime>),
    >,
    mut time_texts: Query<
        (&RsTimeText, &mut Text, &mut TextColor),
        (Without<RsNameText>, Without<RsGatesText>, Without<ResultsRaceTime>),
    >,
    mut gates_texts: Query<
        (&RsGatesText, &mut Text),
        (Without<RsNameText>, Without<RsTimeText>, Without<ResultsRaceTime>),
    >,
    mut color_bars: Query<(&RsColorBar, &mut BackgroundColor), Without<RsPortrait>>,
    mut portraits: Query<(&RsPortrait, Option<&mut ImageNode>, &mut BackgroundColor), Without<RsColorBar>>,
) {
    let Some(progress) = progress else { return };

    // Update race time header
    let elapsed = clock.map(|c| c.elapsed).unwrap_or(0.0);
    if let Ok(mut text) = race_time_text.single_mut() {
        text.0 = format!("Race Time: {}", ui_theme::fmt_time(elapsed));
    }

    let standings = progress.standings();
    let total_gates = progress.total_gates;

    // Build row data indexed by position
    let mut row_data = [(0usize, false, false, false, 0.0f32, 0u32); DRONE_N];
    for (pos, &(drone_idx, state)) in standings.iter().enumerate() {
        if pos >= DRONE_N { break; }
        row_data[pos] = (
            drone_idx,
            true,
            state.finished,
            state.crashed,
            state.finish_time.unwrap_or(0.0),
            state.gates_passed,
        );
    }

    for (nt, mut text, mut tc) in &mut name_texts {
        let pos = nt.0;
        if pos < DRONE_N && row_data[pos].1 {
            let (drone_idx, _, finished, crashed, _, _) = row_data[pos];
            let identity = drones.iter()
                .find(|(d, _)| d.index as usize == drone_idx)
                .map(|(_, id)| id);
            let name = resolve_drone_name(selected.as_deref(), drone_idx, identity);
            text.0 = format!("{:>2}  {}", pos + 1, name);
            let is_winner = pos == 0 && finished;
            *tc = if is_winner {
                TextColor(palette::LIMON)
            } else if crashed {
                TextColor(palette::STONE)
            } else {
                TextColor(palette::VANILLA)
            };
        }
    }

    for (tt, mut text, mut tc) in &mut time_texts {
        let pos = tt.0;
        if pos < DRONE_N && row_data[pos].1 {
            let (_, _, finished, crashed, finish_time, _) = row_data[pos];
            if finished {
                text.0 = ui_theme::fmt_time(finish_time);
                *tc = TextColor(palette::SEA_FOAM);
            } else if crashed {
                text.0 = "DNF".into();
                *tc = TextColor(palette::NEON_RED);
            } else {
                text.0 = "---".into();
                *tc = TextColor(palette::SIDEWALK);
            }
        }
    }

    for (gt, mut text) in &mut gates_texts {
        let pos = gt.0;
        if pos < DRONE_N && row_data[pos].1 {
            let (_, _, _, _, _, gates_passed) = row_data[pos];
            text.0 = format!("{}/{}", gates_passed, total_gates);
        }
    }

    for (cb, mut bg) in &mut color_bars {
        let pos = cb.0;
        if pos < DRONE_N && row_data[pos].1 {
            let drone_idx = row_data[pos].0;
            let identity = drones.iter()
                .find(|(d, _)| d.index as usize == drone_idx)
                .map(|(_, id)| id);
            *bg = BackgroundColor(resolve_drone_color(selected.as_deref(), drone_idx, identity));
        }
    }

    for (portrait, image_node, mut bg) in &mut portraits {
        let pos = portrait.0;
        if pos < DRONE_N && row_data[pos].1 {
            let drone_idx = row_data[pos].0;
            if let Some(mut img) = image_node {
                if let Some(handle) = selected
                    .as_ref()
                    .and_then(|s| s.pilots.get(drone_idx))
                    .and_then(|sel| {
                        portrait_cache
                            .as_ref()
                            .and_then(|cache| cache.get(sel.pilot_id))
                    })
                {
                    img.image = handle;
                }
            } else {
                let identity = drones.iter()
                    .find(|(d, _)| d.index as usize == drone_idx)
                    .map(|(_, id)| id);
                *bg = BackgroundColor(resolve_drone_color(selected.as_deref(), drone_idx, identity));
            }
        }
    }
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
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<NewRaceButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            commands.insert_resource(SkipToLocationSelect);
            next_state.set(AppState::Menu);
        }
    }
}
