use bevy::math::cubic_splines::CubicCurve;
use bevy::prelude::*;

/// Spline control points per gate: approach + departure (no center).
/// The spline naturally curves through the gate region between them.
pub const POINTS_PER_GATE: f32 = 2.0;

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
    /// Which gate the drone is heading toward (0-indexed).
    pub target_gate_index: u32,
    /// Total number of gates in the course.
    pub gate_count: u32,
    /// Catmull-Rom spline through gate centers (cyclic). Parameter t in [0, gate_count].
    pub spline: CubicCurve<Vec3>,
    /// Continuous progress along the spline. Race complete when >= gate_count.
    pub spline_t: f32,
    /// Gate centers in order, for fallback distance checks.
    pub gate_positions: Vec<Vec3>,
    /// World-space forward direction for each gate (expected approach direction).
    pub gate_forwards: Vec<Vec3>,
}

/// Bridge between AI and PID: AI writes the desired position, PID reads it.
#[derive(Component)]
pub struct DesiredPosition {
    pub position: Vec3,
    pub velocity_hint: Vec3,
    /// Curvature-aware speed limit (m/s). Set by AI based on upcoming turn tightness.
    pub max_speed: f32,
}

/// Records the spawn position so drones can be reset on race restart.
#[derive(Component)]
pub struct DroneStartPosition {
    pub translation: Vec3,
    pub rotation: Quat,
}

/// Per-drone lifecycle phase, tracking whether the drone is idle, racing, or returning to start.
#[derive(Component, Default, PartialEq, Eq, Clone, Copy, Debug)]
pub enum DronePhase {
    #[default]
    Idle,
    Racing,
    Returning,
}

/// A one-way spline path for a drone flying back to its start position after finishing the race.
#[derive(Component)]
pub struct ReturnPath {
    pub spline: CubicCurve<Vec3>,
    pub spline_t: f32,
    pub total_t: f32,
}

/// Runtime-tunable AI and physics parameters. Exposed via the dev dashboard (F4).
/// Persists across race restarts so tweaked values carry over.
#[derive(Resource)]
pub struct AiTuningParams {
    pub safe_lateral_accel: f32,
    pub curvature_look_ahead_scale: f32,
    pub min_look_ahead_fraction: f32,
    pub min_curvature_speed: f32,
    pub min_advance_speed_fraction: f32,
    pub speed_curvature_range: f32,
    pub look_ahead_t: f32,
    pub max_speed: f32,
    pub max_tilt_angle: f32,
}

impl Default for AiTuningParams {
    fn default() -> Self {
        Self {
            safe_lateral_accel: 25.0,
            curvature_look_ahead_scale: 30.0,
            min_look_ahead_fraction: 0.33,
            min_curvature_speed: 8.0,
            min_advance_speed_fraction: 0.25,
            speed_curvature_range: 2.0,
            look_ahead_t: 0.3,
            max_speed: 45.0,
            max_tilt_angle: 1.3,
        }
    }
}

/// Metadata for each tunable parameter: display name, step size, min, max.
pub struct ParamMeta {
    pub name: &'static str,
    pub step: f32,
    pub min: f32,
    pub max: f32,
}

/// Ordered list of parameter metadata matching `AiTuningParams` field order.
pub const PARAM_META: [ParamMeta; 9] = [
    ParamMeta { name: "Lateral Accel",   step: 1.0,  min: 5.0,   max: 60.0 },
    ParamMeta { name: "Curv Look Scale", step: 2.0,  min: 5.0,   max: 80.0 },
    ParamMeta { name: "Min Look Frac",   step: 0.05, min: 0.1,   max: 1.0 },
    ParamMeta { name: "Min Curv Speed",  step: 1.0,  min: 2.0,   max: 30.0 },
    ParamMeta { name: "Min Advance",     step: 0.05, min: 0.05,  max: 1.0 },
    ParamMeta { name: "Speed Curv Range",step: 0.25, min: 0.5,   max: 5.0 },
    ParamMeta { name: "Look Ahead T",    step: 0.05, min: 0.05,  max: 1.0 },
    ParamMeta { name: "Max Speed",       step: 1.0,  min: 10.0,  max: 80.0 },
    ParamMeta { name: "Max Tilt Angle",  step: 0.05, min: 0.5,   max: 1.57 },
];

impl AiTuningParams {
    /// Get the value of the i-th parameter (field order matches PARAM_META).
    pub fn get(&self, index: usize) -> f32 {
        match index {
            0 => self.safe_lateral_accel,
            1 => self.curvature_look_ahead_scale,
            2 => self.min_look_ahead_fraction,
            3 => self.min_curvature_speed,
            4 => self.min_advance_speed_fraction,
            5 => self.speed_curvature_range,
            6 => self.look_ahead_t,
            7 => self.max_speed,
            8 => self.max_tilt_angle,
            _ => 0.0,
        }
    }

    /// Set the i-th parameter, clamping to its valid range.
    pub fn set(&mut self, index: usize, value: f32) {
        let meta = &PARAM_META[index];
        let clamped = value.clamp(meta.min, meta.max);
        match index {
            0 => self.safe_lateral_accel = clamped,
            1 => self.curvature_look_ahead_scale = clamped,
            2 => self.min_look_ahead_fraction = clamped,
            3 => self.min_curvature_speed = clamped,
            4 => self.min_advance_speed_fraction = clamped,
            5 => self.speed_curvature_range = clamped,
            6 => self.look_ahead_t = clamped,
            7 => self.max_speed = clamped,
            8 => self.max_tilt_angle = clamped,
            _ => {}
        }
    }
}
