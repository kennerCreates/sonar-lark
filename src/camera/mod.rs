pub mod chase;
pub mod fpv;
pub mod spectator;
pub mod switching;

use bevy::prelude::*;

use crate::states::{AppState, EditorMode};
use spectator::SpectatorSettings;
use switching::CameraState;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraState>()
            .init_resource::<SpectatorSettings>()
            .add_systems(Startup, spawn_camera)
            .add_systems(
                Update,
                spectator::spectator_movement
                    .run_if(in_state(AppState::Race).or(in_state(EditorMode::CourseEditor))),
            );
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 20.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
