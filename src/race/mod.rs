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
        app.add_systems(
            OnEnter(AppState::Race),
            (setup_race, ui::setup_race_ui, ui::setup_leaderboard, ui::setup_camera_hud),
        )
            // Build GatePlanes resource once gate entities are spawned
            .add_systems(
                Update,
                gate::build_gate_planes
                    .run_if(in_state(AppState::Race))
                    .run_if(not(resource_exists::<gate::GatePlanes>)),
            )
            // Race logic chain: ordering matters for correctness
            .add_systems(
                Update,
                (
                    lifecycle::tick_countdown,
                    timing::tick_race_clock,
                    gate::gate_trigger_check,
                    gate::miss_detection,
                    progress::sync_spline_progress,
                    lifecycle::check_race_finished,
                )
                    .chain()
                    .run_if(in_state(AppState::Race)),
            )
            // Results transition timer (runs after race logic chain)
            .add_systems(
                Update,
                lifecycle::tick_results_transition.run_if(in_state(AppState::Race)),
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
                    ui::update_leaderboard,
                    ui::update_camera_hud,
                )
                    .run_if(in_state(AppState::Race)),
            )
            .add_systems(OnExit(AppState::Race), cleanup_race)
            // RaceProgress persists into Results for camera drone-finding; clean up on exit
            .add_systems(OnExit(AppState::Results), cleanup_race_progress);
    }
}

fn setup_race(mut commands: Commands) {
    commands.init_resource::<lifecycle::RacePhase>();
}

fn cleanup_race(mut commands: Commands) {
    commands.remove_resource::<lifecycle::RacePhase>();
    commands.remove_resource::<timing::RaceClock>();
    commands.remove_resource::<lifecycle::CountdownTimer>();
    commands.remove_resource::<lifecycle::ResultsTransitionTimer>();
    commands.remove_resource::<gate::GatePlanes>();
}

fn cleanup_race_progress(mut commands: Commands) {
    commands.remove_resource::<progress::RaceProgress>();
}
