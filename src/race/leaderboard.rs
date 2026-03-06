use bevy::prelude::*;

use crate::common::drone_identity::{resolve_drone_color, resolve_drone_name};
use crate::drone::components::{Drone, DroneIdentity};
use crate::palette;
use crate::pilot::SelectedPilots;
use crate::pilot::portrait::cache::PortraitCache;
use crate::states::AppState;
use crate::ui_theme::{self, UiFont};

use super::lifecycle::RacePhase;
use super::progress::RaceProgress;
use super::timing::RaceClock;

const LB_BG: Color = Color::srgba(0.02, 0.055, 0.102, 0.80);
const LB_FONT: f32 = 13.0;
const LB_WIDTH: f32 = 280.0;
const LB_ROW_HEIGHT: f32 = 64.0;

#[derive(Component)]
pub(crate) struct LeaderboardRoot;

#[derive(Component)]
pub(crate) struct LbNameText(usize);

#[derive(Component)]
pub(crate) struct LbTimeText(usize);

#[derive(Component)]
pub(crate) struct LbPortrait(usize);

const LB_PORTRAIT_SIZE: f32 = 64.0;

pub fn setup_leaderboard(
    mut commands: Commands,
    selected: Option<Res<SelectedPilots>>,
    portrait_cache: Option<Res<PortraitCache>>,
    drones: Query<(&Drone, &DroneIdentity)>,
    font: Res<UiFont>,
) {
    let ui_font = font.0.clone();
    commands
        .spawn((
            LeaderboardRoot,
            DespawnOnExit(AppState::Race),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                width: Val::Px(LB_WIDTH),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(6.0)),
                row_gap: Val::Px(2.0),
                ..default()
            },
            BackgroundColor(LB_BG),
            GlobalZIndex(90),
        ))
        .with_children(|panel| {
            for i in 0..12usize {
                panel
                    .spawn(Node {
                        height: Val::Px(LB_ROW_HEIGHT),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(4.0),
                        padding: UiRect::right(Val::Px(4.0)),
                        ..default()
                    })
                    .with_children(|row| {
                        let identity = drones.iter()
                            .find(|(d, _)| d.index as usize == i)
                            .map(|(_, id)| id);
                        let init_name = resolve_drone_name(selected.as_deref(), i, identity);
                        let init_color = resolve_drone_color(selected.as_deref(), i, identity);
                        // Portrait thumbnail
                        let portrait_handle = selected.as_ref()
                            .and_then(|s| s.pilots.get(i))
                            .and_then(|sel| {
                                portrait_cache.as_ref()
                                    .and_then(|cache| cache.get(sel.pilot_id))
                            });
                        if let Some(handle) = portrait_handle {
                            row.spawn((
                                LbPortrait(i),
                                ImageNode {
                                    image: handle,
                                    image_mode: NodeImageMode::Stretch,
                                    ..default()
                                },
                                Node {
                                    width: Val::Px(LB_PORTRAIT_SIZE),
                                    height: Val::Px(LB_PORTRAIT_SIZE),
                                    flex_shrink: 0.0,
                                    ..default()
                                },
                            ));
                        } else {
                            // Fallback: solid color square
                            row.spawn((
                                LbPortrait(i),
                                Node {
                                    width: Val::Px(LB_PORTRAIT_SIZE),
                                    height: Val::Px(LB_PORTRAIT_SIZE),
                                    flex_shrink: 0.0,
                                    ..default()
                                },
                                BackgroundColor(init_color),
                            ));
                        }
                        // Position + name
                        row.spawn((
                            LbNameText(i),
                            Text::new(format!("{:>2}  {}", i + 1, init_name)),
                            TextFont {
                                font: ui_font.clone(),
                                font_size: LB_FONT,
                                ..default()
                            },
                            TextColor(palette::VANILLA),
                            Node {
                                flex_grow: 1.0,
                                ..default()
                            },
                        ));
                        // Time / status
                        row.spawn((
                            LbTimeText(i),
                            Text::new("--:--.--"),
                            TextFont {
                                font: ui_font.clone(),
                                font_size: LB_FONT,
                                ..default()
                            },
                            TextColor(palette::SIDEWALK),
                        ));
                    });
            }
        });
}

pub fn update_leaderboard(
    phase: Res<RacePhase>,
    progress: Option<Res<RaceProgress>>,
    clock: Option<Res<RaceClock>>,
    selected: Option<Res<SelectedPilots>>,
    portrait_cache: Option<Res<PortraitCache>>,
    drones: Query<(&Drone, &DroneIdentity)>,
    mut root_vis: Query<&mut Visibility, With<LeaderboardRoot>>,
    mut name_texts: Query<
        (&LbNameText, &mut Text, &mut TextColor),
        (Without<LbTimeText>, Without<LbPortrait>),
    >,
    mut time_texts: Query<
        (&LbTimeText, &mut Text, &mut TextColor),
        (Without<LbNameText>, Without<LbPortrait>),
    >,
    mut portraits: Query<(&LbPortrait, Option<&mut ImageNode>, &mut BackgroundColor), (Without<LbNameText>, Without<LbTimeText>)>,
) {
    for mut vis in &mut root_vis {
        *vis = Visibility::Inherited;
    }

    let has_standings = matches!(*phase, RacePhase::Racing | RacePhase::Finished);
    if !has_standings {
        return;
    }

    let Some(progress) = progress else { return };
    let elapsed = clock.map(|c| c.elapsed).unwrap_or(0.0);
    let standings = progress.standings();

    // Build position->data lookup (avoids O(n^2) nested query iteration)
    let mut row_data = [(0usize, false, false, false, 0.0f32); 12];
    for (pos, &(drone_idx, state)) in standings.iter().enumerate() {
        if pos >= 12 { break; }
        row_data[pos] = (drone_idx, true, state.finished, state.crashed, state.finish_time.unwrap_or(0.0));
    }

    for (portrait, image_node, mut bg) in &mut portraits {
        let pos = portrait.0;
        if pos < 12 && row_data[pos].1 {
            let drone_idx = row_data[pos].0;
            if let Some(mut img) = image_node {
                // Update portrait image if cache has one for this pilot
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
                // Fallback: update background color
                let identity = drones.iter()
                    .find(|(d, _)| d.index as usize == drone_idx)
                    .map(|(_, id)| id);
                *bg = BackgroundColor(resolve_drone_color(selected.as_deref(), drone_idx, identity));
            }
        }
    }
    for (nt, mut text, mut tc) in &mut name_texts {
        let pos = nt.0;
        if pos < 12 && row_data[pos].1 {
            let (drone_idx, _, _, crashed, _) = row_data[pos];
            let identity = drones.iter()
                .find(|(d, _)| d.index as usize == drone_idx)
                .map(|(_, id)| id);
            let name = resolve_drone_name(selected.as_deref(), drone_idx, identity);
            text.0 = format!("{:>2}  {}", pos + 1, name);
            *tc = if crashed {
                TextColor(palette::STONE)
            } else {
                TextColor(palette::VANILLA)
            };
        }
    }
    for (tt, mut text, mut tc) in &mut time_texts {
        let pos = tt.0;
        if pos < 12 && row_data[pos].1 {
            let (_, _, finished, crashed, finish_time) = row_data[pos];
            if finished {
                text.0 = ui_theme::fmt_time(finish_time);
                *tc = TextColor(palette::SEA_FOAM);
            } else if crashed {
                text.0 = "DNF".into();
                *tc = TextColor(palette::NEON_RED);
            } else {
                text.0 = ui_theme::fmt_time(elapsed);
                *tc = TextColor(palette::SIDEWALK);
            }
        }
    }
}
