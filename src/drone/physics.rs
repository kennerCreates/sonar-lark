use bevy::prelude::*;

use super::components::*;

const GRAVITY: f32 = 9.81;
const MAX_SPEED: f32 = 45.0;
const GROUND_HEIGHT: f32 = 0.3;
const INTEGRAL_CLAMP: f32 = 10.0;
const MAX_TILT_ANGLE: f32 = 1.3;

/// Noise-driven hover target. Layered sine waves produce organic 1–3 cm drift.
pub fn hover_target(
    time: Res<Time>,
    mut query: Query<(
        &Drone,
        &DroneStartPosition,
        &DroneConfig,
        &mut DesiredPosition,
    )>,
) {
    let t = time.elapsed_secs();

    for (drone, start, config, mut desired) in &mut query {
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
    }
}

/// Outer-loop position PID. Computes desired acceleration from position error,
/// then maps that to a desired body orientation and thrust magnitude.
pub fn position_pid(
    time: Res<Time>,
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

        // Derivative-on-measurement: use -velocity instead of d(error)/dt.
        // Avoids derivative kick when the desired position changes abruptly.
        let derivative = -dynamics.velocity;

        let pid_output = pid.kp * error + pid.ki * pid.integral + pid.kd * derivative;

        // Desired acceleration = PID + gravity compensation
        let desired_accel = pid_output + Vec3::Y * GRAVITY;

        // Desired body-up direction: aligned with desired acceleration
        let desired_up = desired_accel.normalize_or(Vec3::Y);

        // Clamp tilt angle
        let tilt_angle = desired_up.angle_between(Vec3::Y);
        let clamped_up = if tilt_angle > MAX_TILT_ANGLE {
            let tilt_axis = Vec3::Y.cross(desired_up).normalize_or(Vec3::X);
            (Quat::from_axis_angle(tilt_axis, MAX_TILT_ANGLE) * Vec3::Y).normalize()
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
pub fn apply_forces(
    time: Res<Time>,
    mut query: Query<(&Transform, &mut DroneDynamics), With<Drone>>,
) {
    let dt = time.delta_secs();
    if dt == 0.0 {
        return;
    }

    for (transform, mut dynamics) in &mut query {
        // Thrust always along body-up axis
        let body_up = transform.rotation * Vec3::Y;
        let thrust_force = body_up * dynamics.thrust;

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

        let new_speed = dynamics.velocity.length();
        if new_speed > MAX_SPEED {
            dynamics.velocity = dynamics.velocity.normalize() * MAX_SPEED;
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
