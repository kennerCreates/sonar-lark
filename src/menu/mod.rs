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
                    ui::handle_course_selection,
                    ui::update_course_highlights,
                    ui::handle_editor_button,
                    ui::handle_race_button,
                    ui::handle_dev_button,
                    ui::handle_button_visuals,
                )
                    .run_if(in_state(AppState::Menu)),
            );
    }
}
