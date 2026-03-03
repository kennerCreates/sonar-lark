pub mod camera_hud;
pub mod collision;
pub mod collision_math;
pub mod gate;
pub mod leaderboard;
pub mod lifecycle;
pub mod overlays;
pub mod progress;
pub mod script;
pub mod start_button;
pub mod timing;

use bevy::prelude::*;

use crate::pilot::portrait::cache::setup_portrait_cache;
use crate::states::AppState;

pub struct RacePlugin;

impl Plugin for RacePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::Race),
            (setup_race, start_button::setup_race_ui, leaderboard::setup_leaderboard.after(setup_portrait_cache), camera_hud::setup_camera_hud, lifecycle::load_race_sounds),
        )
            // Build GatePlanes resource once gate entities are spawned
            .add_systems(
                Update,
                gate::build_gate_planes
                    .run_if(in_state(AppState::Race))
                    .run_if(not(resource_exists::<gate::GatePlanes>)),
            )
            // Build ObstacleCollisionCache once obstacle entities are spawned
            .add_systems(
                Update,
                collision::build_obstacle_collision_cache
                    .run_if(in_state(AppState::Race))
                    .run_if(not(resource_exists::<collision::ObstacleCollisionCache>)),
            )
            // Race logic chain: ordering matters for correctness.
            // Gate/collision detection replaced by fire_scripted_events in the choreography chain.
            .add_systems(
                Update,
                (
                    lifecycle::tick_countdown,
                    lifecycle::generate_race_script_system,
                    timing::tick_race_clock,
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
                    start_button::handle_start_race_button,
                    start_button::update_start_button_visuals,
                    start_button::update_start_button_text,
                    overlays::show_no_gates_banner,
                    overlays::handle_open_editor_button,
                    overlays::manage_countdown_text,
                    overlays::update_race_clock_display,
                    leaderboard::update_leaderboard,
                    camera_hud::update_camera_hud,
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
    commands.remove_resource::<lifecycle::RaceStartSound>();
    commands.remove_resource::<lifecycle::RaceEndSound>();
    commands.remove_resource::<gate::GatePlanes>();
    commands.remove_resource::<collision::ObstacleCollisionCache>();
    commands.remove_resource::<script::RaceScript>();
    commands.remove_resource::<script::RaceEventLog>();
}

fn cleanup_race_progress(mut commands: Commands) {
    commands.remove_resource::<progress::RaceProgress>();
}
