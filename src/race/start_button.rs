use bevy::prelude::*;

use crate::palette;
use crate::states::AppState;
use crate::ui_theme;

use super::lifecycle::RacePhase;

#[derive(Component)]
pub(crate) struct StartRaceButton;

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
                    ));
                });
        });
}

pub fn handle_start_race_button(
    mut commands: Commands,
    query: Query<(Entity, &Interaction, &ChildOf), (Changed<Interaction>, With<StartRaceButton>)>,
    mut phase: ResMut<RacePhase>,
) {
    for (_entity, interaction, child_of) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if *phase == RacePhase::WaitingToStart {
            *phase = RacePhase::Converging;
            // Despawn the entire button container (parent node + button + text)
            commands.entity(child_of.parent()).despawn();
            info!("Drones converging to start positions!");
        }
    }
}

pub fn update_start_button_visuals(
    mut button_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<StartRaceButton>),
    >,
) {
    for (interaction, mut bg, mut border) in &mut button_query {
        ui_theme::apply_button_visual(interaction, &mut bg, &mut border);
    }
}
