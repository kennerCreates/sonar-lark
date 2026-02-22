pub mod definition;
pub mod library;
pub mod spawning;

use bevy::prelude::*;

pub struct ObstaclePlugin;

impl Plugin for ObstaclePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<library::ObstacleLibrary>();
    }
}
