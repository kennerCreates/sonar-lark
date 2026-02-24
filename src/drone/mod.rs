pub mod ai;
pub mod components;
pub mod debug_draw;
pub mod physics;
pub mod spawning;

use bevy::prelude::*;

use crate::race::lifecycle::race_is_running;
use crate::states::AppState;

pub struct DronePlugin;

impl Plugin for DronePlugin {
    fn build(&self, app: &mut App) {
        app
            // Start loading drone glTF when entering Race
            .add_systems(OnEnter(AppState::Race), spawning::load_drone_gltf)
            // Poll for asset readiness and spawn drones once ready
            .add_systems(
                Update,
                (spawning::setup_drone_assets, spawning::spawn_drones)
                    .chain()
                    .run_if(in_state(AppState::Race)),
            )
            // Physics chain in FixedUpdate.
            // AI systems only run when racing; physics always runs so drones hover at start.
            .add_systems(
                FixedUpdate,
                (
                    ai::update_ai_targets.run_if(race_is_running),
                    ai::compute_racing_line.run_if(race_is_running),
                    physics::hover_target.run_if(not(race_is_running)),
                    physics::position_pid,
                    physics::attitude_controller,
                    physics::motor_lag,
                    physics::apply_forces,
                    physics::integrate_motion,
                    physics::clamp_transform,
                )
                    .chain()
                    .run_if(in_state(AppState::Race)),
            )
            // Flight debug visualization (F3 to toggle)
            .add_systems(
                Update,
                (
                    debug_draw::toggle_debug_draw,
                    debug_draw::draw_spline_path,
                    debug_draw::draw_gate_markers,
                    debug_draw::draw_drone_state,
                    debug_draw::draw_progress_indicators,
                )
                    .run_if(in_state(AppState::Race)),
            )
            // Cleanup resources on exit
            .add_systems(OnExit(AppState::Race), spawning::cleanup_drone_resources);
    }
}
