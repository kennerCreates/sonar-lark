pub mod data;
pub mod loader;

use bevy::prelude::*;

use crate::obstacle::spawning;
use crate::states::AppState;

pub struct CoursePlugin;

impl Plugin for CoursePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::Race),
            (spawning::load_obstacles_gltf, loader::load_course, loader::spawn_course).chain(),
        );
    }
}
