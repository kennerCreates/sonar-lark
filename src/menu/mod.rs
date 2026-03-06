pub mod ui;

use bevy::prelude::*;

use crate::states::AppState;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Menu), ui::setup_menu)
            .add_systems(OnExit(AppState::Menu), ui::cleanup_menu)
            .add_systems(
                Update,
                (
                    ui::handle_start_game_button,
                    ui::handle_dev_mode_button,
                    ui::handle_location_card,
                    ui::handle_edit_poster_button,
                    ui::handle_course_library_button,
                    ui::handle_course_list_item,
                    ui::handle_course_delete_item,
                    ui::handle_course_library_back,
                )
                    .run_if(in_state(AppState::Menu)),
            );
    }
}
