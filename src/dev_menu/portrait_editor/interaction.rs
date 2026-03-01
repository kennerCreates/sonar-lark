use bevy::input::mouse::MouseButton;
use bevy::prelude::*;

use crate::pilot::portrait::{
    ALL_ACCESSORIES, ALL_EYE_STYLES, ALL_FACE_SHAPES, ALL_HAIR_STYLES, ALL_MOUTH_STYLES,
    ALL_SHIRT_STYLES,
};
use crate::states::DevMenuPage;

use crate::dev_menu::portrait_config::{
    DRONE_COLOR_INDEX, MIN_DRONE_COLORS, PortraitColorSlot,
    PortraitPaletteConfig, save_config,
};
use super::{
    AutoAssignAllButton, BackButton, DroneColorPickerCell, EditorTab,
    MakeUniqueButton, PairingPickerCell, PartTab, PortraitEditorState,
    PrimaryColorCell, ResetAllButton,
    ResetSlotButton, SaveButton, SecondaryPairingCell,
    VariantButton,
};

pub fn handle_back_button(
    query: Query<&Interaction, (Changed<Interaction>, With<BackButton>)>,
    mut next_state: ResMut<NextState<DevMenuPage>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(DevMenuPage::PilotGenerator);
        }
    }
}

pub fn handle_part_tabs(
    query: Query<(&Interaction, &PartTab), Changed<Interaction>>,
    mut state: ResMut<PortraitEditorState>,
) {
    for (interaction, tab) in &query {
        if *interaction == Interaction::Pressed && state.active_tab != tab.0 {
            state.active_tab = tab.0;
            state.selected_pairing_primary = None;
            state.preview_dirty = true;
        }
    }
}

pub fn handle_variant_selection(
    query: Query<(&Interaction, &VariantButton), Changed<Interaction>>,
    mut state: ResMut<PortraitEditorState>,
) {
    for (interaction, vb) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match vb.tab {
            EditorTab::Face => {
                state.face_shape = ALL_FACE_SHAPES[vb.index];
            }
            EditorTab::Eyes => {
                state.eye_style = ALL_EYE_STYLES[vb.index];
            }
            EditorTab::Mouth => {
                state.mouth_style = ALL_MOUTH_STYLES[vb.index];
            }
            EditorTab::Hair => {
                state.hair_style = ALL_HAIR_STYLES[vb.index];
            }
            EditorTab::Shirt => {
                state.shirt_style = ALL_SHIRT_STYLES[vb.index];
            }
            EditorTab::Accessory => {
                if vb.index >= ALL_ACCESSORIES.len() {
                    state.accessory = None;
                } else {
                    state.accessory = Some(ALL_ACCESSORIES[vb.index]);
                }
            }
            EditorTab::Drone => {}
        }
        state.preview_dirty = true;
    }
}

pub fn handle_primary_color_click(
    query: Query<(&Interaction, &PrimaryColorCell), Changed<Interaction>>,
    mut state: ResMut<PortraitEditorState>,
) {
    for (interaction, cell) in &query {
        if *interaction == Interaction::Pressed
            && let Some(slot) = state.active_tab.color_slot()
        {
            state.primary_colors.insert(slot, cell.0);
            state.preview_dirty = true;
        }
    }
}

pub fn handle_primary_color_veto(
    query: Query<(&Interaction, &PrimaryColorCell)>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut config: ResMut<PortraitPaletteConfig>,
    state: Res<PortraitEditorState>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Some(slot) = state.active_tab.color_slot() else {
        return;
    };
    let vi = state.current_variant_index();
    for (interaction, cell) in &query {
        if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
            // Block vetoing if it would drop drone colors below minimum
            if slot == PortraitColorSlot::Drone
                && !config.is_vetoed(slot, cell.0)
                && config.drone_colors_allowed() <= MIN_DRONE_COLORS
            {
                return;
            }
            config.toggle_veto_for(slot, vi, cell.0);
        }
    }
}

pub fn handle_save_button(
    query: Query<&Interaction, (Changed<Interaction>, With<SaveButton>)>,
    config: Res<PortraitPaletteConfig>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            if config.drone_colors_allowed() < MIN_DRONE_COLORS {
                warn!(
                    "Cannot save: need at least {} drone colors, have {}",
                    MIN_DRONE_COLORS,
                    config.drone_colors_allowed()
                );
                return;
            }
            save_config(&config);
        }
    }
}

