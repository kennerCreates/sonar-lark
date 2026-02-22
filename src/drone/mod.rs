pub mod components;
pub mod physics;
pub mod ai;
pub mod spawning;

use bevy::prelude::*;

pub struct DronePlugin;

impl Plugin for DronePlugin {
    fn build(&self, _app: &mut App) {
        // Drone systems will be registered in Phase 6
    }
}
