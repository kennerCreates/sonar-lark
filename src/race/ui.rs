use bevy::prelude::*;

use crate::camera::switching::{CameraMode, CameraState, CourseCameras};
use crate::course::loader::SelectedCourse;
use crate::drone::components::*;
use crate::drone::spawning::{DRONE_COLORS, DRONE_NAMES, NoGatesCourse};
use crate::editor::course_editor::PendingEditorCourse;
use crate::palette;
use crate::states::AppState;

use super::lifecycle::{CountdownTimer, RacePhase};
use super::progress::RaceProgress;
use super::timing::RaceClock;

const NORMAL_BUTTON: Color = palette::INDIGO;
const HOVERED_BUTTON: Color = palette::SAPPHIRE;
const PRESSED_BUTTON: Color = palette::GREEN;
const DISABLED_BUTTON: Color = palette::SMOKY_BLACK;

#[derive(Component)]
pub(crate) struct StartRaceButton;

#[derive(Component)]
pub(crate) struct StartRaceButtonText;

#[derive(Component)]
pub(crate) struct CountdownText;

#[derive(Component)]
pub(crate) struct CountdownTextContent;

#[derive(Component)]
pub(crate) struct RaceClockText;

#[derive(Component)]
pub(crate) struct RaceClockTextContent;

pub fn setup_race_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexEnd,
                padding: UiRect::bottom(Val::Px(40.0)),
                ..default()
            },
            DespawnOnExit(AppState::Race),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Button,
                    StartRaceButton,
                    Node {
                        width: Val::Px(220.0),
                        height: Val::Px(60.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(NORMAL_BUTTON),
                    BorderColor::all(palette::STEEL),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("START RACE"),
                        TextFont {
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(palette::VANILLA),
                        StartRaceButtonText,
                    ));
                });
        });
}

pub fn handle_start_race_button(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<StartRaceButton>)>,
    mut phase: ResMut<RacePhase>,
    drones: Query<Entity, With<Drone>>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match *phase {
            RacePhase::WaitingToStart => {
                *phase = RacePhase::Countdown;
                commands.insert_resource(CountdownTimer::default());
                info!("Countdown started!");
            }
            RacePhase::Countdown | RacePhase::Racing => {}
            RacePhase::Finished => {
                // Despawn all drones so spawn_drones re-runs next frame
                // with a fresh RaceSeed and new randomized configs/splines.
                for entity in &drones {
                    commands.entity(entity).despawn();
                }
                commands.remove_resource::<RaceProgress>();
                commands.remove_resource::<RaceClock>();
                commands.remove_resource::<super::lifecycle::ResultsTransitionTimer>();
                *phase = RacePhase::WaitingToStart;
                info!("Race reset — drones will respawn with new randomization");
            }
        }
    }
}

pub fn update_start_button_visuals(
    phase: Res<RacePhase>,
    mut button_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        With<StartRaceButton>,
    >,
    interaction_changed: Query<(), (Changed<Interaction>, With<StartRaceButton>)>,
) {
    if !phase.is_changed() && interaction_changed.is_empty() {
        return;
    }
    for (interaction, mut bg, mut border) in &mut button_query {
        match *phase {
            RacePhase::Countdown | RacePhase::Racing => {
                *bg = BackgroundColor(DISABLED_BUTTON);
                *border = BorderColor::all(palette::INDIGO);
            }
            RacePhase::WaitingToStart | RacePhase::Finished => match *interaction {
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
            },
        }
    }
}

pub fn update_start_button_text(
    phase: Res<RacePhase>,
    mut text_query: Query<(&mut Text, &mut TextColor), With<StartRaceButtonText>>,
) {
    if !phase.is_changed() {
        return;
    }
    for (mut text, mut color) in &mut text_query {
        match *phase {
            RacePhase::WaitingToStart => {
                text.0 = "START RACE".to_string();
                *color = TextColor(palette::VANILLA);
            }
            RacePhase::Countdown => {
                text.0 = "GET READY...".to_string();
                *color = TextColor(palette::STONE);
            }
            RacePhase::Racing => {
                text.0 = "RACING...".to_string();
                *color = TextColor(palette::STONE);
            }
            RacePhase::Finished => {
                text.0 = "RACE AGAIN".to_string();
                *color = TextColor(palette::VANILLA);
            }
        }
    }
}

