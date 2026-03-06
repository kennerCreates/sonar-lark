pub mod ui;

use bevy::prelude::*;

use crate::states::AppState;

pub struct ResultsPlugin;

impl Plugin for ResultsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Results), ui::setup_results_ui)
            .add_systems(
                Update,
                (
                    ui::update_results_ui,
                    ui::handle_replay_button,
                    ui::handle_new_race_button,
                )
                    .run_if(in_state(AppState::Results)),
            );
    }
}
