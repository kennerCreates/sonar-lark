use bevy::prelude::*;

use crate::course::location::{LocationRegistry, SelectedLocation};
use crate::league::LeagueState;
use crate::palette;
use crate::states::{AppState, PendingEditorCourse};
use crate::ui_theme::{self, UiFont};

#[derive(Component)]
pub(crate) struct StartGameButton;

#[derive(Component)]
pub(crate) struct DevModeButton;

#[derive(Component)]
pub(crate) struct LandingRoot;

#[derive(Component)]
pub(crate) struct LocationSelectRoot;

/// Marker on a clickable location card. Stores the location index.
#[derive(Component)]
pub(crate) struct LocationCard(pub usize);

/// When present on entering Menu, skips the landing page and goes straight to location select.
#[derive(Resource)]
pub struct SkipToLocationSelect;

pub fn setup_menu(
    mut commands: Commands,
    skip: Option<Res<SkipToLocationSelect>>,
    font: Res<UiFont>,
    location_registry: Res<LocationRegistry>,
    league: Option<Res<LeagueState>>,
) {
    if skip.is_some() {
        commands.remove_resource::<SkipToLocationSelect>();
        spawn_location_select(&mut commands, &font.0, &location_registry, league.as_deref());
        return;
    }

    let ui_font = font.0.clone();
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(30.0),
                ..default()
            },
            DespawnOnExit(AppState::Menu),
            LandingRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("SONAR LARK"),
                TextFont { font: ui_font.clone(), font_size: 64.0, ..default() },
                TextColor(palette::VANILLA),
            ));

            parent.spawn((
                Text::new("Drone Racing Simulator"),
                TextFont { font: ui_font.clone(), font_size: 24.0, ..default() },
                TextColor(palette::SIDEWALK),
            ));

            let ui_font2 = ui_font.clone();
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(16.0),
                    margin: UiRect::top(Val::Px(20.0)),
                    ..default()
                })
                .with_children(|col| {
                    ui_theme::spawn_menu_button(col, "Start Game", StartGameButton, 260.0, &ui_font2);
                    ui_theme::spawn_disabled_menu_button(col, "Sandbox", 260.0, &ui_font2);
                    ui_theme::spawn_disabled_menu_button(col, "Settings", 260.0, &ui_font2);
                    ui_theme::spawn_menu_button(col, "Dev Mode", DevModeButton, 260.0, &ui_font2);
                });
        });
}

pub fn handle_start_game_button(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<StartGameButton>)>,
    landing_query: Query<Entity, With<LandingRoot>>,
    font: Res<UiFont>,
    location_registry: Res<LocationRegistry>,
    league: Option<Res<LeagueState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            for entity in &landing_query {
                commands.entity(entity).despawn();
            }
            spawn_location_select(&mut commands, &font.0, &location_registry, league.as_deref());
        }
    }
}

pub fn handle_dev_mode_button(
    query: Query<&Interaction, (Changed<Interaction>, With<DevModeButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::DevMenu);
        }
    }
}

fn spawn_location_select(
    commands: &mut Commands,
    font: &Handle<Font>,
    registry: &LocationRegistry,
    league: Option<&LeagueState>,
) {
    let ui_font = font.clone();
    let money = league.map_or(0.0, |l| l.money);

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(30.0),
                ..default()
            },
            DespawnOnExit(AppState::Menu),
            LocationSelectRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Choose your Location..."),
                TextFont { font: ui_font.clone(), font_size: 48.0, ..default() },
                TextColor(palette::VANILLA),
            ));

            // Money display
            parent.spawn((
                Text::new(format!("Budget: ${:.0}", money)),
                TextFont { font: ui_font.clone(), font_size: 20.0, ..default() },
                TextColor(palette::SIDEWALK),
            ));

            let ui_font2 = ui_font.clone();
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(40.0),
                    margin: UiRect::vertical(Val::Px(20.0)),
                    ..default()
                })
                .with_children(|row| {
                    let races_completed = league.map_or(0, |l| l.races_completed);
                    for (i, loc) in registry.locations.iter().enumerate() {
                        let locked = loc.name == "Golf Course" && races_completed == 0;
                        let can_afford = !locked && money >= loc.rental_fee;
                        let label_override = if locked { Some("LOCKED") } else { None };
                        spawn_location_card(row, i, &loc.name, loc.rental_fee, can_afford, label_override, &ui_font2);
                    }
                });
        });
}

fn spawn_location_card(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    name: &str,
    rental_fee: f32,
    can_afford: bool,
    label_override: Option<&str>,
    font: &Handle<Font>,
) {
    let ui_font = font.clone();
    let fee_label = if let Some(label) = label_override {
        label.to_string()
    } else if rental_fee == 0.0 {
        "FREE".to_string()
    } else {
        format!("${:.0}", rental_fee)
    };

    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|col| {
            col.spawn((
                Text::new(fee_label),
                TextFont { font: ui_font.clone(), font_size: 20.0, ..default() },
                TextColor(if can_afford { palette::VANILLA } else { palette::CHAINMAIL }),
            ));

            if can_afford {
                let ui_font2 = ui_font.clone();
                let display_name = name.replace(' ', "\n");
                col.spawn((
                    Button,
                    ui_theme::ThemedButton,
                    LocationCard(index),
                    Node {
                        width: Val::Px(220.0),
                        height: Val::Px(140.0),
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
                        Text::new(display_name),
                        TextFont { font: ui_font2.clone(), font_size: 22.0, ..default() },
                        TextColor(palette::VANILLA),
                        TextLayout::new_with_justify(Justify::Center),
                    ));
                });
            } else {
                let ui_font2 = ui_font.clone();
                let display_name = name.replace(' ', "\n");
                col.spawn((
                    Node {
                        width: Val::Px(220.0),
                        height: Val::Px(140.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(ui_theme::BUTTON_DISABLED),
                    BorderColor::all(ui_theme::BORDER_DISABLED),
                ))
                .with_children(|card| {
                    card.spawn((
                        Text::new(display_name),
                        TextFont { font: ui_font2.clone(), font_size: 22.0, ..default() },
                        TextColor(palette::CHAINMAIL),
                        TextLayout::new_with_justify(Justify::Center),
                    ));
                });
            }
        });
}

pub fn handle_location_card(
    mut commands: Commands,
    query: Query<(&Interaction, &LocationCard), Changed<Interaction>>,
    mut next_state: ResMut<NextState<AppState>>,
    location_registry: Res<LocationRegistry>,
    mut league: Option<ResMut<LeagueState>>,
) {
    for (interaction, card) in &query {
        if *interaction == Interaction::Pressed {
            let Some(location) = location_registry.locations.get(card.0) else {
                continue;
            };

            // Deduct rental fee
            if let Some(ref mut league) = league {
                league.money -= location.rental_fee;
            }

            // Insert location selection
            commands.insert_resource(SelectedLocation(card.0));

            // Load existing location save if present
            let save_path = location.save_path();
            if std::path::Path::new(&save_path).exists() {
                commands.insert_resource(PendingEditorCourse {
                    path: save_path,
                });
            }

            next_state.set(AppState::Editor);
        }
    }
}

pub fn cleanup_menu(mut _commands: Commands) {}