/// Manages the large centered countdown text (3, 2, 1, GO!).
pub fn manage_countdown_text(
    mut commands: Commands,
    phase: Res<RacePhase>,
    timer: Option<Res<CountdownTimer>>,
    wrapper_query: Query<Entity, With<CountdownText>>,
    mut inner_query: Query<&mut Text, With<CountdownTextContent>>,
) {
    match *phase {
        RacePhase::Countdown => {
            let display = timer
                .map(|t| {
                    let secs = t.remaining.ceil() as u32;
                    if secs > 0 {
                        format!("{}", secs)
                    } else {
                        "GO!".to_string()
                    }
                })
                .unwrap_or_default();

            if let Ok(mut text) = inner_query.single_mut() {
                text.0 = display;
            } else if wrapper_query.is_empty() {
                commands
                    .spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            position_type: PositionType::Absolute,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        CountdownText,
                        DespawnOnExit(AppState::Race),
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            Text::new(display),
                            TextFont {
                                font_size: 120.0,
                                ..default()
                            },
                            TextColor(palette::VANILLA),
                            CountdownTextContent,
                        ));
                    });
            }
        }
        _ => {
            for entity in &wrapper_query {
                commands.entity(entity).despawn();
            }
        }
    }
}

/// Manages the race clock display (top-center, M:SS.ss format).
pub fn update_race_clock_display(
    mut commands: Commands,
    phase: Res<RacePhase>,
    clock: Option<Res<RaceClock>>,
    wrapper_query: Query<Entity, With<RaceClockText>>,
    mut inner_query: Query<&mut Text, With<RaceClockTextContent>>,
) {
    let show_clock = matches!(*phase, RacePhase::Racing | RacePhase::Finished);

    if show_clock {
        let elapsed = clock.map(|c| c.elapsed).unwrap_or(0.0);
        let mins = (elapsed / 60.0) as u32;
        let secs = elapsed % 60.0;
        let display = format!("{:01}:{:05.2}", mins, secs);

        if let Ok(mut text) = inner_query.single_mut() {
            text.0 = display;
        } else if wrapper_query.is_empty() {
            commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(20.0),
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    RaceClockText,
                    DespawnOnExit(AppState::Race),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(display),
                        TextFont {
                            font_size: 36.0,
                            ..default()
                        },
                        TextColor(palette::VANILLA),
                        RaceClockTextContent,
                    ));
                });
        }
    } else {
        for entity in &wrapper_query {
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Component)]
pub(crate) struct NoGatesBanner;

#[derive(Component)]
pub(crate) struct OpenEditorButton;

