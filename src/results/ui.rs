use bevy::prelude::*;

use crate::common::drone_identity::{resolve_drone_color, resolve_drone_name, DRONE_COUNT};
use crate::course::loader::SelectedCourse;
use crate::drone::components::{Drone, DroneConfig, DroneIdentity, RaceSeed};
use crate::drone::spawning::ReplayRequest;
use crate::league::fan_network::{FanTier, RaceAttractionResult};
use crate::league::recruitment::accessible_tier;
use crate::league::LeagueState;
use crate::palette;
use crate::pilot::{PilotConfigs, SelectedPilots};
use crate::pilot::portrait::cache::PortraitCache;
use crate::race::progress::RaceProgress;
use crate::race::timing::RaceClock;
use crate::race::track_quality::TrackQuality;
use crate::course::data::CourseData;
use crate::course::location::LocationRegistry;
use crate::states::AppState;
use crate::ui_theme::{self, UiFont};

#[derive(Component)]
pub(crate) struct ReplayRaceButton;

#[derive(Component)]
pub(crate) struct ContinueButton;

#[derive(Component)]
pub(crate) struct NewRaceButton;

#[derive(Component)]
pub(crate) struct ViewFanNetworkButton;

#[derive(Component)]
pub(crate) struct FanNetworkOverlay;

#[derive(Component)]
pub(crate) struct CloseFanNetworkButton;

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
    track_quality: Option<Res<TrackQuality>>,
    attraction: Option<Res<RaceAttractionResult>>,
    league: Option<Res<LeagueState>>,
    course_data: Option<Res<CourseData>>,
    location_registry: Res<LocationRegistry>,
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

            // League stats panel
            {
                let ui_font_stats = ui_font.clone();
                spawn_league_stats_panel(
                    parent,
                    &ui_font_stats,
                    track_quality.as_deref(),
                    attraction.as_deref(),
                    league.as_deref(),
                    course_data.as_deref(),
                    &location_registry,
                );
            }

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
                    ui_theme::spawn_menu_button(row, "VIEW FANS", ViewFanNetworkButton, 180.0, &ui_font);
                    ui_theme::spawn_menu_button(row, "NEW RACE", NewRaceButton, 180.0, &ui_font);
                    ui_theme::spawn_menu_button(row, "CONTINUE", ContinueButton, 180.0, &ui_font);
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

fn spawn_league_stats_panel(
    parent: &mut ChildSpawnerCommands,
    ui_font: &Handle<Font>,
    track_quality: Option<&TrackQuality>,
    attraction: Option<&RaceAttractionResult>,
    league: Option<&LeagueState>,
    course_data: Option<&CourseData>,
    location_registry: &LocationRegistry,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(24.0),
            margin: UiRect::vertical(Val::Px(4.0)),
            ..default()
        })
        .with_children(|row| {
            // Track Quality column
            if let Some(tq) = track_quality {
                spawn_stat_column(row, ui_font, "TRACK QUALITY", &[
                    ("Gates", tq.gate_count_score),
                    ("Variety", tq.obstacle_variety_score),
                    ("Turn Mix", tq.turn_mix_score),
                    ("Elevation", tq.elevation_score),
                    ("Spectacle", tq.gate_spectacle_score),
                ], Some(("Overall", tq.overall)));
            }

            // Venue & Attendance column
            {
                let location_name = course_data
                    .map(|c| c.location.as_str())
                    .unwrap_or("Abandoned Warehouse");
                let location = location_registry.get(location_name);
                let attractiveness = location.map(|l| l.base_attractiveness).unwrap_or(0.2);
                let capacity = location.map(|l| l.capacity).unwrap_or(40);

                let mut lines: Vec<String> = vec![
                    format!("Venue: {location_name}"),
                    format!("Appeal: {:.0}%", attractiveness * 100.0),
                    format!("Capacity: {capacity}"),
                ];

                if let Some(att) = attraction {
                    lines.push(format!(
                        "Attended: {} / {} wanted",
                        att.actual_attendance, att.demand
                    ));
                    if att.turned_away > 0 {
                        lines.push(format!("Turned away: {}", att.turned_away));
                    }
                    let ticket_price = league.map(|l| l.ticket_price).unwrap_or(0);
                    let ticket_revenue = att.actual_attendance as f32 * ticket_price as f32;
                    lines.push(format!("Tickets: +${ticket_revenue:.0}"));
                }

                spawn_text_column(row, ui_font, "VENUE", &lines);
            }

            // Fan Network column
            if let Some(att) = attraction {
                let tier = league
                    .map(|l| accessible_tier(l.fan_network.fan_count()).label)
                    .unwrap_or("Amateur");
                let total_money = league.map(|l| l.money).unwrap_or(0.0);

                let mut lines = vec![
                    format!("Network: {} people", att.network_size),
                    format!("Fans: {}", att.fan_count),
                    format!("League Tier: {tier}"),
                    format!("New spreads: {}", att.new_aware_from_spread),
                    format!("Promotions: {} / Demotions: {}", att.promotions, att.demotions),
                ];
                if att.removed > 0 {
                    lines.push(format!("Lost interest: {}", att.removed));
                }
                lines.push(format!("Total Money: ${total_money:.0}"));

                spawn_text_column(row, ui_font, "FAN NETWORK", &lines);
            }
        });
}

