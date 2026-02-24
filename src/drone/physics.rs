use bevy::prelude::*;

use crate::race::timing::RaceClock;
use super::components::*;

const GRAVITY: f32 = 9.81;
/// Approximate full-pack race duration for battery sag curve (seconds).
const RACE_DURATION_ESTIMATE: f32 = 90.0;
const GROUND_HEIGHT: f32 = 0.3;
const INTEGRAL_CLAMP: f32 = 10.0;

/// Noise-driven hover target. Layered sine waves produce organic 1–3 cm drift.
pub fn hover_target(
    time: Res<Time>,
    tuning: Res<AiTuningParams>,
    mut query: Query<(
        &Drone,
        &DroneStartPosition,
        &DroneConfig,
        &DronePhase,
        &mut DesiredPosition,
    )>,
) {
    let t = time.elapsed_secs();

    for (drone, start, config, phase, mut desired) in &mut query {
        if *phase != DronePhase::Idle {
            continue;
        }
        let phase = drone.index as f32 * 1.7;

        let noise = Vec3::new(
            (t * config.hover_noise_freq.x + phase).sin() * config.hover_noise_amp.x
                + (t * config.hover_noise_freq.x * 2.3 + phase * 0.7).sin()
                    * config.hover_noise_amp.x
                    * 0.3,
            (t * config.hover_noise_freq.y + phase * 1.3).sin() * config.hover_noise_amp.y,
            (t * config.hover_noise_freq.z + phase * 0.9).sin() * config.hover_noise_amp.z
                + (t * config.hover_noise_freq.z * 1.7 + phase * 1.1).sin()
                    * config.hover_noise_amp.z
                    * 0.3,
        );

        desired.position = start.translation + noise;
        desired.velocity_hint = Vec3::ZERO;
        desired.max_speed = tuning.max_speed;
    }
}

/// Outer-loop position PID. Computes desired acceleration from position error,
/// then maps that to a desired body orientation and thrust magnitude.
pub fn position_pid(
    time: Res<Time>,
    tuning: Res<AiTuningParams>,
    mut query: Query<(
        &Transform,
        &DesiredPosition,
        &mut PositionPid,
        &DroneDynamics,
        &mut DesiredAttitude,
    )>,
) {
    let dt = time.delta_secs();
    if dt == 0.0 {
        return;
    }

    for (transform, desired, mut pid, dynamics, mut attitude) in &mut query {
        let error = desired.position - transform.translation;

        pid.integral = (pid.integral + error * dt).clamp(
            Vec3::splat(-INTEGRAL_CLAMP),
            Vec3::splat(INTEGRAL_CLAMP),
        );

        // Blended derivative: mix absolute damping with velocity feedforward.
        // At ff_blend=0: pure damping (-velocity), for position hold.
        // At ff_blend=1: velocity-error damping (desired_vel - velocity),
        // which stops fighting motion when the drone is at target speed.
        let desired_velocity = desired.velocity_hint * desired.max_speed;
        let derivative = -dynamics.velocity + desired_velocity * tuning.feedforward_blend;

        let pid_output = pid.kp * error + pid.ki * pid.integral + pid.kd * derivative;

        // Desired acceleration = PID + gravity compensation
        let desired_accel = pid_output + Vec3::Y * GRAVITY;

        // Desired body-up direction: aligned with desired acceleration
        let desired_up = desired_accel.normalize_or(Vec3::Y);

        // Clamp tilt angle (read from tuning resource)
        let max_tilt = tuning.max_tilt_angle;
        let tilt_angle = desired_up.angle_between(Vec3::Y);
        let clamped_up = if tilt_angle > max_tilt {
            let tilt_axis = Vec3::Y.cross(desired_up).normalize_or(Vec3::X);
            (Quat::from_axis_angle(tilt_axis, max_tilt) * Vec3::Y).normalize()
        } else {
            desired_up
        };

        // Thrust magnitude: sized so the vertical component matches desired_accel.y.
        // This prevents excess vertical force when tilt is clamped.
        // (When unclamped this reduces to |desired_accel| * mass.)
        let cos_tilt = clamped_up.y.max(0.05);
        attitude.thrust_magnitude =
            (desired_accel.y * dynamics.mass / cos_tilt).clamp(0.0, dynamics.max_thrust);

        // Yaw: face movement direction from velocity_hint, or keep current heading
        let yaw_dir = if desired.velocity_hint.length_squared() > 0.01 {
            let flat = Vec3::new(desired.velocity_hint.x, 0.0, desired.velocity_hint.z);
            flat.normalize_or(Vec3::NEG_Z)
        } else {
            let fwd = transform.rotation * Vec3::NEG_Z;
            Vec3::new(fwd.x, 0.0, fwd.z).normalize_or(Vec3::NEG_Z)
        };

        // Build orientation: combine tilt (body-up direction) with yaw (heading)
        let right = yaw_dir.cross(clamped_up).normalize_or(Vec3::X);
        let corrected_fwd = clamped_up.cross(right).normalize();
        attitude.orientation =
            Quat::from_mat3(&Mat3::from_cols(right, clamped_up, -corrected_fwd)).normalize();
    }
}

