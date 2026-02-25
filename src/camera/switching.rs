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

/// Cycle camera mode on C key press: Chase → FPV → Spectator → Chase.
pub fn cycle_camera_mode(
    mut state: ResMut<CameraState>,
    mut key_events: MessageReader<KeyboardInput>,
) {
    for event in key_events.read() {
        if event.state.is_pressed() && event.key_code == KeyCode::KeyC {
            state.mode = match state.mode {
                CameraMode::Chase => CameraMode::Fpv,
                CameraMode::Fpv => CameraMode::Spectator,
                CameraMode::Spectator => CameraMode::Chase,
            };
        }
    }
}

/// Cycle target drone in FPV mode: [ for previous, ] for next (standings order).
pub fn cycle_target_drone(
    mut state: ResMut<CameraState>,
    mut key_events: MessageReader<KeyboardInput>,
    progress: Option<Res<RaceProgress>>,
) {
    if state.mode != CameraMode::Fpv {
        return;
    }
    let count = progress
        .as_ref()
        .map(|p| p.drone_states.len())
        .unwrap_or(12);
    if count == 0 {
        return;
    }

    for event in key_events.read() {
        if !event.state.is_pressed() {
            continue;
        }
        match event.key_code {
            KeyCode::BracketRight => {
                state.target_standings_index = (state.target_standings_index + 1) % count;
            }
            KeyCode::BracketLeft => {
                state.target_standings_index =
                    (state.target_standings_index + count - 1) % count;
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
