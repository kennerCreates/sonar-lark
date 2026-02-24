use bevy::prelude::*;

use crate::course::loader::SelectedCourse;
use crate::drone::components::*;
use crate::drone::spawning::NoGatesCourse;
use crate::editor::course_editor::PendingEditorCourse;
use crate::states::AppState;
use super::lifecycle::RacePhase;

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);
const DISABLED_BUTTON: Color = Color::srgb(0.1, 0.1, 0.1);

#[derive(Component)]
pub(crate) struct StartRaceButton;

#[derive(Component)]
pub(crate) struct StartRaceButtonText;

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
                    BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("START RACE"),
                        TextFont {
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        StartRaceButtonText,
                    ));
                });
        });
}

pub fn handle_start_race_button(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<StartRaceButton>)>,
    mut phase: ResMut<RacePhase>,
    mut drones: Query<(Entity, &mut DronePhase), With<Drone>>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match *phase {
            RacePhase::WaitingToStart => {
                *phase = RacePhase::Racing;
                for (_, mut drone_phase) in &mut drones {
                    *drone_phase = DronePhase::Racing;
                }
                info!("Race started!");
            }
            RacePhase::Racing => {}
            RacePhase::Finished => {
                // Despawn all drones so spawn_drones re-runs next frame
                // with a fresh RaceSeed and new randomized configs/splines.
                for (entity, ..) in &drones {
                    commands.entity(entity).despawn();
                }
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
            RacePhase::Racing => {
                *bg = BackgroundColor(DISABLED_BUTTON);
                *border = BorderColor::all(Color::srgb(0.2, 0.2, 0.2));
            }
            RacePhase::WaitingToStart | RacePhase::Finished => match *interaction {
                Interaction::Pressed => {
                    *bg = BackgroundColor(PRESSED_BUTTON);
                    *border = BorderColor::all(Color::WHITE);
                }
                Interaction::Hovered => {
                    *bg = BackgroundColor(HOVERED_BUTTON);
                    *border = BorderColor::all(Color::srgb(0.6, 0.6, 0.6));
                }
                Interaction::None => {
                    *bg = BackgroundColor(NORMAL_BUTTON);
                    *border = BorderColor::all(Color::srgb(0.3, 0.3, 0.3));
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
                *color = TextColor(Color::srgb(0.9, 0.9, 0.9));
            }
            RacePhase::Racing => {
                text.0 = "RACING...".to_string();
                *color = TextColor(Color::srgb(0.4, 0.4, 0.4));
            }
            RacePhase::Finished => {
                text.0 = "RACE AGAIN".to_string();
                *color = TextColor(Color::srgb(0.9, 0.9, 0.9));
            }
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
                    BackgroundColor(Color::srgba(0.15, 0.1, 0.0, 0.9)),
                    BorderColor::all(Color::srgb(0.9, 0.6, 0.1)),
                ))
                .with_children(|banner| {
                    banner.spawn((
                        Text::new("No gates on this course — add gates in the editor to race"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.8, 0.2)),
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
                    BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("OPEN EDITOR"),
                        TextFont {
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
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
                *border = BorderColor::all(Color::WHITE);
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(HOVERED_BUTTON);
                *border = BorderColor::all(Color::srgb(0.6, 0.6, 0.6));
            }
            Interaction::None => {
                *bg = BackgroundColor(NORMAL_BUTTON);
                *border = BorderColor::all(Color::srgb(0.3, 0.3, 0.3));
            }
        }
    }
}