fn spawn_stat_column(
    parent: &mut ChildSpawnerCommands,
    ui_font: &Handle<Font>,
    title: &str,
    scores: &[(&str, f32)],
    overall: Option<(&str, f32)>,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            ..default()
        })
        .with_children(|col| {
            col.spawn((
                Text::new(title),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
            ));

            for &(label, score) in scores {
                let bar_width = (score * 60.0).round();
                col.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(4.0),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Text::new(format!("{label:>12}")),
                        TextFont {
                            font: ui_font.clone(),
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(palette::SAND),
                        Node {
                            width: Val::Px(80.0),
                            ..default()
                        },
                    ));
                    // Bar background
                    row.spawn(Node {
                        width: Val::Px(62.0),
                        height: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|bar_bg| {
                        bar_bg.spawn((
                            Node {
                                width: Val::Px(60.0),
                                height: Val::Px(8.0),
                                ..default()
                            },
                            BackgroundColor(palette::INDIGO),
                        ));
                        bar_bg.spawn((
                            Node {
                                width: Val::Px(bar_width),
                                height: Val::Px(8.0),
                                position_type: PositionType::Absolute,
                                ..default()
                            },
                            BackgroundColor(score_color(score)),
                        ));
                    });
                });
            }

            if let Some((label, score)) = overall {
                col.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(4.0),
                    margin: UiRect::top(Val::Px(2.0)),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Text::new(format!("{label:>12}")),
                        TextFont {
                            font: ui_font.clone(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(palette::VANILLA),
                        Node {
                            width: Val::Px(80.0),
                            ..default()
                        },
                    ));
                    row.spawn((
                        Text::new(format!("{:.0}%", score * 100.0)),
                        TextFont {
                            font: ui_font.clone(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(score_color(score)),
                    ));
                });
            }
        });
}

fn spawn_text_column(
    parent: &mut ChildSpawnerCommands,
    ui_font: &Handle<Font>,
    title: &str,
    lines: &[String],
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            ..default()
        })
        .with_children(|col| {
            col.spawn((
                Text::new(title),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
            ));
            for line in lines {
                col.spawn((
                    Text::new(line.as_str()),
                    TextFont {
                        font: ui_font.clone(),
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(palette::SAND),
                ));
            }
        });
}

fn score_color(score: f32) -> Color {
    if score >= 0.7 {
        palette::SEA_FOAM
    } else if score >= 0.4 {
        palette::SUNSHINE
    } else {
        palette::NEON_RED
    }
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
    query: Query<&Interaction, (Changed<Interaction>, With<NewRaceButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Race);
        }
    }
}

pub fn handle_continue_button(
    query: Query<&Interaction, (Changed<Interaction>, With<ContinueButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Bounties);
        }
    }
}

