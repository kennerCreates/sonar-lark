pub mod portrait_config;
pub mod portrait_editor;

use bevy::prelude::*;

use crate::states::AppState;

pub struct DevMenuPlugin;

impl Plugin for DevMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::DevMenu),
            portrait_editor::setup_portrait_editor,
        )
        .add_systems(
            OnExit(AppState::DevMenu),
            portrait_editor::cleanup_portrait_editor,
        )
        .add_systems(
            Update,
            (
                portrait_editor::handle_back_button,
                portrait_editor::handle_part_tabs,
                portrait_editor::handle_variant_selection,
                portrait_editor::handle_primary_color_click,
                portrait_editor::handle_primary_color_veto,
                portrait_editor::handle_secondary_color_click,
                portrait_editor::handle_auto_secondary,
                portrait_editor::handle_pairing_row_click,
                portrait_editor::handle_pairing_picker_click,
                portrait_editor::handle_auto_assign_all,
                portrait_editor::handle_save_button,
                portrait_editor::handle_reset_slot_button,
                portrait_editor::handle_make_unique_button,
            )
                .run_if(in_state(AppState::DevMenu)),
        )
        .add_systems(
            Update,
            (
                portrait_editor::handle_reset_all_button,
                portrait_editor::update_preview,
                portrait_editor::update_tab_visuals,
                portrait_editor::rebuild_variant_panel,
                portrait_editor::rebuild_unique_status_row,
                portrait_editor::rebuild_primary_grid,
                portrait_editor::rebuild_secondary_grid,
                portrait_editor::rebuild_pairing_panel,
                portrait_editor::update_drone_warning,
                portrait_editor::update_color_name_on_hover,
                portrait_editor::handle_button_hover_visuals,
            )
                .run_if(in_state(AppState::DevMenu)),
        );
    }
}
