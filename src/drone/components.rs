use bevy::prelude::*;

#[derive(Component)]
pub struct Drone {
    pub index: u8,
}

#[derive(Component)]
pub struct PidController {
    pub kp: Vec3,
    pub ki: Vec3,
    pub kd: Vec3,
    pub integral: Vec3,
    pub prev_error: Vec3,
}

impl Default for PidController {
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

#[derive(Component)]
pub struct DroneDynamics {
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
    pub thrust: f32,
    pub thrust_direction: Vec3,
    pub max_thrust: f32,
    pub mass: f32,
    pub drag_coefficient: f32,
}

impl Default for DroneDynamics {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            thrust: 0.0,
            thrust_direction: Vec3::Y,
            max_thrust: 40.0,
            mass: 0.8,
            drag_coefficient: 0.3,
        }
    }
}

#[derive(Component)]
pub struct DroneConfig {
    pub pid_variation: Vec3,
    pub line_offset: f32,
    pub noise_amplitude: f32,
    pub noise_frequency: f32,
    /// Per-axis phase offsets for hover animation (radians).
    pub hover_phase: Vec3,
    /// Per-axis primary oscillation frequencies (Hz) for idle hover.
    pub hover_freq: Vec3,
    /// Per-axis amplitude of idle hover movement (units). X/Z may be negative
    /// to vary drift direction across drones; Y is always positive (drift up).
    pub hover_amp: Vec3,
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
