use bevy::prelude::*;

use crate::drone::components::{Drone, DroneDynamics, DronePhase};

use super::orbit::MainCamera;
use super::switching::{CameraMode, CameraState};

/// Smoothed state for the chase camera, persists across frames.
#[derive(Resource)]
pub struct ChaseState {
    pub smoothed_center: Vec3,
    pub smoothed_velocity: Vec3,
    pub initialized: bool,
}

impl Default for ChaseState {
    fn default() -> Self {
        Self {
            smoothed_center: Vec3::ZERO,
            smoothed_velocity: Vec3::NEG_Z,
            initialized: false,
        }
    }
}

const CHASE_HEIGHT: f32 = 10.0;
const CHASE_BEHIND: f32 = 18.0;
const LOOK_AHEAD: f32 = 8.0;
const POSITION_SMOOTHING: f32 = 2.0;
const VELOCITY_SMOOTHING: f32 = 1.5;

/// Broadcast-style chase camera that follows the center-of-mass of active drones.
pub fn chase_camera_update(
    camera_state: Res<CameraState>,
    time: Res<Time>,
    drones: Query<(&Transform, &DroneDynamics, &DronePhase), With<Drone>>,
    mut camera: Query<&mut Transform, (With<MainCamera>, Without<Drone>)>,
    mut chase: ResMut<ChaseState>,
) {
    if camera_state.mode != CameraMode::Chase {
        chase.initialized = false;
        return;
    }

    let dt = time.delta_secs();

    // Gather active drone positions and velocities
    let mut center = Vec3::ZERO;
    let mut avg_vel = Vec3::ZERO;
    let mut count = 0u32;

    for (transform, dynamics, phase) in &drones {
        if *phase == DronePhase::Crashed {
            continue;
        }
        center += transform.translation;
        avg_vel += dynamics.velocity;
        count += 1;
    }

    if count == 0 {
        // Fall back to all drones including crashed
        for (transform, dynamics, _) in &drones {
            center += transform.translation;
            avg_vel += dynamics.velocity;
            count += 1;
        }
    }

    if count == 0 {
        return;
    }

    center /= count as f32;
    avg_vel /= count as f32;

    if !chase.initialized {
        chase.smoothed_center = center;
        chase.smoothed_velocity = if avg_vel.length_squared() > 0.1 {
            avg_vel
        } else {
            Vec3::NEG_Z
        };
        chase.initialized = true;
    }

    // Smooth the center and velocity
    let pos_factor = 1.0 - (-POSITION_SMOOTHING * dt).exp();
    let vel_factor = 1.0 - (-VELOCITY_SMOOTHING * dt).exp();
    chase.smoothed_center = chase.smoothed_center.lerp(center, pos_factor);
    chase.smoothed_velocity = chase.smoothed_velocity.lerp(avg_vel, vel_factor);

    // Compute camera direction from average velocity (horizontal only)
    let mut forward = Vec3::new(
        chase.smoothed_velocity.x,
        0.0,
        chase.smoothed_velocity.z,
    );
    if forward.length_squared() < 0.01 {
        forward = Vec3::NEG_Z;
    }
    forward = forward.normalize();

    let Ok(mut cam_transform) = camera.single_mut() else {
        return;
    };

    let target_pos =
        chase.smoothed_center - forward * CHASE_BEHIND + Vec3::Y * CHASE_HEIGHT;
    let look_target = chase.smoothed_center + forward * LOOK_AHEAD;

    // Smooth camera position
    cam_transform.translation = cam_transform.translation.lerp(target_pos, pos_factor);
    cam_transform.look_at(look_target, Vec3::Y);
}
