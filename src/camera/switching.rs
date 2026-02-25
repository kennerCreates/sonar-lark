use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

use crate::race::progress::RaceProgress;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum CameraMode {
    #[default]
    Chase,
    Fpv,
    Spectator,
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

/// Camera switching via number keys:
/// 1 = Chase, 2 = Spectator, 3 = FPV (repeated press cycles drone).
pub fn handle_camera_keys(
    mut state: ResMut<CameraState>,
    mut key_events: MessageReader<KeyboardInput>,
    progress: Option<Res<RaceProgress>>,
) {
    for event in key_events.read() {
        if !event.state.is_pressed() {
            continue;
        }
        match event.key_code {
            KeyCode::Digit1 => {
                state.mode = CameraMode::Chase;
            }
            KeyCode::Digit2 => {
                state.mode = CameraMode::Spectator;
            }
            KeyCode::Digit3 => {
                if state.mode == CameraMode::Fpv {
                    // Already in FPV — cycle to next drone
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
            _ => {}
        }
    }
}

pub fn reset_camera_for_race(mut state: ResMut<CameraState>) {
    state.mode = CameraMode::Chase;
    state.target_standings_index = 0;
}

pub fn reset_camera_on_exit(mut state: ResMut<CameraState>) {
    state.mode = CameraMode::Spectator;
    state.target_standings_index = 0;
}
