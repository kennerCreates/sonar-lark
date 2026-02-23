use bevy::prelude::*;

use crate::drone::components::*;
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
    query: Query<&Interaction, (Changed<Interaction>, With<StartRaceButton>)>,
    mut phase: ResMut<RacePhase>,
    mut drones: Query<(
        &mut AIController,
        &mut DroneDynamics,
        &mut PositionPid,
        &mut DesiredPosition,
        &mut DesiredAttitude,
        &mut Transform,
        &DroneStartPosition,
    )>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match *phase {
            RacePhase::WaitingToStart => {
                *phase = RacePhase::Racing;
                info!("Race started!");
            }
            RacePhase::Racing => {}
            RacePhase::Finished => {
                for (mut ai, mut dynamics, mut pid, mut desired, mut attitude, mut transform, start_pos) in
                    &mut drones
                {
                    ai.current_waypoint = 0;
                    ai.target_gate_index = 0;

                    dynamics.velocity = Vec3::ZERO;
                    dynamics.angular_velocity = Vec3::ZERO;
                    dynamics.thrust = 0.0;
                    dynamics.commanded_thrust = 0.0;

                    pid.integral = Vec3::ZERO;
                    pid.prev_error = Vec3::ZERO;

                    transform.translation = start_pos.translation;
                    transform.rotation = start_pos.rotation;

                    desired.position = start_pos.translation;
                    desired.velocity_hint = Vec3::ZERO;

                    attitude.orientation = start_pos.rotation;
                    attitude.thrust_magnitude = 9.81 * dynamics.mass;
                }
                *phase = RacePhase::WaitingToStart;
                info!("Race reset!");
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
