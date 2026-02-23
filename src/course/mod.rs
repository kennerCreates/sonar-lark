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
            (spawning::load_obstacles_gltf, loader::load_course).chain(),
        )
        // Poll each frame until glTF is loaded, then spawn obstacles once
        .add_systems(
            Update,
            loader::spawn_course.run_if(in_state(AppState::Race)),
        )
        .add_systems(OnExit(AppState::Race), loader::cleanup_course_spawned);
    }
}
