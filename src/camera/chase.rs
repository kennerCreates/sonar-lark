use bevy::prelude::*;
use bevy::time::Fixed;

use crate::drone::components::{Drone, DroneDynamics, DronePhase};
use crate::drone::interpolation::PreviousTranslation;
use crate::race::gate::{GateForward, GateIndex};
use crate::race::progress::RaceProgress;

use super::orbit::MainCamera;
use super::spring::{SpringF32, SpringVec3};
use super::switching::{CameraMode, CameraState};

/// Smoothed state for the chase camera, persists across frames.
#[derive(Resource)]
pub struct ChaseState {
    pub center: SpringVec3,
    pub velocity_dir: SpringVec3,
    pub look_target: SpringVec3,
    pub fov: SpringF32,
    pub initialized: bool,
}

impl Default for ChaseState {
    fn default() -> Self {
        Self {
            center: SpringVec3::new(Vec3::ZERO),
            velocity_dir: SpringVec3::new(Vec3::NEG_Z),
            look_target: SpringVec3::new(Vec3::ZERO),
            fov: SpringF32::new(CHASE_BASE_FOV_DEG.to_radians()),
            initialized: false,
        }
    }
}

const CHASE_HEIGHT: f32 = 5.0;
const CHASE_BEHIND: f32 = 10.0;
const LOOK_AHEAD: f32 = 5.0;
const PROXIMITY_RADIUS: f32 = 15.0;
const LEADER_WEIGHT: f32 = 0.7;

// Spring half-lives (seconds to reach halfway to target)
const CENTER_HALF_LIFE: f32 = 0.15;
const VELOCITY_HALF_LIFE: f32 = 0.20;
const LOOK_TARGET_HALF_LIFE: f32 = 0.08;

// Dynamic FOV
const CHASE_BASE_FOV_DEG: f32 = 60.0;
const CHASE_FOV_INCREASE_DEG: f32 = 10.0;
const CHASE_FOV_HALF_LIFE: f32 = 0.30;
const MAX_DRONE_SPEED: f32 = 55.0;

// Finish camera: wider establishing shot of the finish gate
const FINISH_BEHIND: f32 = 25.0;
const FINISH_HEIGHT: f32 = 10.0;
const FINISH_HALF_LIFE: f32 = 0.5;

