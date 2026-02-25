use bevy::prelude::*;

use crate::course::loader::SelectedCourse;
use crate::drone::components::*;
use crate::drone::spawning::NoGatesCourse;
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
