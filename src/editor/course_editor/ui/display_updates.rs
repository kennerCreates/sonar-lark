use bevy::prelude::*;

use crate::editor::course_editor::{EditorSelection, EditorTransform, PlacedObstacle};
use crate::palette;
use crate::ui_theme;

use super::types::*;

pub fn update_display_values(
    transform_state: Res<EditorTransform>,
    selection: Res<EditorSelection>,
    mut gate_mode_text: Query<
        &mut Text,
        With<GateOrderModeText>,
    >,
    mut gate_mode_bg: Query<
        &mut BackgroundColor,
        (With<GateOrderModeButton>, Without<PaletteButton>),
    >,
    mut palette_bgs: Query<
        (&PaletteButton, &mut BackgroundColor),
        Without<GateOrderModeButton>,
    >,
) {
    if !transform_state.is_changed() && !selection.is_changed() {
        return;
    }

    if let Ok(mut text) = gate_mode_text.single_mut() {
        **text = if transform_state.gate_order_mode {
            "Gate Mode: ON".to_string()
        } else {
            "Gate Mode: OFF".to_string()
        };
    }

    if let Ok(mut bg) = gate_mode_bg.single_mut() {
        *bg = BackgroundColor(if transform_state.gate_order_mode {
            ui_theme::TOGGLE_ON
        } else {
            ui_theme::TOGGLE_OFF
        });
    }

    for (btn, mut bg) in &mut palette_bgs {
        *bg = BackgroundColor(
            if selection.palette_id.as_ref() == Some(&btn.0) {
                ui_theme::BUTTON_SELECTED
            } else {
                ui_theme::BUTTON_NORMAL
            },
        );
    }
}

pub fn handle_button_hover(
    mut query: Query<
        (&Interaction, &mut BackgroundColor),
        (
            Changed<Interaction>,
            Or<(
                With<BackToMenuButton>,
                With<ClearGateOrdersButton>,
            )>,
        ),
    >,
) {
    for (interaction, mut bg) in &mut query {
        ui_theme::apply_button_bg(interaction, &mut bg);
    }
}

pub fn handle_transform_mode_buttons(
    mut transform_state: ResMut<EditorTransform>,
    query: Query<(&Interaction, &TransformModeButton), Changed<Interaction>>,
) {
    for (interaction, btn) in &query {
        if *interaction == Interaction::Pressed {
            transform_state.mode = btn.0;
        }
    }
}

pub fn update_transform_mode_ui(
    transform_state: Res<EditorTransform>,
    mut buttons: Query<(&TransformModeButton, &mut BackgroundColor)>,
) {
    if !transform_state.is_changed() {
        return;
    }
    for (btn, mut bg) in &mut buttons {
        *bg = BackgroundColor(if btn.0 == transform_state.mode {
            ui_theme::BUTTON_SELECTED
        } else {
            ui_theme::BUTTON_NORMAL
        });
    }
}

pub fn update_gate_count_display(
    placed_query: Query<&PlacedObstacle>,
    mut text_query: Query<(&mut Text, &mut TextColor), With<GateCountText>>,
) {
    let gate_count = placed_query.iter().filter(|p| p.gate_order.is_some()).count();
    if let Ok((mut text, mut color)) = text_query.single_mut() {
        **text = format!("Gates: {gate_count} (loop)");
        *color = if gate_count >= 2 {
            TextColor(palette::SKY)
        } else {
            TextColor(palette::BRONZE)
        };
    }
}
