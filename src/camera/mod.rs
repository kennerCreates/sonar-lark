pub mod chase;
pub mod fpv;
pub mod orbit;
pub mod settings;
pub mod spectator;
pub mod spring;
pub mod switching;

use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;

use crate::rendering::{fog_color, FOG_END, FOG_START};
use crate::states::{AppState, DevMenuPage, EditorMode};
use chase::ChaseState;
use fpv::FpvFollowState;
use orbit::MainCamera;
use settings::CameraSettings;
use spectator::SpectatorOrbitState;
use switching::{CameraMode, CameraState, CourseCameras};

/// Run condition: true during Race or Results (camera follows drones in both).
fn in_race_or_results(state: Res<State<AppState>>) -> bool {
    matches!(state.get(), AppState::Race | AppState::Results)
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraState>()
            .init_resource::<CameraSettings>()
            .init_resource::<ChaseState>()
            .init_resource::<FpvFollowState>()
            .init_resource::<SpectatorOrbitState>()
            .add_systems(Startup, spawn_camera)
            // Race camera lifecycle (must run after course loading inserts CourseData)
            .add_systems(
                OnEnter(AppState::Race),
                (
                    switching::build_course_cameras,
                    switching::reset_camera_for_race,
                )
                    .chain()
                    .after(crate::course::loader::load_course),
            )
            .add_systems(OnExit(AppState::Results), switching::reset_camera_on_exit)
            // Race camera mode switching (active during Race and Results)
            .add_systems(
                Update,
                switching::handle_camera_keys.run_if(in_race_or_results),
            )
            // Mode-specific camera systems during Race and Results
            .add_systems(
                Update,
                spectator::spectator_movement
                    .run_if(in_race_or_results)
                    .run_if(camera_mode_is(CameraMode::Spectator)),
            )
            .add_systems(
                Update,
                chase::chase_camera_update
                    .run_if(in_race_or_results)
                    .run_if(camera_mode_is(CameraMode::Chase)),
            )
            .add_systems(
                Update,
                fpv::fpv_camera_update
                    .run_if(in_race_or_results)
                    .run_if(camera_mode_is(CameraMode::Fpv)),
            )
            .add_systems(
                Update,
                course_camera_update
                    .run_if(in_race_or_results)
                    .run_if(camera_mode_is_course_camera),
            )
            // Editor camera rig lifecycle
            .add_systems(OnEnter(AppState::Editor), orbit::setup_editor_camera)
            .add_systems(OnExit(AppState::Editor), orbit::teardown_editor_camera)
            // Mode-specific resets
            .add_systems(
                OnEnter(EditorMode::CourseEditor),
                orbit::reset_rig_for_course_editor,
            )
            // Editor camera systems
            .add_systems(
                Update,
                orbit::rts_camera_system.run_if(in_state(EditorMode::CourseEditor)),
            )
            // Workshop camera (lives under DevMenu)
            .add_systems(
                OnEnter(DevMenuPage::ObstacleWorkshop),
                (orbit::setup_editor_camera, orbit::reset_rig_for_workshop).chain(),
            )
            .add_systems(
                OnExit(DevMenuPage::ObstacleWorkshop),
                orbit::teardown_editor_camera,
            )
            .add_systems(
                Update,
                orbit::workshop_orbit_camera_system
                    .run_if(in_state(DevMenuPage::ObstacleWorkshop)),
            );
    }
}

fn camera_mode_is(mode: CameraMode) -> impl Fn(Res<CameraState>) -> bool {
    move |state: Res<CameraState>| state.mode == mode
}

fn camera_mode_is_course_camera(state: Res<CameraState>) -> bool {
    matches!(state.mode, CameraMode::CourseCamera(_))
}

/// Snaps the camera to the stored course camera transform.
fn course_camera_update(
    state: Res<CameraState>,
    course_cameras: Option<Res<CourseCameras>>,
    settings: Res<CameraSettings>,
    mut camera: Query<(&mut Transform, &mut Projection), With<MainCamera>>,
) {
    let CameraMode::CourseCamera(idx) = state.mode else {
        return;
    };
    let Some(cc) = course_cameras else { return };
    let Some(entry) = cc.cameras.get(idx) else { return };

    if let Ok((mut transform, mut projection)) = camera.single_mut() {
        *transform = entry.transform;
        if let Projection::Perspective(ref mut persp) = *projection {
            persp.fov = settings.fov_degrees.to_radians();
        }
    }
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
