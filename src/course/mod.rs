pub mod data;
pub mod discovery;
pub mod loader;
pub mod location;

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
        // Spawn course obstacles once glTF is loaded and course data is available
        .add_systems(
            Update,
            loader::spawn_course
                .run_if(in_state(AppState::Race))
                .run_if(spawning::obstacles_gltf_ready)
                .run_if(resource_exists::<data::CourseData>)
                .run_if(not(resource_exists::<loader::CourseSpawned>)),
        )
        // Reset spawn guard so obstacles can be re-spawned on next Race entry
        .add_systems(OnExit(AppState::Race), loader::cleanup_course_spawned)
        // Obstacles use DespawnOnExit(Results) to stay visible as a backdrop.
        // For the Race → Editor path (bypasses Results), manually despawn them.
        .add_systems(OnEnter(AppState::Editor), loader::despawn_course_obstacles);
    }
}
