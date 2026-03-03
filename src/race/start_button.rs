use bevy::prelude::*;

use crate::drone::components::Drone;
use crate::palette;
use crate::states::AppState;
use crate::ui_theme;

use super::lifecycle::{CountdownTimer, RacePhase, ResultsTransitionTimer};
use super::progress::RaceProgress;
use super::timing::RaceClock;

#[derive(Component)]
pub(crate) struct StartRaceButton;

#[derive(Component)]
pub(crate) struct StartRaceButtonText;

#[derive(Component)]
pub(crate) struct CountdownText;

#[derive(Component)]
pub(crate) struct CountdownTextContent;

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
                    BackgroundColor(ui_theme::BUTTON_NORMAL),
                    BorderColor::all(ui_theme::BORDER_NORMAL),
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
                // Sound is played by tick_countdown when the visible 3-2-1 starts
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
                commands.remove_resource::<ResultsTransitionTimer>();
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
                *bg = BackgroundColor(ui_theme::BUTTON_DISABLED);
                *border = BorderColor::all(ui_theme::BORDER_DISABLED);
            }
            RacePhase::WaitingToStart | RacePhase::Finished => {
                ui_theme::apply_button_visual(interaction, &mut bg, &mut border);
            }
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
