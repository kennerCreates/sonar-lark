use bevy::prelude::*;

use crate::drone::components::{Drone, DroneDynamics, DronePhase};
use crate::race::progress::RaceProgress;

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

const CHASE_HEIGHT: f32 = 5.0;
const CHASE_BEHIND: f32 = 10.0;
const LOOK_AHEAD: f32 = 5.0;
const POSITION_SMOOTHING: f32 = 3.0;
const VELOCITY_SMOOTHING: f32 = 2.0;
const PROXIMITY_RADIUS: f32 = 15.0;
const LEADER_WEIGHT: f32 = 0.7;

/// Leader-focused chase camera. Follows the P1 drone closely,
/// blending in nearby drones for natural pack-racing framing.
pub fn chase_camera_update(
    camera_state: Res<CameraState>,
    time: Res<Time>,
    progress: Option<Res<RaceProgress>>,
    drones: Query<(&Transform, &Drone, &DroneDynamics, &DronePhase)>,
    mut camera: Query<&mut Transform, (With<MainCamera>, Without<Drone>)>,
    mut chase: ResMut<ChaseState>,
) {
    if camera_state.mode != CameraMode::Chase {
        chase.initialized = false;
        return;
    }

    let dt = time.delta_secs();

    // Find the leader drone via standings
    let leader_idx = progress
        .as_ref()
        .and_then(|p| {
            let standings = p.standings();
            standings.first().map(|&(idx, _)| idx)
        });

    // Find leader entity
    let leader_data = leader_idx.and_then(|idx| {
        drones
            .iter()
            .find(|(_, d, _, _)| d.index as usize == idx)
    });

    // Fall back to first non-crashed drone if no standings yet
    let (leader_pos, leader_vel) = if let Some((tf, _, dynamics, _)) = leader_data {
        (tf.translation, dynamics.velocity)
    } else {
        // Pre-race fallback: center of all drones
        let mut center = Vec3::ZERO;
        let mut avg_vel = Vec3::ZERO;
        let mut count = 0u32;
        for (tf, _, dynamics, phase) in &drones {
            if *phase != DronePhase::Crashed {
                center += tf.translation;
                avg_vel += dynamics.velocity;
                count += 1;
            }
        }
        if count == 0 {
            return;
        }
        (center / count as f32, avg_vel / count as f32)
    };

    // Find nearby non-crashed drones within proximity radius of leader
    let mut nearby_center = Vec3::ZERO;
    let mut nearby_count = 0u32;
    for (tf, _, _, phase) in &drones {
        if *phase == DronePhase::Crashed {
            continue;
        }
        let dist = tf.translation.distance(leader_pos);
        if dist > 0.1 && dist < PROXIMITY_RADIUS {
            nearby_center += tf.translation;
            nearby_count += 1;
        }
    }

    // Blend target: leader-focused, with nearby drones pulling the focus slightly
    let target_center = if nearby_count > 0 {
        nearby_center /= nearby_count as f32;
        leader_pos * LEADER_WEIGHT + nearby_center * (1.0 - LEADER_WEIGHT)
    } else {
        leader_pos
    };

    if !chase.initialized {
        chase.smoothed_center = target_center;
        chase.smoothed_velocity = if leader_vel.length_squared() > 0.1 {
            leader_vel
        } else {
            Vec3::NEG_Z
        };
        chase.initialized = true;
    }

    // Smooth the center and velocity
    let pos_factor = 1.0 - (-POSITION_SMOOTHING * dt).exp();
    let vel_factor = 1.0 - (-VELOCITY_SMOOTHING * dt).exp();
    chase.smoothed_center = chase.smoothed_center.lerp(target_center, pos_factor);
    chase.smoothed_velocity = chase.smoothed_velocity.lerp(leader_vel, vel_factor);

    // Compute camera direction from velocity (horizontal only)
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

    cam_transform.translation = cam_transform.translation.lerp(target_pos, pos_factor);
    cam_transform.look_at(look_target, Vec3::Y);
}
