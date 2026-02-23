pub mod ai;
pub mod components;
pub mod physics;
pub mod spawning;

use bevy::prelude::*;

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
            // Physics chain in FixedUpdate
            .add_systems(
                FixedUpdate,
                (
                    ai::update_ai_targets,
                    ai::compute_racing_line,
                    physics::pid_compute,
                    physics::apply_forces,
                    physics::integrate_motion,
                    physics::clamp_transform,
                )
                    .chain()
                    .run_if(in_state(AppState::Race)),
            )
            // Cleanup resources on exit
            .add_systems(OnExit(AppState::Race), spawning::cleanup_drone_resources);
    }
}
