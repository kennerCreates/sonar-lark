pub mod ui;

use bevy::prelude::*;

use crate::race::progress::RaceResults;
use crate::states::AppState;

pub struct ResultsPlugin;

impl Plugin for ResultsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Results), ui::setup_results_ui)
            .add_systems(
                Update,
                (
                    ui::handle_replay_button,
                    ui::handle_new_race_button,
                )
                    .run_if(in_state(AppState::Results)),
            )
            .add_systems(OnExit(AppState::Results), cleanup_results);
    }
}

fn cleanup_results(mut commands: Commands) {
    commands.remove_resource::<RaceResults>();
}
