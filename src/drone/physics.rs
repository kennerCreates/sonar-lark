use bevy::prelude::*;

use crate::race::lifecycle::RacePhase;
use super::components::*;

const GRAVITY: f32 = 9.81;
const MAX_SPEED: f32 = 30.0;
const GROUND_HEIGHT: f32 = 0.3;
const INTEGRAL_CLAMP: f32 = 10.0;
const MAX_LEAN_ANGLE: f32 = 0.6;
const ROTATION_SMOOTHING: f32 = 8.0;
/// Per-drone idle hover: slow drift away from start position, fast snap back.
/// 80% of each cycle is a gradual drift, 20% is an abrupt return — creating
/// the jockeying / showing-off feel of competitors on a start line.
pub fn hover_bob(
    mut query: Query<(&DroneStartPosition, &mut DesiredPosition)>,
) {
    // drift_snap disabled for debugging — drones hold their spawn positions.
    for (start, mut desired) in &mut query {
        desired.position = start.translation;
    }
}

/// Asymmetric waveform: slow linear drift (0→1) then fast linear snap (1→0).
/// Returns 0.0–1.0. The snap portion is 4× faster than the drift.
fn drift_snap(t: f32, freq: f32, phase: f32) -> f32 {
    const DRIFT: f32 = 0.8;
    let cycle = (t * freq + phase).fract().abs();
    if cycle < DRIFT {
        cycle / DRIFT
    } else {
        (1.0 - cycle) / (1.0 - DRIFT)
    }
}

pub fn pid_compute(
    time: Res<Time>,
    mut query: Query<(
        &Transform,
        &DesiredPosition,
        &mut PidController,
        &mut DroneDynamics,
    )>,
) {
    let dt = time.delta_secs();
    if dt == 0.0 {
        return;
    }

    for (transform, desired, mut pid, mut dynamics) in &mut query {
        let error = desired.position - transform.translation;

        // Anti-windup: clamp integral
        pid.integral = (pid.integral + error * dt).clamp(
            Vec3::splat(-INTEGRAL_CLAMP),
            Vec3::splat(INTEGRAL_CLAMP),
        );

        let derivative = (error - pid.prev_error) / dt;
        pid.prev_error = error;

        let pid_output = pid.kp * error + pid.ki * pid.integral + pid.kd * derivative;

        // Add gravity compensation so hover is the steady state
        let desired_accel = pid_output + Vec3::Y * GRAVITY;

        dynamics.thrust_direction = desired_accel.normalize_or(Vec3::Y);
        dynamics.thrust =
            (desired_accel.length() * dynamics.mass).clamp(0.0, dynamics.max_thrust);
    }
}

pub fn apply_forces(time: Res<Time>, mut query: Query<&mut DroneDynamics>) {
    let dt = time.delta_secs();
    if dt == 0.0 {
        return;
    }

    for mut dynamics in &mut query {
        let thrust_force = dynamics.thrust_direction * dynamics.thrust;
        let gravity_force = Vec3::NEG_Y * GRAVITY * dynamics.mass;
        let drag_force = -dynamics.velocity * dynamics.drag_coefficient;

        let net_force = thrust_force + gravity_force + drag_force;
        let acceleration = net_force / dynamics.mass;

        // Semi-implicit Euler: update velocity first
        dynamics.velocity += acceleration * dt;

        let speed = dynamics.velocity.length();
        if speed > MAX_SPEED {
            dynamics.velocity = dynamics.velocity.normalize() * MAX_SPEED;
        }
    }
}

pub fn integrate_motion(
    time: Res<Time>,
    phase: Option<Res<RacePhase>>,
    mut query: Query<(&mut Transform, &DroneDynamics), With<Drone>>,
) {
    let dt = time.delta_secs();
    if dt == 0.0 {
        return;
    }
    let is_racing = phase.is_some_and(|p| *p == RacePhase::Racing);

    for (mut transform, dynamics) in &mut query {
        // Position integration using updated velocity (semi-implicit Euler)
        transform.translation += dynamics.velocity * dt;

        // Lean/yaw only while racing — during hover drones face forward
        if !is_racing {
            continue;
        }

        let speed = dynamics.velocity.length();
        if speed > 0.5 {
            let move_dir = dynamics.velocity / speed;

            let lean_amount = (speed / MAX_SPEED * MAX_LEAN_ANGLE).min(MAX_LEAN_ANGLE);
            let lean_axis = Vec3::Y.cross(move_dir).normalize_or(Vec3::X);
            let lean_rotation = Quat::from_axis_angle(lean_axis, lean_amount);

            // Yaw: face movement direction on XZ plane
            let flat_dir = Vec3::new(move_dir.x, 0.0, move_dir.z);
            let yaw = if flat_dir.length_squared() > 0.001 {
                Quat::from_rotation_arc(Vec3::NEG_Z, flat_dir.normalize())
            } else {
                transform.rotation
            };

            let target = yaw * lean_rotation;
            transform.rotation = transform
                .rotation
                .slerp(target, (ROTATION_SMOOTHING * dt).min(1.0));
        }
    }
}

pub fn clamp_transform(
    mut query: Query<(&mut Transform, &mut DroneDynamics), With<Drone>>,
) {
    for (mut transform, mut dynamics) in &mut query {
        if transform.translation.y < GROUND_HEIGHT {
            transform.translation.y = GROUND_HEIGHT;
            if dynamics.velocity.y < 0.0 {
                dynamics.velocity.y = 0.0;
            }
        }
    }
}
