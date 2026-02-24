pub mod ai;
pub mod components;
pub mod debug_draw;
pub mod dev_dashboard;
pub mod physics;
pub mod spawning;

use bevy::prelude::*;

use crate::race::lifecycle::drones_are_active;
use crate::states::AppState;
use components::AiTuningParams;

pub struct DronePlugin;

impl Plugin for DronePlugin {
    fn build(&self, app: &mut App) {
        app
            // AI tuning params persist across race restarts
            .init_resource::<AiTuningParams>()
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
                    ai::update_ai_targets.run_if(drones_are_active),
                    ai::compute_racing_line.run_if(drones_are_active),
                    physics::hover_target.run_if(not(drones_are_active)),
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
            // Dev dashboard (F4 to toggle)
            .add_systems(
                Update,
                (
                    dev_dashboard::toggle_dev_dashboard,
                    dev_dashboard::handle_param_buttons,
                    dev_dashboard::handle_reset_button,
                    dev_dashboard::update_param_labels,
                    dev_dashboard::update_button_colors,
                )
                    .run_if(in_state(AppState::Race)),
            )
            // Cleanup resources on exit
            .add_systems(OnExit(AppState::Race), spawning::cleanup_drone_resources);
    }
}
