use bevy::prelude::*;
use rand::Rng;

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
    /// Per-axis max offset range for idle hover drift (units, always positive).
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

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum HoverPhase {
    #[default]
    Drifting,
    Snapping,
    Correcting,
}

/// Per-drone state machine for idle hover animation.
/// Cycle: drift to random offset → snap back past home (overshoot) → correct to home → repeat.
#[derive(Component)]
pub struct HoverCycle {
    pub phase: HoverPhase,
    pub timer: f32,
    pub drift_duration: f32,
    pub snap_duration: f32,
    pub correct_duration: f32,
    /// Random offset target for this cycle (world space).
    pub pos_r: Vec3,
    /// Overshoot position past home (world space).
    pub overshoot_pos: Vec3,
}

impl HoverCycle {
    pub fn new(pos_a: Vec3, hover_amp: Vec3, initial_timer: f32) -> Self {
        let mut rng = rand::thread_rng();

        let offset = Vec3::new(
            rng.gen_range(-hover_amp.x.abs()..=hover_amp.x.abs()),
            rng.gen_range(-hover_amp.y.abs()..=hover_amp.y.abs()),
            rng.gen_range(-hover_amp.z.abs()..=hover_amp.z.abs()),
        );
        let pos_r = pos_a + offset;

        let overshoot_fraction = rng.gen_range(0.15f32..=0.50);
        let overshoot_pos = pos_a + (pos_a - pos_r) * overshoot_fraction;

        Self {
            phase: HoverPhase::Drifting,
            timer: initial_timer,
            drift_duration: rng.gen_range(2.0f32..=4.0),
            snap_duration: rng.gen_range(0.3f32..=0.8),
            correct_duration: rng.gen_range(0.5f32..=1.2),
            pos_r,
            overshoot_pos,
        }
    }
}
