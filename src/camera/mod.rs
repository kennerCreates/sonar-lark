pub mod chase;
pub mod fpv;
pub mod orbit;
pub mod settings;
pub mod spectator;
pub mod switching;

use bevy::prelude::*;

use crate::states::{AppState, EditorMode};
use orbit::MainCamera;
use settings::CameraSettings;
use spectator::SpectatorSettings;
use switching::CameraState;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraState>()
            .init_resource::<SpectatorSettings>()
            .init_resource::<CameraSettings>()
            .add_systems(Startup, spawn_camera)
            // Spectator for Race only
            .add_systems(
                Update,
                spectator::spectator_movement.run_if(in_state(AppState::Race)),
            )
            // Editor camera rig lifecycle
            .add_systems(OnEnter(AppState::Editor), orbit::setup_editor_camera)
            .add_systems(OnExit(AppState::Editor), orbit::teardown_editor_camera)
            // Mode-specific resets
            .add_systems(
                OnEnter(EditorMode::ObstacleWorkshop),
                orbit::reset_rig_for_workshop,
            )
            .add_systems(
                OnEnter(EditorMode::CourseEditor),
                orbit::reset_rig_for_course_editor,
            )
            // Editor camera systems
            .add_systems(
                Update,
                orbit::rts_camera_system.run_if(in_state(EditorMode::CourseEditor)),
            )
            .add_systems(
                Update,
                orbit::workshop_orbit_camera_system
                    .run_if(in_state(EditorMode::ObstacleWorkshop)),
            );
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        MainCamera,
        Transform::from_xyz(0.0, 20.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
