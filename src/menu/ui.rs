use bevy::prelude::*;

use crate::states::AppState;

pub fn setup_menu(mut _commands: Commands) {
    // TODO: spawn menu UI entities with DespawnOnExit(AppState::Menu)
}

pub fn handle_menu_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard.just_pressed(KeyCode::KeyE) {
        next_state.set(AppState::Editor);
    }
    if keyboard.just_pressed(KeyCode::KeyR) {
        next_state.set(AppState::Race);
    }
}