/// Inner-loop attitude PD controller. Computes torques from orientation error
/// and integrates angular velocity and orientation.
pub fn attitude_controller(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut DroneDynamics, &DesiredAttitude, &AttitudePd)>,
) {
    let dt = time.delta_secs();
    if dt == 0.0 {
        return;
    }

    for (mut transform, mut dynamics, attitude, pd) in &mut query {
        // Orientation error: rotation from current to desired
        let error_quat = attitude.orientation * transform.rotation.inverse();

        // Convert to axis-angle, taking the short path
        let (axis, mut angle) = error_quat.to_axis_angle();
        if angle > std::f32::consts::PI {
            angle = -(2.0 * std::f32::consts::PI - angle);
        }

        let error_vec = if angle.abs() > 0.001 {
            axis * angle
        } else {
            Vec3::ZERO
        };

        // PD torque: split roll/pitch (x,z) from yaw (y)
        let torque = Vec3::new(
            error_vec.x * pd.kp_roll_pitch - dynamics.angular_velocity.x * pd.kd_roll_pitch,
            error_vec.y * pd.kp_yaw - dynamics.angular_velocity.y * pd.kd_yaw,
            error_vec.z * pd.kp_roll_pitch - dynamics.angular_velocity.z * pd.kd_roll_pitch,
        );

        let angular_accel = torque / dynamics.moment_of_inertia;
        dynamics.angular_velocity += angular_accel * dt;

        // Clamp angular rates
        dynamics.angular_velocity = dynamics
            .angular_velocity
            .clamp(-pd.max_angular_rate, pd.max_angular_rate);

        // Integrate orientation via quaternion rotation
        let omega_mag = dynamics.angular_velocity.length();
        if omega_mag > 0.0001 {
            let delta_q =
                Quat::from_axis_angle(dynamics.angular_velocity / omega_mag, omega_mag * dt);
            transform.rotation = (delta_q * transform.rotation).normalize();
        }
    }
}

const DIRTY_AIR_RADIUS: f32 = 5.0;
const WAKE_CONE_COS: f32 = 0.707; // cos(45°) half-angle
const PROP_WASH_TORQUE: f32 = 5.0;
const PROP_WASH_ONSET: f32 = 2.0; // descent speed (m/s) before prop wash kicks in

/// Faked aerodynamic perturbations: dirty air from leading drones and prop wash on descent.
/// Runs after attitude_controller so perturbations fight against the PD (producing visible wobble)
/// but before motor_lag so the motors can partially react.
pub fn dirty_air_perturbation(
    time: Res<Time>,
    tuning: Res<AiTuningParams>,
    race_seed: Option<Res<RaceSeed>>,
    mut query: Query<(&Transform, &Drone, &mut DroneDynamics)>,
) {
    let dt = time.delta_secs();
    if dt == 0.0 || tuning.dirty_air_strength == 0.0 {
        return;
    }
    let t = time.elapsed_secs();
    let seed_phase = race_seed.map_or(0.0, |s| (s.0 & 0xFFFF) as f32 / 65536.0 * 100.0);

    // Collect positions and velocities (12 drones = tiny allocation)
    let drone_data: Vec<(u8, Vec3, Vec3, f32)> = query
        .iter()
        .map(|(tr, drone, dyn_)| (drone.index, tr.translation, dyn_.velocity, dyn_.velocity.length()))
        .collect();

    for (transform, drone, mut dynamics) in &mut query {
        let my_pos = transform.translation;
        let my_idx = drone.index;

        // --- Dirty air: perturbation from flying in another drone's wake ---
        for &(other_idx, other_pos, other_vel, other_speed) in &drone_data {
            if other_idx == my_idx || other_speed < 1.0 {
                continue;
            }
            let to_me = my_pos - other_pos;
            let dist = to_me.length();
            if dist > DIRTY_AIR_RADIUS || dist < 0.1 {
                continue;
            }

            // Am I behind the other drone (in their velocity direction)?
            let other_vel_dir = other_vel / other_speed;
            let behind_dot = (-to_me / dist).dot(other_vel_dir);
            if behind_dot < WAKE_CONE_COS {
                continue;
            }

            let strength = (1.0 - dist / DIRTY_AIR_RADIUS) * (other_speed / tuning.max_speed);

            // Deterministic pseudo-random perturbation using layered sin waves
            let phase = my_idx as f32 * 2.71 + other_idx as f32 * 1.37 + seed_phase;
            let perturbation = Vec3::new(
                (t * 17.3 + phase).sin() + (t * 31.7 + phase * 2.1).sin() * 0.5,
                (t * 13.1 + phase * 1.5).sin() * 0.3,
                (t * 23.7 + phase * 0.8).sin() + (t * 41.3 + phase * 1.7).sin() * 0.5,
            );

            dynamics.angular_velocity += perturbation * tuning.dirty_air_strength * strength * dt;
        }

        // --- Prop wash: perturbation when descending through own downwash ---
        let descent_rate = (-dynamics.velocity.y).max(0.0);
        if descent_rate > PROP_WASH_ONSET {
            let wash_strength = ((descent_rate - PROP_WASH_ONSET) / 10.0).min(1.0);
            let phase = my_idx as f32 * 3.14 + 100.0 + seed_phase;
            let wash_noise = Vec3::new(
                (t * 19.7 + phase).sin() + (t * 37.3 + phase * 1.3).sin() * 0.6,
                (t * 11.3 + phase * 2.1).sin() * 0.2,
                (t * 29.1 + phase * 0.7).sin() + (t * 43.9 + phase * 1.9).sin() * 0.6,
            );
            dynamics.angular_velocity += wash_noise * PROP_WASH_TORQUE * wash_strength * dt;
        }
    }
}