pub fn handle_view_fan_network_button(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<ViewFanNetworkButton>)>,
    existing: Query<Entity, With<FanNetworkOverlay>>,
    league: Option<Res<LeagueState>>,
    font: Res<UiFont>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        // Toggle: if overlay exists, despawn it
        if let Ok(entity) = existing.single() {
            commands.entity(entity).despawn();
            return;
        }
        let Some(league) = &league else { return };
        spawn_fan_network_overlay(&mut commands, &league.fan_network, &font.0);
    }
}

pub fn handle_close_fan_network(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<CloseFanNetworkButton>)>,
    overlay: Query<Entity, With<FanNetworkOverlay>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed
            && let Ok(entity) = overlay.single()
        {
            commands.entity(entity).despawn();
        }
    }
}

fn tier_label(tier: FanTier) -> &'static str {
    match tier {
        FanTier::Aware => "Aware",
        FanTier::Attendee => "Attendee",
        FanTier::Fan => "Fan",
        FanTier::Superfan => "Superfan",
    }
}

fn tier_color(tier: FanTier) -> Color {
    match tier {
        FanTier::Aware => palette::FROG,
        FanTier::Attendee => palette::SUNSHINE,
        FanTier::Fan => palette::TANGERINE,
        FanTier::Superfan => palette::NEON_RED,
    }
}

// 35 male, 35 female, 30 gender-neutral = 100 names.
// Deterministic pick via person_id.
const FAN_NAMES: [&str; 100] = [
    // Male (0..35)
    "James", "Michael", "Robert", "David", "John", "William", "Thomas", "Daniel",
    "Matthew", "Andrew", "Christopher", "Joseph", "Ryan", "Nathan", "Kevin", "Brian",
    "Eric", "Mark", "Steven", "Patrick", "Jason", "Timothy", "Sean", "Kyle",
    "Brandon", "Justin", "Aaron", "Derek", "Tyler", "Luke", "Marcus", "Grant",
    "Ian", "Owen", "Cole",
    // Female (35..70)
    "Emma", "Olivia", "Sarah", "Jessica", "Ashley", "Emily", "Hannah", "Megan",
    "Rachel", "Lauren", "Samantha", "Nicole", "Stephanie", "Amanda", "Kayla", "Brianna",
    "Natalie", "Victoria", "Grace", "Rebecca", "Chloe", "Julia", "Sophia", "Madison",
    "Lily", "Abigail", "Ella", "Claire", "Hailey", "Zoe", "Leah", "Nora",
    "Violet", "Audrey", "Maya",
    // Gender-neutral (70..100)
    "Alex", "Jordan", "Taylor", "Morgan", "Casey", "Riley", "Avery", "Quinn",
    "Skyler", "Dakota", "Rowan", "Sage", "Finley", "Hayden", "Jamie", "Reese",
    "Parker", "Blake", "Charlie", "Drew", "Emery", "Kendall", "Peyton", "River",
    "Cameron", "Frankie", "Harley", "Kai", "Remy", "Wren",
];

fn fan_name(person_id: u32) -> &'static str {
    // Simple hash to spread IDs across the name list
    let h = person_id.wrapping_mul(2654435761);
    FAN_NAMES[(h as usize) % FAN_NAMES.len()]
}

fn tier_badge_size(tier: FanTier) -> (f32, f32, f32) {
    // (dot_size, font_size, padding)
    match tier {
        FanTier::Superfan => (12.0, 13.0, 6.0),
        FanTier::Fan => (10.0, 12.0, 5.0),
        FanTier::Attendee => (8.0, 11.0, 4.0),
        FanTier::Aware => (6.0, 10.0, 3.0),
    }
}

