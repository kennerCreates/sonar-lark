use bevy::prelude::*;

use crate::course::loader::SelectedCourse;
use crate::drone::spawning::NoGatesCourse;
use crate::palette;
use crate::states::{AppState, PendingEditorCourse};
use crate::ui_theme;

use super::lifecycle::{CountdownTimer, RacePhase};
use super::start_button::{CountdownText, CountdownTextContent, StartRaceButton};
use super::timing::RaceClock;

#[derive(Component)]
pub(crate) struct RaceClockText;

#[derive(Component)]
pub(crate) struct RaceClockTextContent;

#[derive(Component)]
pub(crate) struct NoGatesBanner;

#[derive(Component)]
pub(crate) struct OpenEditorButton;

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
                    if secs > 3 {
                        // Convergence phase — no visible countdown yet
                        String::new()
                    } else if secs > 0 {
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
        let display = ui_theme::fmt_time(elapsed);

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
            ui_theme::spawn_menu_button(parent, "OPEN EDITOR", OpenEditorButton, 220.0);
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