/// Spawns a warning banner and "OPEN EDITOR" button the first frame
/// `NoGatesCourse` exists, and hides the start race button.
pub fn show_no_gates_banner(
    mut commands: Commands,
    no_gates: Option<Res<NoGatesCourse>>,
    existing: Query<(), With<NoGatesBanner>>,
    mut start_btn: Query<&mut Node, With<StartRaceButton>>,
) {
    if no_gates.is_none() || !existing.is_empty() {
        return;
    }

    // Hide the start race button
    for mut node in &mut start_btn {
        node.display = Display::None;
    }

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
            NoGatesBanner,
            DespawnOnExit(AppState::Race),
        ))
        .with_children(|parent| {
            // Warning text
            parent
                .spawn((
                    Node {
                        padding: UiRect::axes(Val::Px(24.0), Val::Px(12.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(palette::SMOKY_BLACK),
                    BorderColor::all(palette::GOLDENROD),
                ))
                .with_children(|banner| {
                    banner.spawn((
                        Text::new("No gates on this course — add gates in the editor to race"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(palette::LIMON),
                    ));
                });

            // "OPEN EDITOR" button
            parent
                .spawn((
                    Button,
                    OpenEditorButton,
                    Node {
                        width: Val::Px(220.0),
                        height: Val::Px(60.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(NORMAL_BUTTON),
                    BorderColor::all(palette::STEEL),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("OPEN EDITOR"),
                        TextFont {
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(palette::VANILLA),
                    ));
                });
        });
}

pub fn handle_open_editor_button(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<OpenEditorButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
    selected: Option<Res<SelectedCourse>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            if let Some(ref sel) = selected {
                commands.insert_resource(PendingEditorCourse {
                    path: sel.path.clone(),
                });
            }
            next_state.set(AppState::Editor);
        }
    }
}

pub fn update_open_editor_button_visuals(
    mut query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<OpenEditorButton>),
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

// ── Leaderboard ─────────────────────────────────────────────────────────────

const LB_BG: Color = Color::srgba(0.02, 0.055, 0.102, 0.80);
const LB_FONT: f32 = 13.0;
const LB_WIDTH: f32 = 190.0;
const LB_ROW_HEIGHT: f32 = 20.0;
const LB_COLOR_BAR_W: f32 = 4.0;

#[derive(Component)]
pub(crate) struct LeaderboardRoot;

#[derive(Component)]
pub(crate) struct LbColorBar(usize);

#[derive(Component)]
pub(crate) struct LbNameText(usize);

#[derive(Component)]
pub(crate) struct LbTimeText(usize);

pub fn setup_leaderboard(mut commands: Commands) {
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
            Visibility::Hidden,
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
                        // Colored accent bar
                        row.spawn((
                            LbColorBar(i),
                            Node {
                                width: Val::Px(LB_COLOR_BAR_W),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(DRONE_COLORS[i]),
                        ));
                        // Position + name
                        row.spawn((
                            LbNameText(i),
                            Text::new(format!("{:>2}  {}", i + 1, DRONE_NAMES[i])),
                            TextFont {
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
    mut root_vis: Query<&mut Visibility, With<LeaderboardRoot>>,
    mut color_bars: Query<(&LbColorBar, &mut BackgroundColor)>,
    mut name_texts: Query<
        (&LbNameText, &mut Text, &mut TextColor),
        Without<LbTimeText>,
    >,
    mut time_texts: Query<
        (&LbTimeText, &mut Text, &mut TextColor),
        Without<LbNameText>,
    >,
) {
    let show = matches!(*phase, RacePhase::Racing | RacePhase::Finished);
    for mut vis in &mut root_vis {
        *vis = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    if !show {
        return;
    }

    let Some(progress) = progress else { return };
    let elapsed = clock.map(|c| c.elapsed).unwrap_or(0.0);
    let standings = progress.standings();

    // Build position→data lookup (avoids O(n²) nested query iteration)
    let mut row_data = [(0usize, false, false, false, 0.0f32); 12];
    for (pos, &(drone_idx, state)) in standings.iter().enumerate() {
        if pos >= 12 { break; }
        row_data[pos] = (drone_idx, true, state.finished, state.crashed, state.finish_time.unwrap_or(0.0));
    }

    for (bar, mut bg) in &mut color_bars {
        let pos = bar.0;
        if pos < 12 && row_data[pos].1 {
            *bg = BackgroundColor(DRONE_COLORS[row_data[pos].0]);
        }
    }
    for (nt, mut text, mut tc) in &mut name_texts {
        let pos = nt.0;
        if pos < 12 && row_data[pos].1 {
            let (drone_idx, _, _, crashed, _) = row_data[pos];
            text.0 = format!("{:>2}  {}", pos + 1, DRONE_NAMES[drone_idx]);
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
                text.0 = fmt_time(finish_time);
                *tc = TextColor(palette::SEA_FOAM);
            } else if crashed {
                text.0 = "DNF".into();
                *tc = TextColor(palette::NEON_RED);
            } else {
                text.0 = fmt_time(elapsed);
                *tc = TextColor(palette::SIDEWALK);
            }
        }
    }
}

fn fmt_time(t: f32) -> String {
    let mins = (t / 60.0) as u32;
    let secs = t % 60.0;
    format!("{:01}:{:05.2}", mins, secs)
}

// ── Camera HUD ──────────────────────────────────────────────────────────────

#[derive(Component)]
pub(crate) struct CameraHudRoot;

#[derive(Component)]
pub(crate) struct CameraHudModeText;

#[derive(Component)]
pub(crate) struct CameraHudHintText;

pub fn setup_camera_hud(mut commands: Commands) {
    commands
        .spawn((
            CameraHudRoot,
            DespawnOnExit(AppState::Race),
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                left: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                ..default()
            },
            GlobalZIndex(80),
        ))
        .with_children(|panel| {
            panel.spawn((
                CameraHudModeText,
                Text::new("CHASE CAM"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(palette::SKY),
            ));
            panel.spawn((
                CameraHudHintText,
                Text::new("[1] Chase  [2] Spectator  [3] FPV"),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(palette::STONE),
            ));
        });
}

pub fn update_camera_hud(
    camera_state: Res<CameraState>,
    progress: Option<Res<RaceProgress>>,
    course_cameras: Option<Res<CourseCameras>>,
    mut mode_text: Query<&mut Text, (With<CameraHudModeText>, Without<CameraHudHintText>)>,
    mut hint_text: Query<&mut Text, (With<CameraHudHintText>, Without<CameraHudModeText>)>,
) {
    if !camera_state.is_changed()
        && (camera_state.mode != CameraMode::Fpv
            || progress.as_ref().is_none_or(|p| !p.is_changed()))
    {
        return;
    }

    let cam_count = course_cameras
        .as_ref()
        .map(|cc| cc.cameras.len())
        .unwrap_or(0);

    let mode_label = match camera_state.mode {
        CameraMode::Chase => "CHASE CAM".to_string(),
        CameraMode::Spectator => "SPECTATOR".to_string(),
        CameraMode::Fpv => {
            let drone_name = progress
                .as_ref()
                .and_then(|p| {
                    let standings = p.standings();
                    let idx = camera_state
                        .target_standings_index
                        .min(standings.len().saturating_sub(1));
                    standings
                        .get(idx)
                        .map(|&(drone_idx, _)| {
                            DRONE_NAMES.get(drone_idx).copied().unwrap_or("???")
                        })
                })
                .unwrap_or("---");
            format!("FPV: {drone_name}")
        }
        CameraMode::CourseCamera(idx) => {
            let label = course_cameras
                .as_ref()
                .and_then(|cc| cc.cameras.get(idx))
                .and_then(|entry| entry.label.as_deref());
            if let Some(name) = label {
                format!("CAM {}: {name}", idx + 1)
            } else {
                format!("CAM {}", idx + 1)
            }
        }
    };

    for mut text in &mut mode_text {
        text.0 = mode_label.clone();
    }

    let hint = if cam_count > 0 {
        let cam_keys = if cam_count == 1 {
            "[1] Cam".to_string()
        } else {
            format!("[1-{}] Cams", cam_count.min(9))
        };
        match camera_state.mode {
            CameraMode::Fpv => format!("{cam_keys}  [2] Chase  [Shift+F] Next  [Shift+S] Spec"),
            _ => format!("{cam_keys}  [2] Chase  [Shift+F] FPV  [Shift+S] Spec"),
        }
    } else {
        match camera_state.mode {
            CameraMode::Fpv => {
                "[1] Chase  [2] Chase  [Shift+F] Next  [Shift+S] Spec".to_string()
            }
            _ => "[1] Chase  [Shift+F] FPV  [Shift+S] Spectator".to_string(),
        }
    };
    for mut text in &mut hint_text {
        text.0 = hint.clone();
    }
}