fn spawn_fan_network_overlay(
    commands: &mut Commands,
    network: &crate::league::fan_network::FanNetwork,
    ui_font: &Handle<Font>,
) {
    let mut counts = [0u32; 4];
    for person in &network.people {
        let idx = match person.tier {
            FanTier::Aware => 0,
            FanTier::Attendee => 1,
            FanTier::Fan => 2,
            FanTier::Superfan => 3,
        };
        counts[idx] += 1;
    }

    commands
        .spawn((
            FanNetworkOverlay,
            DespawnOnExit(AppState::Results),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            ZIndex(10),
        ))
        .with_children(|backdrop| {
            backdrop
                .spawn((
                    Node {
                        width: Val::Px(620.0),
                        max_height: Val::Percent(85.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(16.0)),
                        row_gap: Val::Px(8.0),
                        border_radius: BorderRadius::all(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.02, 0.04, 0.08, 0.95)),
                ))
                .with_children(|panel| {
                    // Header
                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|header| {
                            header.spawn((
                                Text::new("FAN NETWORK"),
                                TextFont {
                                    font: ui_font.clone(),
                                    font_size: 24.0,
                                    ..default()
                                },
                                TextColor(palette::VANILLA),
                            ));
                            ui_theme::spawn_menu_button(
                                header,
                                "CLOSE",
                                CloseFanNetworkButton,
                                80.0,
                                ui_font,
                            );
                        });

                    // Tier legend row
                    let tiers = [
                        (FanTier::Superfan, counts[3]),
                        (FanTier::Fan, counts[2]),
                        (FanTier::Attendee, counts[1]),
                        (FanTier::Aware, counts[0]),
                    ];
                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(16.0),
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|row| {
                            for (tier, count) in tiers {
                                let (dot, _, _) = tier_badge_size(tier);
                                row.spawn(Node {
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(4.0),
                                    align_items: AlignItems::Center,
                                    ..default()
                                })
                                .with_children(|item| {
                                    // Color dot
                                    item.spawn((
                                        Node {
                                            width: Val::Px(dot),
                                            height: Val::Px(dot),
                                            border_radius: BorderRadius::MAX,
                                            ..default()
                                        },
                                        BackgroundColor(tier_color(tier)),
                                    ));
                                    item.spawn((
                                        Text::new(format!("{}: {count}", tier_label(tier))),
                                        TextFont {
                                            font: ui_font.clone(),
                                            font_size: 12.0,
                                            ..default()
                                        },
                                        TextColor(tier_color(tier)),
                                    ));
                                });
                            }
                        });

                    panel.spawn((
                        Text::new(format!("{} people in network", network.people.len())),
                        TextFont {
                            font: ui_font.clone(),
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(palette::STONE),
                    ));

                    // Flowing badge grid (scrollable)
                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: Val::Px(6.0),
                            row_gap: Val::Px(6.0),
                            overflow: Overflow::scroll_y(),
                            padding: UiRect::all(Val::Px(8.0)),
                            ..default()
                        })
                        .with_children(|grid| {
                            // Sort: highest tier first, then by id
                            let mut sorted: Vec<_> = network.people.iter().collect();
                            sorted.sort_by(|a, b| {
                                let ta = tier_sort_key(a.tier);
                                let tb = tier_sort_key(b.tier);
                                tb.cmp(&ta).then(a.id.cmp(&b.id))
                            });

                            for person in &sorted {
                                let color = tier_color(person.tier);
                                let (dot_sz, font_sz, pad) = tier_badge_size(person.tier);
                                let name = fan_name(person.id);

                                grid.spawn((
                                    Node {
                                        flex_direction: FlexDirection::Row,
                                        align_items: AlignItems::Center,
                                        column_gap: Val::Px(3.0),
                                        padding: UiRect::axes(
                                            Val::Px(pad + 2.0),
                                            Val::Px(pad),
                                        ),
                                        border_radius: BorderRadius::all(Val::Px(12.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.08, 0.1, 0.15, 0.8)),
                                ))
                                .with_children(|badge| {
                                    // Color dot
                                    badge.spawn((
                                        Node {
                                            width: Val::Px(dot_sz),
                                            height: Val::Px(dot_sz),
                                            border_radius: BorderRadius::MAX,
                                            ..default()
                                        },
                                        BackgroundColor(color),
                                    ));
                                    // Name
                                    badge.spawn((
                                        Text::new(name),
                                        TextFont {
                                            font: ui_font.clone(),
                                            font_size: font_sz,
                                            ..default()
                                        },
                                        TextColor(color),
                                    ));
                                });
                            }
                        });
                });
        });
}

fn tier_sort_key(tier: FanTier) -> u8 {
    match tier {
        FanTier::Aware => 0,
        FanTier::Attendee => 1,
        FanTier::Fan => 2,
        FanTier::Superfan => 3,
    }
}
