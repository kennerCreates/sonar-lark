use bevy::prelude::*;
use bevy::time::Fixed;

use crate::drone::components::{AIController, Drone, DroneDynamics, DronePhase, POINTS_PER_GATE};
use crate::drone::interpolation::PreviousTranslation;
use crate::race::progress::RaceProgress;

use super::orbit::MainCamera;
use super::spring::{SpringF32, SpringVec3};
use super::switching::{CameraMode, CameraState};

const FOLLOW_DISTANCE: f32 = 2.5;
const FOLLOW_HEIGHT: f32 = 1.0;
const LOOK_AHEAD_OFFSET: f32 = 2.0;
const LOOK_AHEAD_SPLINE_T: f32 = 0.5;

// Spring half-lives (seconds to reach halfway to target)
const POSITION_HALF_LIFE: f32 = 0.08;
const HEADING_HALF_LIFE: f32 = 0.12;
const LOOK_TARGET_HALF_LIFE: f32 = 0.06;

// Dynamic FOV
const FPV_BASE_FOV_DEG: f32 = 60.0;
const FPV_FOV_INCREASE_DEG: f32 = 15.0;
const FPV_FOV_HALF_LIFE: f32 = 0.25;
const MAX_DRONE_SPEED: f32 = 55.0;

/// Smoothed state for the FPV follow camera.
#[derive(Resource)]
pub struct FpvFollowState {
    pub position: SpringVec3,
    pub heading: SpringVec3,
    pub look_target: SpringVec3,
    pub fov: SpringF32,
    pub initialized: bool,
}

impl Default for FpvFollowState {
    fn default() -> Self {
        Self {
            position: SpringVec3::new(Vec3::ZERO),
            heading: SpringVec3::new(Vec3::NEG_Z),
            look_target: SpringVec3::new(Vec3::ZERO),
            fov: SpringF32::new(FPV_BASE_FOV_DEG.to_radians()),
            initialized: false,
        }
    }
}

/// Close stabilized follow camera behind the target drone.
/// Tracks drone heading (yaw only), ignores roll/pitch wobble.
pub fn fpv_camera_update(
    camera_state: Res<CameraState>,
    time: Res<Time>,
    fixed_time: Res<Time<Fixed>>,
    progress: Option<Res<RaceProgress>>,
    drones: Query<(
        &Transform,
        &Drone,
        &DronePhase,
        &DroneDynamics,
        &AIController,
        &PreviousTranslation,
    )>,
    mut camera: Query<(&mut Transform, &mut Projection), (With<MainCamera>, Without<Drone>)>,
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

    let Some((drone_tf, _, phase, dynamics, ai, prev)) = drones
        .iter()
        .find(|(_, d, _, _, _, _)| d.index as usize == drone_idx)
    else {
        return;
    };

    let Ok((mut cam_tf, mut projection)) = camera.single_mut() else {
        return;
    };

    // Interpolated drone position (smooth between fixed ticks)
    let alpha = fixed_time.overstep_fraction();
    let interp_pos = prev.0.lerp(drone_tf.translation, alpha);

    // For crashed drones, just look at the crash site
    if *phase == DronePhase::Crashed {
        cam_tf.look_at(interp_pos, Vec3::Y);
        return;
    }

    // For returning/idle drones, freeze the camera
    if *phase == DronePhase::Returning || *phase == DronePhase::Idle {
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
        follow.heading = SpringVec3::new(raw_heading);
        let target_pos =
            interp_pos - raw_heading * FOLLOW_DISTANCE + Vec3::Y * FOLLOW_HEIGHT;
        follow.position = SpringVec3::new(target_pos);
        follow.look_target =
            SpringVec3::new(interp_pos + raw_heading * LOOK_AHEAD_OFFSET);
        follow.fov = SpringF32::new(FPV_BASE_FOV_DEG.to_radians());
        follow.initialized = true;
    }

    // Spring-smooth heading (yaw-only tracking, no roll/pitch)
    follow.heading.update(raw_heading, HEADING_HALF_LIFE, dt);
    let smoothed_heading = Vec3::new(
        follow.heading.value.x,
        0.0,
        follow.heading.value.z,
    )
    .normalize_or(Vec3::NEG_Z);

    // Desired camera position: behind and above the drone
    let target_pos =
        interp_pos - smoothed_heading * FOLLOW_DISTANCE + Vec3::Y * FOLLOW_HEIGHT;

    // Spring-smooth position
    follow.position.update(target_pos, POSITION_HALF_LIFE, dt);
    cam_tf.translation = follow.position.value;

    // Compute look target: spline-based during Racing/VictoryLap, velocity-based otherwise
    let raw_look_target = if matches!(*phase, DronePhase::Racing | DronePhase::VictoryLap)
        && ai.spline_t > 0.0
    {
        let cycle_t = ai.gate_count as f32 * POINTS_PER_GATE;
        let sample_t = ai.spline_t + LOOK_AHEAD_SPLINE_T;
        let spline_ahead = ai.spline.position(sample_t.rem_euclid(cycle_t));
        // Blend: mostly spline target, grounded by actual drone position
        interp_pos * 0.3 + spline_ahead * 0.7
    } else {
        interp_pos + smoothed_heading * LOOK_AHEAD_OFFSET
    };

    // Spring-smooth look target for stable rotation
    follow.look_target.update(raw_look_target, LOOK_TARGET_HALF_LIFE, dt);
    cam_tf.look_at(follow.look_target.value, Vec3::Y);

    // Dynamic FOV based on drone speed
    let speed = dynamics.velocity.length();
    let speed_fraction = (speed / MAX_DRONE_SPEED).clamp(0.0, 1.0);
    let target_fov =
        (FPV_BASE_FOV_DEG + speed_fraction * FPV_FOV_INCREASE_DEG).to_radians();
    follow.fov.update(target_fov, FPV_FOV_HALF_LIFE, dt);
    if let Projection::Perspective(ref mut persp) = *projection {
        persp.fov = follow.fov.value;
    }
}
