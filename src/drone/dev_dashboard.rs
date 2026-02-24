use bevy::prelude::*;

use super::components::*;
use crate::states::AppState;

const PANEL_BG: Color = Color::srgba(0.06, 0.06, 0.06, 0.92);
const ROW_BG: Color = Color::srgba(0.1, 0.1, 0.1, 0.8);
const ROW_BG_ALT: Color = Color::srgba(0.13, 0.13, 0.13, 0.8);
const BTN_NORMAL: Color = Color::srgb(0.2, 0.2, 0.2);
const BTN_HOVERED: Color = Color::srgb(0.35, 0.35, 0.35);
const BTN_PRESSED: Color = Color::srgb(0.3, 0.7, 0.3);
const RESET_NORMAL: Color = Color::srgb(0.5, 0.15, 0.15);
const RESET_HOVERED: Color = Color::srgb(0.65, 0.2, 0.2);
const RESET_PRESSED: Color = Color::srgb(0.8, 0.3, 0.3);
const TITLE_COLOR: Color = Color::srgb(0.9, 0.75, 0.2);
const LABEL_COLOR: Color = Color::srgb(0.7, 0.7, 0.7);
const VALUE_COLOR: Color = Color::srgb(0.95, 0.95, 0.95);
const MODIFIED_COLOR: Color = Color::srgb(0.3, 0.95, 0.5);

/// Marker on the dashboard root entity.
#[derive(Component)]
pub(crate) struct DevDashboardRoot;

/// Marker on a value text entity, with the parameter index.
#[derive(Component)]
pub(crate) struct ParamValueText(usize);

/// Marker on a +/- button. `delta` is the signed step.
#[derive(Component)]
pub(crate) struct ParamButton {
    index: usize,
    delta: f32,
}

/// Marker on the Reset All button.
#[derive(Component)]
pub(crate) struct ResetAllButton;

/// Toggle the dev dashboard panel with F4.
pub fn toggle_dev_dashboard(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    existing: Query<Entity, With<DevDashboardRoot>>,
    tuning: Res<AiTuningParams>,
) {
    if !keyboard.just_pressed(KeyCode::F4) {
        return;
    }

    if let Ok(entity) = existing.single() {
        commands.entity(entity).despawn();
        info!("Dev dashboard: OFF");
        return;
    }

    info!("Dev dashboard: ON");
    spawn_dashboard(&mut commands, &tuning);
}

fn spawn_dashboard(commands: &mut Commands, tuning: &AiTuningParams) {
    let defaults = AiTuningParams::default();

    commands
        .spawn((
            DevDashboardRoot,
            DespawnOnExit(AppState::Race),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                top: Val::Px(10.0),
                width: Val::Px(310.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            GlobalZIndex(100),
        ))
        .with_children(|panel| {
            // Title row
            panel.spawn((
                Text::new("AI TUNING (F4)"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(TITLE_COLOR),
                Node {
                    margin: UiRect::bottom(Val::Px(4.0)),
                    ..default()
                },
            ));

            // Parameter rows
            for (i, meta) in PARAM_META.iter().enumerate() {
                let bg = if i % 2 == 0 { ROW_BG } else { ROW_BG_ALT };
                let value = tuning.get(i);
                let is_modified = (value - defaults.get(i)).abs() > 0.001;

                panel
                    .spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(28.0),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            padding: UiRect::horizontal(Val::Px(6.0)),
                            column_gap: Val::Px(4.0),
                            ..default()
                        },
                        BackgroundColor(bg),
                    ))
                    .with_children(|row| {
                        // Label
                        row.spawn((
                            Text::new(meta.name),
                            TextFont {
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(LABEL_COLOR),
                            Node {
                                width: Val::Px(130.0),
                                ..default()
                            },
                        ));

                        // Value
                        let val_color = if is_modified {
                            MODIFIED_COLOR
                        } else {
                            VALUE_COLOR
                        };
                        row.spawn((
                            Text::new(format_value(value, meta.step)),
                            TextFont {
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(val_color),
                            ParamValueText(i),
                            Node {
                                width: Val::Px(70.0),
                                justify_content: JustifyContent::FlexEnd,
                                ..default()
                            },
                        ));

                        // Minus button
                        spawn_param_button(row, i, -meta.step, "-");

                        // Plus button
                        spawn_param_button(row, i, meta.step, "+");
                    });
            }

            // Reset All button
            panel
                .spawn((
                    Button,
                    ResetAllButton,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(30.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        margin: UiRect::top(Val::Px(6.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(RESET_NORMAL),
                    BorderColor::all(Color::srgb(0.6, 0.2, 0.2)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("RESET ALL"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.95, 0.85, 0.85)),
                    ));
                });
        });
}

fn spawn_param_button(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    delta: f32,
    label: &str,
) {
    parent
        .spawn((
            Button,
            ParamButton { index, delta },
            Node {
                width: Val::Px(28.0),
                height: Val::Px(22.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(BTN_NORMAL),
            BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
            ));
        });
}

/// Handle +/- button presses to adjust parameter values.
pub fn handle_param_buttons(
    mut tuning: ResMut<AiTuningParams>,
    query: Query<(&Interaction, &ParamButton), Changed<Interaction>>,
) {
    for (interaction, param) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let current = tuning.get(param.index);
        tuning.set(param.index, current + param.delta);
    }
}

/// Handle Reset All button press.
pub fn handle_reset_button(
    mut tuning: ResMut<AiTuningParams>,
    query: Query<&Interaction, (Changed<Interaction>, With<ResetAllButton>)>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            *tuning = AiTuningParams::default();
        }
    }
}

/// Update displayed values to match current resource state.
pub fn update_param_labels(
    tuning: Res<AiTuningParams>,
    mut query: Query<(&ParamValueText, &mut Text, &mut TextColor)>,
) {
    if !tuning.is_changed() {
        return;
    }

    let defaults = AiTuningParams::default();
    for (param, mut text, mut color) in &mut query {
        let value = tuning.get(param.0);
        let meta = &PARAM_META[param.0];
        text.0 = format_value(value, meta.step);

        let is_modified = (value - defaults.get(param.0)).abs() > 0.001;
        *color = TextColor(if is_modified {
            MODIFIED_COLOR
        } else {
            VALUE_COLOR
        });
    }
}

/// Hover/press visual feedback on +/- buttons.
pub fn update_button_colors(
    mut param_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<ParamButton>),
    >,
    mut reset_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<ResetAllButton>, Without<ParamButton>),
    >,
) {
    for (interaction, mut bg) in &mut param_query {
        *bg = BackgroundColor(match *interaction {
            Interaction::Pressed => BTN_PRESSED,
            Interaction::Hovered => BTN_HOVERED,
            Interaction::None => BTN_NORMAL,
        });
    }
    for (interaction, mut bg) in &mut reset_query {
        *bg = BackgroundColor(match *interaction {
            Interaction::Pressed => RESET_PRESSED,
            Interaction::Hovered => RESET_HOVERED,
            Interaction::None => RESET_NORMAL,
        });
    }
}

/// Format a value with appropriate decimal places based on step size.
fn format_value(value: f32, step: f32) -> String {
    if step >= 1.0 {
        format!("{:.1}", value)
    } else if step >= 0.1 {
        format!("{:.2}", value)
    } else {
        format!("{:.3}", value)
    }
}
