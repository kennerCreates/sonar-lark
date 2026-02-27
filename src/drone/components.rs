use bevy::math::cubic_splines::CubicCurve;
use bevy::prelude::*;

/// Spline control points per gate: approach, departure, midleg-to-next.
/// The midleg waypoint between consecutive gates spreads the turn across
/// two segments, reducing peak curvature for higher cornering speed.
pub const POINTS_PER_GATE: f32 = 3.0;

#[derive(Component)]
pub struct Drone {
    pub index: u8,
}

/// Per-drone identity info, set during spawn from SelectedPilots.
/// Decouples drone identity from array-index lookups into DRONE_NAMES/DRONE_COLORS.
#[derive(Component)]
#[allow(dead_code)]
pub struct DroneIdentity {
    pub name: String,
    pub color: Color,
}

/// Outer-loop position PID: position error → desired acceleration.
/// Uses derivative-on-measurement (-velocity) instead of d(error)/dt to avoid derivative kick.
#[derive(Component)]
pub struct PositionPid {
    pub kp: Vec3,
    pub ki: Vec3,
    pub kd: Vec3,
    pub integral: Vec3,
}

impl Default for PositionPid {
    fn default() -> Self {
        Self {
            kp: Vec3::new(6.0, 8.0, 6.0),
            ki: Vec3::new(0.1, 0.2, 0.1),
            kd: Vec3::new(4.0, 5.0, 4.0),
            integral: Vec3::ZERO,
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
        // Gains tuned for slightly underdamped response at ~64 Hz fixed timestep with
        // moment_of_inertia (0.003, 0.005, 0.003). The lower kd allows a single
        // overshoot on aggressive attitude changes, producing visible settle wobble
        // that reads as authentic flight controller behavior.
        Self {
            kp_roll_pitch: 7.0,
            kd_roll_pitch: 0.20,
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
            max_thrust: 75.0,
            mass: 0.8,
            drag_constant: 0.025,
            moment_of_inertia: Vec3::new(0.003, 0.005, 0.003),
            motor_time_constant: 0.025,
        }
    }
}

#[derive(Component, Clone)]
pub struct DroneConfig {
    pub pid_variation: Vec3,
    pub line_offset: f32,
    pub noise_amplitude: f32,
    pub noise_frequency: f32,
    pub hover_noise_amp: Vec3,
    pub hover_noise_freq: Vec3,
    /// Multiplier on safe_lateral_accel: >1.0 = aggressive cornering, <1.0 = cautious.
    pub cornering_aggression: f32,
    /// Multiplier on speed_curvature_range: >1.0 = brakes earlier, <1.0 = brakes later.
    pub braking_distance: f32,
    /// Per-drone multiplier on attitude PD kp_roll_pitch.
    pub attitude_kp_mult: f32,
    /// Per-drone multiplier on attitude PD kd_roll_pitch.
    pub attitude_kd_mult: f32,
    /// Lateral shift magnitude (meters) for midleg waypoints in the per-drone spline.
    /// Positive = right bias. Correlated with cornering_aggression.
    pub racing_line_bias: f32,
    /// Multiplier on approach/departure offset distance.
    /// <1.0 = shorter approach (commits later, sharper near gate).
    /// >1.0 = longer approach (smoother, more committed).
    pub approach_offset_scale: f32,
    /// Fraction (0–1) of each gate's half-extents used for per-drone pass-through offset.
    /// Each gate gets a deterministic 2D offset (width + height) within ±(fraction × half_extent),
    /// so drones spread across the gate opening instead of all flying through center.
    pub gate_pass_offset: f32,
}

#[derive(Component)]
pub struct AIController {
    /// Which gate the drone is heading toward (0-indexed).
    pub target_gate_index: u32,
    /// Total number of gates in the course.
    pub gate_count: u32,
    /// Cyclic Catmull-Rom spline with 3 control points per gate (approach, departure, midleg).
    /// Parameter t in [0, gate_count * POINTS_PER_GATE].
    pub spline: CubicCurve<Vec3>,
    /// Continuous progress along the spline. Race complete when >= gate_count * POINTS_PER_GATE + FINISH_EXTENSION.
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
}

/// Random seed generated fresh each race, mixed into deterministic hashes so that
/// race outcomes vary between runs.
#[derive(Resource)]
pub struct RaceSeed(pub u32);

/// Per-drone lifecycle phase.
#[derive(Component, Default, PartialEq, Eq, Clone, Copy, Debug)]
pub enum DronePhase {
    #[default]
    Idle,
    Racing,
    /// Drone has finished the race and is continuing to lap the course.
    VictoryLap,
    /// Drone wanders freely in the Results screen.
    Wandering,
    Crashed,
}

/// Wandering state: drone picks random waypoints within the course bounding box.
#[derive(Component)]
pub struct WanderState {
    pub target: Vec3,
    pub dwell_timer: f32,
    pub step: u32,
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
    pub battery_sag_factor: f32,
    pub dirty_air_strength: f32,
    pub avoidance_radius: f32,
    pub avoidance_strength: f32,
    pub feedforward_blend: f32,
}

impl Default for AiTuningParams {
    fn default() -> Self {
        Self {
            safe_lateral_accel: 50.0,
            curvature_look_ahead_scale: 30.0,
            min_look_ahead_fraction: 0.33,
            min_curvature_speed: 14.0,
            min_advance_speed_fraction: 0.25,
            speed_curvature_range: 1.25,
            look_ahead_t: 0.3,
            max_speed: 55.0,
            max_tilt_angle: 1.45,
            battery_sag_factor: 0.15,
            dirty_air_strength: 0.0,
            avoidance_radius: 8.0,
            avoidance_strength: 0.0,
            feedforward_blend: 0.85,
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
pub const PARAM_META: [ParamMeta; 14] = [
    ParamMeta { name: "Lateral Accel",   step: 2.0,  min: 5.0,   max: 100.0 },
    ParamMeta { name: "Curv Look Scale", step: 2.0,  min: 5.0,   max: 80.0 },
    ParamMeta { name: "Min Look Frac",   step: 0.05, min: 0.1,   max: 1.0 },
    ParamMeta { name: "Min Curv Speed",  step: 1.0,  min: 2.0,   max: 40.0 },
    ParamMeta { name: "Min Advance",     step: 0.05, min: 0.05,  max: 1.0 },
    ParamMeta { name: "Speed Curv Range",step: 0.25, min: 0.25,  max: 5.0 },
    ParamMeta { name: "Look Ahead T",    step: 0.05, min: 0.05,  max: 1.0 },
    ParamMeta { name: "Max Speed",       step: 1.0,  min: 10.0,  max: 100.0 },
    ParamMeta { name: "Max Tilt Angle",  step: 0.05, min: 0.5,   max: 1.57 },
    ParamMeta { name: "Battery Sag",     step: 0.05, min: 0.0,   max: 0.4 },
    ParamMeta { name: "Dirty Air Str",   step: 1.0,  min: 0.0,   max: 20.0 },
    ParamMeta { name: "Avoid Radius",    step: 1.0,  min: 2.0,   max: 15.0 },
    ParamMeta { name: "Avoid Strength",  step: 1.0,  min: 0.0,   max: 30.0 },
    ParamMeta { name: "FF Blend",        step: 0.05, min: 0.0,   max: 1.0 },
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
            9 => self.battery_sag_factor,
            10 => self.dirty_air_strength,
            11 => self.avoidance_radius,
            12 => self.avoidance_strength,
            13 => self.feedforward_blend,
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
            9 => self.battery_sag_factor = clamped,
            10 => self.dirty_air_strength = clamped,
            11 => self.avoidance_radius = clamped,
            12 => self.avoidance_strength = clamped,
            13 => self.feedforward_blend = clamped,
            _ => {}
        }
    }
}
