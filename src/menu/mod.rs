pub mod ui;

use bevy::prelude::*;

use crate::states::AppState;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Menu), ui::setup_menu)
            .add_systems(Update, ui::handle_menu_input.run_if(in_state(AppState::Menu)));
    }
}
