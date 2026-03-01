use bevy::prelude::*;

use crate::palette;

// Standard button background colors (used across all UI modules).
pub const BUTTON_NORMAL: Color = palette::INDIGO;
pub const BUTTON_HOVERED: Color = palette::SAPPHIRE;
pub const BUTTON_PRESSED: Color = palette::GREEN;
pub const BUTTON_DISABLED: Color = palette::SMOKY_BLACK;
pub const BUTTON_SELECTED: Color = palette::TEAL;

// Standard button border colors.
pub const BORDER_NORMAL: Color = palette::STEEL;
pub const BORDER_HOVERED: Color = palette::SIDEWALK;
pub const BORDER_PRESSED: Color = palette::VANILLA;
pub const BORDER_DISABLED: Color = palette::INDIGO;

// Panel background.
pub const PANEL_BG: Color = palette::SMOKY_BLACK;

// Toggle colors.
pub const TOGGLE_ON: Color = palette::FROG;
pub const TOGGLE_OFF: Color = palette::BURGUNDY;

/// Apply standard button visuals (background + border) based on interaction state.
pub fn apply_button_visual(
    interaction: &Interaction,
    bg: &mut BackgroundColor,
    border: &mut BorderColor,
) {
    match *interaction {
        Interaction::Pressed => {
            *bg = BackgroundColor(BUTTON_PRESSED);
            *border = BorderColor::all(BORDER_PRESSED);
        }
        Interaction::Hovered => {
            *bg = BackgroundColor(BUTTON_HOVERED);
            *border = BorderColor::all(BORDER_HOVERED);
        }
        Interaction::None => {
            *bg = BackgroundColor(BUTTON_NORMAL);
            *border = BorderColor::all(BORDER_NORMAL);
        }
    }
}

/// Apply standard button background only (no border change).
pub fn apply_button_bg(interaction: &Interaction, bg: &mut BackgroundColor) {
    *bg = BackgroundColor(match *interaction {
        Interaction::Pressed => BUTTON_PRESSED,
        Interaction::Hovered => BUTTON_HOVERED,
        Interaction::None => BUTTON_NORMAL,
    });
}

/// Spawn a standard full-width button used in editor panels.
pub fn spawn_panel_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    marker: impl Component,
) {
    parent
        .spawn((
            Button,
            marker,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(30.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(BUTTON_NORMAL),
            BorderColor::all(BORDER_NORMAL),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(palette::SAND),
            ));
        });
}

/// Spawn a prominent action button with a custom background color.
pub fn spawn_action_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    marker: impl Component,
    bg: Color,
) {
    parent
        .spawn((
            Button,
            marker,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(36.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                margin: UiRect::vertical(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(bg),
            BorderColor::all(BORDER_NORMAL),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));
        });
}

/// Spawn a large button used in menu/results screens.
pub fn spawn_menu_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    marker: impl Component,
) {
    parent
        .spawn((
            Button,
            marker,
            Node {
                width: Val::Px(200.0),
                height: Val::Px(60.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(3.0)),
                ..default()
            },
            BackgroundColor(BUTTON_NORMAL),
            BorderColor::all(BORDER_NORMAL),
        ))
        .with_children(|btn: &mut ChildSpawnerCommands| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));
        });
}

/// Spawn a horizontal divider line.
pub fn spawn_divider(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(1.0),
            margin: UiRect::vertical(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(BORDER_NORMAL),
    ));
}
