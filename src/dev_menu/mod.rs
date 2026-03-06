pub mod color_picker_data;
pub mod pilot_generator;
pub mod pilot_roster_viewer;
pub mod portrait_config;
pub mod portrait_editor;

use bevy::prelude::*;

use crate::editor::workshop;
use crate::states::DevMenuPage;

pub struct DevMenuPlugin;

impl Plugin for DevMenuPlugin {
    fn build(&self, app: &mut App) {
        // ── Portrait Palette Editor (PaletteEditor page) ───────────────
        app.add_systems(
            OnEnter(DevMenuPage::PaletteEditor),
            portrait_editor::setup_portrait_editor,
        )
        .add_systems(
            OnExit(DevMenuPage::PaletteEditor),
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
                portrait_editor::handle_secondary_pairing_click,
                portrait_editor::handle_pairing_picker_click,
                portrait_editor::dismiss_pairing_picker,
                portrait_editor::handle_auto_assign_all,
                portrait_editor::handle_save_button,
                portrait_editor::handle_reset_slot_button,
            )
                .run_if(in_state(DevMenuPage::PaletteEditor)),
        )
        .add_systems(
            Update,
            (
                portrait_editor::handle_make_unique_button,
                portrait_editor::handle_reset_all_button,
                portrait_editor::update_preview,
                portrait_editor::update_tab_visuals,
                portrait_editor::rebuild_variant_panel,
                portrait_editor::rebuild_unique_status_row,
                portrait_editor::rebuild_primary_grid,
                portrait_editor::rebuild_pairing_picker,
                portrait_editor::update_drone_warning,
                portrait_editor::update_color_name_on_hover,
                portrait_editor::handle_button_hover_visuals,
            )
                .run_if(in_state(DevMenuPage::PaletteEditor)),
        );

        // ── Obstacle Workshop (ObstacleWorkshop page) ─────────────────
        app.add_plugins(workshop::WorkshopPlugin);

        // ── Pilot Roster Viewer (PilotRosterViewer page) ────────────────
        app.add_systems(
            OnEnter(DevMenuPage::PilotRosterViewer),
            pilot_roster_viewer::setup_roster_viewer,
        )
        .add_systems(
            OnExit(DevMenuPage::PilotRosterViewer),
            pilot_roster_viewer::cleanup_roster_viewer,
        )
        .add_systems(
            Update,
            (
                pilot_roster_viewer::handle_back_button,
                pilot_roster_viewer::handle_pilot_generator_button,
                pilot_roster_viewer::handle_palette_editor_button,
                pilot_roster_viewer::handle_obstacle_workshop_button,
                pilot_roster_viewer::handle_delete_button,
                pilot_roster_viewer::rebuild_roster_list,
            )
                .run_if(in_state(DevMenuPage::PilotRosterViewer)),
        );

        // ── Pilot Generator (PilotGenerator page — default) ───────────
        app.add_systems(
            OnEnter(DevMenuPage::PilotGenerator),
            pilot_generator::setup_pilot_generator,
        )
        .add_systems(
            OnExit(DevMenuPage::PilotGenerator),
            pilot_generator::cleanup_pilot_generator,
        )
        .add_systems(
            Update,
            (
                pilot_generator::handle_back_button,
                pilot_generator::handle_palette_editor_button,
                pilot_generator::handle_obstacle_workshop_button,
                pilot_generator::handle_reroll_portrait_button,
                pilot_generator::handle_reroll_gamertag_button,
                pilot_generator::handle_reroll_personality_button,
                pilot_generator::handle_reroll_skill_button,
                pilot_generator::handle_accept_button,
                pilot_generator::handle_roster_viewer_button,
                pilot_generator::update_preview,
                pilot_generator::update_pilot_info,
            )
                .run_if(in_state(DevMenuPage::PilotGenerator)),
        );
    }
}
