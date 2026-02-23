pub mod gate;
pub mod lifecycle;
pub mod progress;
pub mod timing;
pub mod ui;

use bevy::prelude::*;

use crate::states::AppState;

pub struct RacePlugin;

impl Plugin for RacePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Race), (setup_race, ui::setup_race_ui))
            .add_systems(
                Update,
                (
                    ui::handle_start_race_button,
                    ui::update_start_race_button_visuals,
                    lifecycle::check_race_finished,
                )
                    .run_if(in_state(AppState::Race)),
            )
            .add_systems(OnExit(AppState::Race), cleanup_race);
    }
}

fn setup_race(mut commands: Commands) {
    commands.init_resource::<lifecycle::RacePhase>();
}

fn cleanup_race(mut commands: Commands) {
    commands.remove_resource::<lifecycle::RacePhase>();
}
