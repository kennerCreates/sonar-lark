pub mod chase;
pub mod fpv;
pub mod orbit;
pub mod settings;
pub mod spectator;
pub mod switching;

use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;

use crate::rendering::{fog_color, FOG_END, FOG_START};
use crate::states::{AppState, EditorMode};
use chase::ChaseState;
use orbit::MainCamera;
use settings::CameraSettings;
use spectator::SpectatorSettings;
use switching::{CameraMode, CameraState};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraState>()
            .init_resource::<SpectatorSettings>()
            .init_resource::<CameraSettings>()
            .init_resource::<ChaseState>()
            .add_systems(Startup, spawn_camera)
            // Race camera lifecycle
            .add_systems(OnEnter(AppState::Race), switching::reset_camera_for_race)
            .add_systems(OnExit(AppState::Race), switching::reset_camera_on_exit)
            // Race camera mode switching (always active during race)
            .add_systems(
                Update,
                (
                    switching::cycle_camera_mode,
                    switching::cycle_target_drone,
                )
                    .run_if(in_state(AppState::Race)),
            )
            // Mode-specific camera systems during Race
            .add_systems(
                Update,
                spectator::spectator_movement
                    .run_if(in_state(AppState::Race))
                    .run_if(camera_mode_is(CameraMode::Spectator)),
            )
            .add_systems(
                Update,
                chase::chase_camera_update
                    .run_if(in_state(AppState::Race))
                    .run_if(camera_mode_is(CameraMode::Chase)),
            )
            .add_systems(
                Update,
                fpv::fpv_camera_update
                    .run_if(in_state(AppState::Race))
                    .run_if(camera_mode_is(CameraMode::Fpv)),
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

fn camera_mode_is(mode: CameraMode) -> impl Fn(Res<CameraState>) -> bool {
    move |state: Res<CameraState>| state.mode == mode
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        MainCamera,
        Transform::from_xyz(0.0, 20.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
        DistanceFog {
            color: fog_color(),
            directional_light_color: Color::NONE,
            directional_light_exponent: 0.0,
            falloff: FogFalloff::Linear {
                start: FOG_START,
                end: FOG_END,
            },
        },
    ));
}
