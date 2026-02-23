use bevy::prelude::*;

#[derive(Component)]
pub struct Drone {
    pub index: u8,
}

/// Outer-loop position PID: position error → desired acceleration.
#[derive(Component)]
pub struct PositionPid {
    pub kp: Vec3,
    pub ki: Vec3,
    pub kd: Vec3,
    pub integral: Vec3,
    pub prev_error: Vec3,
}

impl Default for PositionPid {
    fn default() -> Self {
        Self {
            kp: Vec3::new(6.0, 8.0, 6.0),
            ki: Vec3::new(0.1, 0.2, 0.1),
            kd: Vec3::new(4.0, 5.0, 4.0),
            integral: Vec3::ZERO,
            prev_error: Vec3::ZERO,
        }
    }
}

/// Inner-loop attitude PD: orientation error → torque.
#[derive(Component)]
pub struct AttitudePd {
    pub kp_roll_pitch: f32,
    pub kd_roll_pitch: f32,
    pub kp_yaw: f32,
    pub kd_yaw: f32,
    pub max_angular_rate: Vec3,
}

impl Default for AttitudePd {
    fn default() -> Self {
        // Gains are tuned for discrete stability at ~64 Hz fixed timestep with
        // moment_of_inertia (0.003, 0.005, 0.003).  Rule of thumb: kd·dt/I < 2.
        Self {
            kp_roll_pitch: 5.0,
            kd_roll_pitch: 0.24,
            kp_yaw: 3.0,
            kd_yaw: 0.25,
            max_angular_rate: Vec3::new(20.0, 20.0, 10.0),
        }
    }
}

/// Bridge between position PID (outer loop) and attitude controller (inner loop).
#[derive(Component)]
pub struct DesiredAttitude {
    pub orientation: Quat,
    pub thrust_magnitude: f32,
}

#[derive(Component)]
pub struct DroneDynamics {
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
    pub thrust: f32,
    pub commanded_thrust: f32,
    pub max_thrust: f32,
    pub mass: f32,
    pub drag_constant: f32,
    pub moment_of_inertia: Vec3,
    pub motor_time_constant: f32,
}

impl Default for DroneDynamics {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            thrust: 0.0,
            commanded_thrust: 0.0,
            max_thrust: 55.0,
            mass: 0.8,
            drag_constant: 0.025,
            moment_of_inertia: Vec3::new(0.003, 0.005, 0.003),
            motor_time_constant: 0.040,
        }
    }
}

#[derive(Component)]
pub struct DroneConfig {
    pub pid_variation: Vec3,
    pub line_offset: f32,
    pub noise_amplitude: f32,
    pub noise_frequency: f32,
    pub hover_noise_amp: Vec3,
    pub hover_noise_freq: Vec3,
}

#[derive(Component)]
pub struct AIController {
    pub target_gate_index: u32,
    pub waypoints: Vec<Vec3>,
    pub current_waypoint: usize,
}

/// Bridge between AI and PID: AI writes the desired position, PID reads it.
#[derive(Component)]
pub struct DesiredPosition {
    pub position: Vec3,
    pub velocity_hint: Vec3,
}

/// Records the spawn position so drones can be reset on race restart.
#[derive(Component)]
pub struct DroneStartPosition {
    pub translation: Vec3,
    pub rotation: Quat,
}