pub fn handle_reset_slot_button(
    query: Query<&Interaction, (Changed<Interaction>, With<ResetSlotButton>)>,
    mut config: ResMut<PortraitPaletteConfig>,
    mut state: ResMut<PortraitEditorState>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed
            && let Some(slot) = state.active_tab.color_slot()
        {
            config.reset_slot(slot);
            state.preview_dirty = true;
        }
    }
}

pub fn handle_reset_all_button(
    query: Query<&Interaction, (Changed<Interaction>, With<ResetAllButton>)>,
    mut config: ResMut<PortraitPaletteConfig>,
    mut state: ResMut<PortraitEditorState>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            config.reset_all();
            state.preview_dirty = true;
        }
    }
}

pub fn handle_make_unique_button(
    query: Query<&Interaction, (Changed<Interaction>, With<MakeUniqueButton>)>,
    mut config: ResMut<PortraitPaletteConfig>,
    mut state: ResMut<PortraitEditorState>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed
            && let Some(slot) = state.active_tab.color_slot()
            && let Some(vi) = state.current_variant_index()
        {
            if config.is_variant_unique(slot, vi) {
                config.revert_variant_to_default(slot, vi);
            } else {
                config.make_variant_unique(slot, vi);
            }
            state.preview_dirty = true;
        }
    }
}

// ── Pairing interaction handlers ─────────────────────────────────────────────

/// Clicking a secondary pairing cell opens the picker for that primary color.
pub fn handle_secondary_pairing_click(
    query: Query<(&Interaction, &SecondaryPairingCell), Changed<Interaction>>,
    mut state: ResMut<PortraitEditorState>,
) {
    for (interaction, cell) in &query {
        if *interaction == Interaction::Pressed {
            if state.selected_pairing_primary == Some(cell.0) {
                // Toggle off if clicking the same cell
                state.selected_pairing_primary = None;
            } else {
                state.selected_pairing_primary = Some(cell.0);
            }
        }
    }
}

/// Clicking a color in the picker sets the complementary and closes the picker.
pub fn handle_pairing_picker_click(
    query: Query<(&Interaction, &PairingPickerCell), Changed<Interaction>>,
    drone_query: Query<&Interaction, (Changed<Interaction>, With<DroneColorPickerCell>)>,
    mut state: ResMut<PortraitEditorState>,
    mut config: ResMut<PortraitPaletteConfig>,
) {
    let Some(primary_idx) = state.selected_pairing_primary else {
        return;
    };

    // Handle drone-color rainbow cell
    for interaction in &drone_query {
        if *interaction == Interaction::Pressed
            && let Some(slot) = state.active_tab.color_slot()
        {
            let vi = state.current_variant_index();
            config.set_complementary_for(slot, vi, primary_idx, DRONE_COLOR_INDEX);
            state.selected_pairing_primary = None;
            state.preview_dirty = true;
            return;
        }
    }

    // Handle normal palette cell
    for (interaction, cell) in &query {
        if *interaction == Interaction::Pressed
            && let Some(slot) = state.active_tab.color_slot()
        {
            let vi = state.current_variant_index();
            config.set_complementary_for(slot, vi, primary_idx, cell.0);
            state.selected_pairing_primary = None;
            state.preview_dirty = true;
        }
    }
}

/// Dismiss the pairing picker when clicking outside it.
pub fn dismiss_pairing_picker(
    mouse: Res<ButtonInput<MouseButton>>,
    mut state: ResMut<PortraitEditorState>,
    picker_cells: Query<&Interaction, With<PairingPickerCell>>,
    drone_cell: Query<&Interaction, With<DroneColorPickerCell>>,
    secondary_cells: Query<&Interaction, With<SecondaryPairingCell>>,
) {
    if state.selected_pairing_primary.is_none() {
        return;
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    // Don't dismiss if hovering over picker, drone-color, or secondary cells
    for interaction in &picker_cells {
        if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
            return;
        }
    }
    for interaction in &drone_cell {
        if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
            return;
        }
    }
    for interaction in &secondary_cells {
        if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
            return;
        }
    }
    state.selected_pairing_primary = None;
}

pub fn handle_auto_assign_all(
    query: Query<&Interaction, (Changed<Interaction>, With<AutoAssignAllButton>)>,
    mut config: ResMut<PortraitPaletteConfig>,
    mut state: ResMut<PortraitEditorState>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed
            && state.show_pairing()
            && let Some(slot) = state.active_tab.color_slot()
        {
            let vi = state.current_variant_index();
            config.auto_assign_all_for(slot, vi);
            state.preview_dirty = true;
        }
    }
}
