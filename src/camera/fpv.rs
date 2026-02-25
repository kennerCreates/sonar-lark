use bevy::prelude::*;

use crate::drone::components::{Drone, DronePhase};
use crate::race::progress::RaceProgress;

use super::orbit::MainCamera;
use super::switching::{CameraMode, CameraState};

const FPV_FORWARD_OFFSET: f32 = 0.3;
const FPV_UP_OFFSET: f32 = 0.15;
const ROTATION_SMOOTHING: f32 = 15.0;

/// First-person camera locked to the target drone's transform.
pub fn fpv_camera_update(
    camera_state: Res<CameraState>,
    time: Res<Time>,
    progress: Option<Res<RaceProgress>>,
    drones: Query<(&Transform, &Drone, &DronePhase)>,
    mut camera: Query<&mut Transform, (With<MainCamera>, Without<Drone>)>,
) {
    if camera_state.mode != CameraMode::Fpv {
        return;
    }

    // Resolve standings index → drone index → entity
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

    // Find the drone entity with matching index
    let Some((drone_transform, _, phase)) = drones
        .iter()
        .find(|(_, d, _)| d.index as usize == drone_idx)
    else {
        return;
    };

    let Ok(mut cam_transform) = camera.single_mut() else {
        return;
    };

    if *phase == DronePhase::Crashed {
        cam_transform.look_at(drone_transform.translation, Vec3::Y);
        return;
    }

    let dt = time.delta_secs();

    let drone_forward = drone_transform.forward().as_vec3();
    let drone_up = drone_transform.up().as_vec3();
    let target_pos = drone_transform.translation
        + drone_forward * FPV_FORWARD_OFFSET
        + drone_up * FPV_UP_OFFSET;

    cam_transform.translation = target_pos;

    // Smooth rotation to reduce physics wobble jitter
    let slerp_factor = 1.0 - (-ROTATION_SMOOTHING * dt).exp();
    cam_transform.rotation = cam_transform
        .rotation
        .slerp(drone_transform.rotation, slerp_factor);
}
