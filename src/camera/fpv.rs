use bevy::prelude::*;

use crate::drone::components::{Drone, DroneDynamics, DronePhase};
use crate::race::progress::RaceProgress;

use super::orbit::MainCamera;
use super::switching::{CameraMode, CameraState};

const FOLLOW_DISTANCE: f32 = 2.5;
const FOLLOW_HEIGHT: f32 = 1.0;
const LOOK_AHEAD_OFFSET: f32 = 2.0;
const POSITION_SMOOTHING: f32 = 5.0;
const HEADING_SMOOTHING: f32 = 4.0;

/// Smoothed state for the FPV follow camera.
#[derive(Resource)]
pub struct FpvFollowState {
    pub smoothed_pos: Vec3,
    pub smoothed_heading: Vec3,
    pub initialized: bool,
}

impl Default for FpvFollowState {
    fn default() -> Self {
        Self {
            smoothed_pos: Vec3::ZERO,
            smoothed_heading: Vec3::NEG_Z,
            initialized: false,
        }
    }
}

/// Close stabilized follow camera behind the target drone.
/// Tracks drone heading (yaw only), ignores roll/pitch wobble.
pub fn fpv_camera_update(
    camera_state: Res<CameraState>,
    time: Res<Time>,
    progress: Option<Res<RaceProgress>>,
    drones: Query<(&Transform, &Drone, &DronePhase, &DroneDynamics)>,
    mut camera: Query<&mut Transform, (With<MainCamera>, Without<Drone>)>,
    mut follow: ResMut<FpvFollowState>,
) {
    if camera_state.mode != CameraMode::Fpv {
        follow.initialized = false;
        return;
    }

    let Some(progress) = progress.as_deref() else {
        return;
    };
    let standings = progress.standings();
    let idx = camera_state
        .target_standings_index
        .min(standings.len().saturating_sub(1));
    let Some(&(drone_idx, _)) = standings.get(idx) else {
        return;
    };

    let Some((drone_tf, _, phase, dynamics)) = drones
        .iter()
        .find(|(_, d, _, _)| d.index as usize == drone_idx)
    else {
        return;
    };

    let Ok(mut cam_tf) = camera.single_mut() else {
        return;
    };

    // For crashed drones, just look at the crash site
    if *phase == DronePhase::Crashed {
        cam_tf.look_at(drone_tf.translation, Vec3::Y);
        return;
    }

    let dt = time.delta_secs();

    // Extract heading from velocity (horizontal only) for stability.
    // Falls back to drone's forward direction if nearly stationary.
    let vel_horizontal = Vec3::new(dynamics.velocity.x, 0.0, dynamics.velocity.z);
    let raw_heading = if vel_horizontal.length_squared() > 1.0 {
        vel_horizontal.normalize()
    } else {
        let fwd = drone_tf.forward().as_vec3();
        Vec3::new(fwd.x, 0.0, fwd.z).normalize_or(Vec3::NEG_Z)
    };

    if !follow.initialized {
        follow.smoothed_heading = raw_heading;
        let target_pos =
            drone_tf.translation - raw_heading * FOLLOW_DISTANCE + Vec3::Y * FOLLOW_HEIGHT;
        follow.smoothed_pos = target_pos;
        follow.initialized = true;
    }

    // Smooth heading (yaw-only tracking, no roll/pitch)
    let heading_factor = 1.0 - (-HEADING_SMOOTHING * dt).exp();
    follow.smoothed_heading = follow
        .smoothed_heading
        .lerp(raw_heading, heading_factor)
        .normalize_or(Vec3::NEG_Z);

    // Desired camera position: behind and above the drone
    let target_pos = drone_tf.translation - follow.smoothed_heading * FOLLOW_DISTANCE
        + Vec3::Y * FOLLOW_HEIGHT;

    // Smooth position
    let pos_factor = 1.0 - (-POSITION_SMOOTHING * dt).exp();
    follow.smoothed_pos = follow.smoothed_pos.lerp(target_pos, pos_factor);

    cam_tf.translation = follow.smoothed_pos;

    // Look at drone (slightly ahead for natural framing)
    let look_target = drone_tf.translation + follow.smoothed_heading * LOOK_AHEAD_OFFSET;
    cam_tf.look_at(look_target, Vec3::Y);
}
