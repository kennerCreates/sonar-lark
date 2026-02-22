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

#[derive(Component)]
pub struct DroneDynamics {
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
    pub thrust: f32,
    pub max_thrust: f32,
    pub mass: f32,
    pub drag_coefficient: f32,
}

#[derive(Component)]
pub struct DroneConfig {
    pub pid_variation: Vec3,
    pub line_offset: f32,
    pub noise_amplitude: f32,
    pub noise_frequency: f32,
}

#[derive(Component)]
pub struct AIController {
    pub target_gate_index: u32,
    pub waypoints: Vec<Vec3>,
    pub current_waypoint: usize,
}
