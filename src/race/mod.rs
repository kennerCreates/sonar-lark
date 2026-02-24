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
            // Race logic chain: ordering matters for correctness
            .add_systems(
                Update,
                (
                    lifecycle::tick_countdown,
                    timing::tick_race_clock,
                    gate::gate_trigger_check,
                    gate::miss_detection,
                    lifecycle::check_race_finished,
                )
                    .chain()
                    .run_if(in_state(AppState::Race)),
            )
            // UI systems: independent, no ordering needed
            .add_systems(
                Update,
                (
                    ui::handle_start_race_button,
                    ui::update_start_button_visuals,
                    ui::update_start_button_text,
                    ui::show_no_gates_banner,
                    ui::handle_open_editor_button,
                    ui::update_open_editor_button_visuals,
                    ui::manage_countdown_text,
                    ui::update_race_clock_display,
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
    commands.remove_resource::<progress::RaceProgress>();
    commands.remove_resource::<timing::RaceClock>();
    commands.remove_resource::<lifecycle::CountdownTimer>();
}