/// Leader-focused chase camera. Follows the P1 drone closely,
/// blending in nearby drones for natural pack-racing framing.
pub fn chase_camera_update(
    camera_state: Res<CameraState>,
    time: Res<Time>,
    fixed_time: Res<Time<Fixed>>,
    progress: Option<Res<RaceProgress>>,
    drones: Query<(
        &Transform,
        &Drone,
        &DroneDynamics,
        &DronePhase,
        &PreviousTranslation,
    )>,
    gates: Query<(&GlobalTransform, &GateIndex, &GateForward)>,
    mut camera: Query<(&mut Transform, &mut Projection), (With<MainCamera>, Without<Drone>)>,
    mut chase: ResMut<ChaseState>,
) {
    if camera_state.mode != CameraMode::Chase {
        chase.initialized = false;
        return;
    }

    let dt = time.delta_secs();
    let alpha = fixed_time.overstep_fraction();

    // Find the best drone to follow: prefer highest-ranked still-Racing drone.
    // If no drones are racing, freeze the camera at its current position
    // (keeps framing the finish area instead of following returning drones).
    let target_data = progress
        .as_ref()
        .and_then(|p| {
            let standings = p.standings();
            // First: highest-ranked drone that's still Racing
            standings
                .iter()
                .find_map(|&(idx, _)| {
                    drones.iter().find(|(_, d, _, phase, _)| {
                        d.index as usize == idx && **phase == DronePhase::Racing
                    })
                })
                // Fallback: highest-ranked non-crashed drone (pre-race / idle)
                .or_else(|| {
                    standings.iter().find_map(|&(idx, _)| {
                        drones.iter().find(|(_, d, _, phase, _)| {
                            d.index as usize == idx && **phase != DronePhase::Crashed
                                && **phase != DronePhase::Returning
                        })
                    })
                })
        });

    // If all drones are returning/crashed, smoothly pull back to frame the finish gate
    let (leader_pos, leader_vel) = if let Some((tf, _, dynamics, _, prev)) = target_data {
        let interp_pos = prev.0.lerp(tf.translation, alpha);
        (interp_pos, dynamics.velocity)
    } else if chase.initialized {
        // Find the finish gate (gate 0) and spring-smooth toward it
        let finish_gate = gates.iter().find(|(_, idx, _)| idx.0 == 0);
        let Ok((mut cam_transform, mut projection)) = camera.single_mut() else {
            return;
        };

        if let Some((gate_tf, _, gate_fwd)) = finish_gate {
            let gate_pos = gate_tf.translation();
            let gate_forward = gate_fwd.0.normalize_or(Vec3::NEG_Z);

            // Spring-smooth center toward gate position
            chase.center.update(gate_pos, FINISH_HALF_LIFE, dt);
            // Spring-smooth velocity_dir toward gate forward (camera looks from approach side)
            chase.velocity_dir.update(gate_forward, FINISH_HALF_LIFE, dt);
        }
        // If no gate found, springs just hold their last values

        let forward = Vec3::new(
            chase.velocity_dir.value.x,
            0.0,
            chase.velocity_dir.value.z,
        )
        .normalize_or(Vec3::NEG_Z);
        cam_transform.translation =
            chase.center.value - forward * FINISH_BEHIND + Vec3::Y * FINISH_HEIGHT;
        let look_target = chase.center.value;
        chase.look_target.update(look_target, FINISH_HALF_LIFE, dt);
        cam_transform.look_at(chase.look_target.value, Vec3::Y);

        chase.fov.update(CHASE_BASE_FOV_DEG.to_radians(), CHASE_FOV_HALF_LIFE, dt);
        if let Projection::Perspective(ref mut persp) = *projection {
            persp.fov = chase.fov.value;
        }
        return;
    } else {
        return;
    };

    // Find nearby non-crashed drones within proximity radius of leader
    let mut nearby_center = Vec3::ZERO;
    let mut nearby_count = 0u32;
    for (tf, _, _, phase, prev) in &drones {
        if *phase == DronePhase::Crashed {
            continue;
        }
        let interp_pos = prev.0.lerp(tf.translation, alpha);
        let dist = interp_pos.distance(leader_pos);
        if dist > 0.1 && dist < PROXIMITY_RADIUS {
            nearby_center += interp_pos;
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
        chase.center = SpringVec3::new(target_center);
        chase.velocity_dir = SpringVec3::new(if leader_vel.length_squared() > 0.1 {
            leader_vel
        } else {
            Vec3::NEG_Z
        });
        let forward = Vec3::new(
            chase.velocity_dir.value.x,
            0.0,
            chase.velocity_dir.value.z,
        )
        .normalize_or(Vec3::NEG_Z);
        chase.look_target = SpringVec3::new(target_center + forward * LOOK_AHEAD);
        chase.fov = SpringF32::new(CHASE_BASE_FOV_DEG.to_radians());
        chase.initialized = true;
    }

    // Spring-smooth the center and velocity direction
    chase.center.update(target_center, CENTER_HALF_LIFE, dt);
    chase.velocity_dir.update(leader_vel, VELOCITY_HALF_LIFE, dt);

    // Compute camera direction from velocity (horizontal only)
    let mut forward = Vec3::new(
        chase.velocity_dir.value.x,
        0.0,
        chase.velocity_dir.value.z,
    );
    if forward.length_squared() < 0.01 {
        forward = Vec3::NEG_Z;
    }
    forward = forward.normalize();

    let Ok((mut cam_transform, mut projection)) = camera.single_mut() else {
        return;
    };

    // Derive camera position directly from spring-smoothed center
    cam_transform.translation =
        chase.center.value - forward * CHASE_BEHIND + Vec3::Y * CHASE_HEIGHT;

    // Spring-smooth look target for stable rotation
    let raw_look_target = chase.center.value + forward * LOOK_AHEAD;
    chase.look_target.update(raw_look_target, LOOK_TARGET_HALF_LIFE, dt);
    cam_transform.look_at(chase.look_target.value, Vec3::Y);

    // Dynamic FOV based on leader speed
    let speed = leader_vel.length();
    let speed_fraction = (speed / MAX_DRONE_SPEED).clamp(0.0, 1.0);
    let target_fov =
        (CHASE_BASE_FOV_DEG + speed_fraction * CHASE_FOV_INCREASE_DEG).to_radians();
    chase.fov.update(target_fov, CHASE_FOV_HALF_LIFE, dt);
    if let Projection::Perspective(ref mut persp) = *projection {
        persp.fov = chase.fov.value;
    }
}