/// First-order low-pass filter on thrust, simulating motor spool-up lag.
pub fn motor_lag(time: Res<Time>, mut query: Query<(&mut DroneDynamics, &DesiredAttitude)>) {
    let dt = time.delta_secs();
    if dt == 0.0 {
        return;
    }

    for (mut dynamics, attitude) in &mut query {
        dynamics.commanded_thrust = attitude.thrust_magnitude;
        let alpha = (dt / dynamics.motor_time_constant).min(1.0);
        dynamics.thrust += (dynamics.commanded_thrust - dynamics.thrust) * alpha;
        dynamics.thrust = dynamics.thrust.clamp(0.0, dynamics.max_thrust);
    }
}

/// Applies thrust (along body-up), gravity, and quadratic drag to update velocity.
/// Uses per-drone curvature-aware speed limit from `DesiredPosition.max_speed`,
/// capped by the global `AiTuningParams.max_speed`.
/// Battery sag gradually reduces effective thrust over the race duration.
pub fn apply_forces(
    time: Res<Time>,
    tuning: Res<AiTuningParams>,
    race_clock: Option<Res<RaceClock>>,
    mut query: Query<(&Transform, &mut DroneDynamics, &DesiredPosition), With<Drone>>,
) {
    let dt = time.delta_secs();
    if dt == 0.0 {
        return;
    }

    // Battery sag: linear thrust reduction over race duration
    let sag_mult = if let Some(ref clock) = race_clock {
        if clock.running {
            let progress = (clock.elapsed / RACE_DURATION_ESTIMATE).min(1.0);
            1.0 - tuning.battery_sag_factor * progress
        } else {
            1.0
        }
    } else {
        1.0
    };

    for (transform, mut dynamics, desired) in &mut query {
        // Thrust always along body-up axis, reduced by battery sag
        let body_up = transform.rotation * Vec3::Y;
        let thrust_force = body_up * dynamics.thrust * sag_mult;

        let gravity_force = Vec3::NEG_Y * GRAVITY * dynamics.mass;

        // Quadratic drag: F = -k * |v| * v
        let speed = dynamics.velocity.length();
        let drag_force = if speed > 0.001 {
            -dynamics.drag_constant * speed * dynamics.velocity
        } else {
            Vec3::ZERO
        };

        let net_force = thrust_force + gravity_force + drag_force;
        let acceleration = net_force / dynamics.mass;

        dynamics.velocity += acceleration * dt;

        // Per-drone speed limit (curvature-aware), capped by global max_speed.
        // Soft clamping: exponential decay toward the limit instead of instant snap.
        // At rate 10.0, ~14.5% of overspeed removed per tick (64Hz), 99.5% within 0.5s.
        const SPEED_CLAMP_RATE: f32 = 10.0;
        let effective_max = desired.max_speed.min(tuning.max_speed);
        let new_speed = dynamics.velocity.length();
        if new_speed > effective_max {
            let alpha = (1.0 - (-SPEED_CLAMP_RATE * dt).exp()).min(1.0);
            let target_speed = new_speed * (1.0 - alpha) + effective_max * alpha;
            dynamics.velocity *= target_speed / new_speed;
        }
    }
}

/// Position integration from velocity.
pub fn integrate_motion(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &DroneDynamics), With<Drone>>,
) {
    let dt = time.delta_secs();
    if dt == 0.0 {
        return;
    }

    for (mut transform, dynamics) in &mut query {
        transform.translation += dynamics.velocity * dt;
    }
}

/// Prevents drones from going below ground.
pub fn clamp_transform(mut query: Query<(&mut Transform, &mut DroneDynamics), With<Drone>>) {
    for (mut transform, mut dynamics) in &mut query {
        if transform.translation.y < GROUND_HEIGHT {
            transform.translation.y = GROUND_HEIGHT;
            if dynamics.velocity.y < 0.0 {
                dynamics.velocity.y = 0.0;
            }
        }
    }
}
