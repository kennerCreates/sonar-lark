use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

use crate::race::progress::RaceProgress;

use super::orbit::MainCamera;
use super::settings::CameraSettings;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum CameraMode {
    #[default]
    Chase,
    Fpv,
    Spectator,
    /// Index into CourseCameras.cameras
    CourseCamera(usize),
}

/// Course cameras built from CourseData at race start.
#[derive(Resource, Default)]
pub struct CourseCameras {
    pub cameras: Vec<CourseCameraEntry>,
}

pub struct CourseCameraEntry {
    pub transform: Transform,
    pub label: Option<String>,
}

#[derive(Resource)]
pub struct CameraState {
    pub mode: CameraMode,
    /// Index into standings order (0 = leader). Used for FPV targeting.
    pub target_standings_index: usize,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            mode: CameraMode::Chase,
            target_standings_index: 0,
        }
    }
}

/// Build CourseCameras from CourseData. Primary goes first.
pub fn build_course_cameras(
    mut commands: Commands,
    course: Option<Res<crate::course::data::CourseData>>,
) {
    let Some(course) = course else {
        commands.insert_resource(CourseCameras::default());
        return;
    };

    let mut cameras: Vec<CourseCameraEntry> = Vec::new();
    let mut non_primary: Vec<CourseCameraEntry> = Vec::new();

    for cam in &course.cameras {
        let entry = CourseCameraEntry {
            transform: Transform::from_translation(cam.translation)
                .with_rotation(cam.rotation),
            label: cam.label.clone(),
        };
        if cam.is_primary {
            cameras.insert(0, entry);
        } else {
            non_primary.push(entry);
        }
    }
    cameras.append(&mut non_primary);
    commands.insert_resource(CourseCameras { cameras });
}

/// Camera switching:
/// - Number keys 1-9,0: CourseCamera(0..8) if present, else 1=fallback Chase, 2=Chase always
/// - Shift+F: FPV (cycle drone on repeat)
/// - Shift+S: Spectator
pub fn handle_camera_keys(
    mut state: ResMut<CameraState>,
    mut key_events: MessageReader<KeyboardInput>,
    progress: Option<Res<RaceProgress>>,
    course_cameras: Option<Res<CourseCameras>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let cam_count = course_cameras
        .as_ref()
        .map(|cc| cc.cameras.len())
        .unwrap_or(0);
    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    for event in key_events.read() {
        if !event.state.is_pressed() {
            continue;
        }

        // Shift combos
        if shift {
            match event.key_code {
                KeyCode::KeyF => {
                    if state.mode == CameraMode::Fpv {
                        let count = progress
                            .as_ref()
                            .map(|p| p.drone_states.len())
                            .unwrap_or(12);
                        if count > 0 {
                            state.target_standings_index =
                                (state.target_standings_index + 1) % count;
                        }
                    } else {
                        state.mode = CameraMode::Fpv;
                        state.target_standings_index = 0;
                    }
                }
                KeyCode::KeyS => {
                    state.mode = CameraMode::Spectator;
                }
                _ => {}
            }
            continue;
        }

        // Number keys (unmodified)
        match event.key_code {
            KeyCode::Digit1 => {
                if cam_count > 0 {
                    state.mode = CameraMode::CourseCamera(0);
                } else {
                    state.mode = CameraMode::Chase;
                }
            }
            KeyCode::Digit2 => {
                state.mode = CameraMode::Chase;
            }
            KeyCode::Digit3 => {
                if cam_count > 1 {
                    state.mode = CameraMode::CourseCamera(1);
                }
            }
            KeyCode::Digit4 => {
                if cam_count > 2 {
                    state.mode = CameraMode::CourseCamera(2);
                }
            }
            KeyCode::Digit5 => {
                if cam_count > 3 {
                    state.mode = CameraMode::CourseCamera(3);
                }
            }
            KeyCode::Digit6 => {
                if cam_count > 4 {
                    state.mode = CameraMode::CourseCamera(4);
                }
            }
            KeyCode::Digit7 => {
                if cam_count > 5 {
                    state.mode = CameraMode::CourseCamera(5);
                }
            }
            KeyCode::Digit8 => {
                if cam_count > 6 {
                    state.mode = CameraMode::CourseCamera(6);
                }
            }
            KeyCode::Digit9 => {
                if cam_count > 7 {
                    state.mode = CameraMode::CourseCamera(7);
                }
            }
            KeyCode::Digit0 => {
                if cam_count > 8 {
                    state.mode = CameraMode::CourseCamera(8);
                }
            }
            _ => {}
        }
    }
}

/// Set default camera at race start. Uses CourseCamera(0) if available.
pub fn reset_camera_for_race(
    mut state: ResMut<CameraState>,
    settings: Res<CameraSettings>,
    mut camera: Query<&mut Projection, With<MainCamera>>,
    course_cameras: Option<Res<CourseCameras>>,
) {
    let has_cameras = course_cameras.is_some_and(|cc| !cc.cameras.is_empty());
    state.mode = if has_cameras {
        CameraMode::CourseCamera(0)
    } else {
        CameraMode::Chase
    };
    state.target_standings_index = 0;
    if let Ok(mut projection) = camera.single_mut()
        && let Projection::Perspective(ref mut persp) = *projection
    {
        persp.fov = settings.fov_degrees.to_radians();
    }
}

pub fn reset_camera_on_exit(
    mut commands: Commands,
    mut state: ResMut<CameraState>,
    settings: Res<CameraSettings>,
    mut camera: Query<&mut Projection, With<MainCamera>>,
) {
    state.mode = CameraMode::Spectator;
    state.target_standings_index = 0;
    commands.remove_resource::<CourseCameras>();
    if let Ok(mut projection) = camera.single_mut()
        && let Projection::Perspective(ref mut persp) = *projection
    {
        persp.fov = settings.fov_degrees.to_radians();
    }
}
