use bevy::math::cubic_splines::CubicCurve;
use bevy::prelude::*;

pub const GRAVITY: f32 = 9.81;

#[derive(Component)]
pub struct Drone {
    pub index: u8,
}

/// Per-drone identity info, set during spawn from SelectedPilots.
/// Decouples drone identity from array-index lookups into DRONE_NAMES/DRONE_COLORS.
#[derive(Component)]
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

/// Bridge between position PID and attitude mapping.
/// Separates "where to accelerate" from "how to orient for that acceleration."
#[derive(Component)]
pub struct DesiredAcceleration {
    pub acceleration: Vec3,
}

/// Per-entity tilt angle limit (radians from vertical).
/// Synced from AiTuningParams.max_tilt_angle during normal flight.
/// Maneuver systems will override this for individual drones.
#[derive(Component)]
pub struct TiltClamp {
    pub max_angle: f32,
}

/// Bridge between attitude mapping and attitude controller (inner loop).
#[derive(Component)]
pub struct DesiredAttitude {
    pub orientation: Quat,
    pub thrust_magnitude: f32,
}

/// Body-rate control override for acrobatic maneuvers.
/// When present, `attitude_controller` switches from orientation tracking to rate
/// tracking, and `motor_lag` reads thrust from here instead of `DesiredAttitude`.
/// Inserted by maneuver systems (Phase 6), removed when maneuver completes.
#[derive(Component)]
pub struct DesiredBodyRates {
    /// Target angular velocity in world frame (rad/s).
    pub angular_velocity: Vec3,
    /// Target thrust magnitude (N).
    pub thrust: f32,
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

/// Per-entity choreography tracking, inserted when DronePhase transitions to Racing.
#[derive(Component)]
pub struct ChoreographyState {
    /// spline_t from the previous tick — needed by fire_scripted_events
    /// for gate crossing detection (previous_t < threshold <= current_t).
    pub previous_spline_t: f32,
    /// Pilot consistency (0..1), cached at race start for jitter scaling.
    pub consistency: f32,
}

/// Ballistic arc state for crashed drones (inserted on crash, removed on ground impact).
#[derive(Component)]
pub struct BallisticState {
    pub velocity: Vec3,
}

/// Random seed generated fresh each race, mixed into deterministic hashes so that
/// race outcomes vary between runs.
#[derive(Resource)]
pub struct RaceSeed(pub u32);

pub use crate::common::race_participant::DronePhase;

/// Wandering state: drone picks random waypoints within the course bounding box.
#[derive(Component)]
pub struct WanderState {
    pub target: Vec3,
    pub dwell_timer: f32,
    pub step: u32,
}

/// Metadata for each tunable parameter: display name, step size, min, max.
pub struct ParamMeta {
    pub name: &'static str,
    pub step: f32,
    pub min: f32,
    pub max: f32,
}

/// Generates `AiTuningParams` struct, its `Default` impl, and indexed `get`/`set`
/// from a single field list. Adding a new tunable parameter is a single line.
macro_rules! tuning_params {
    ($( $field:ident, $name:literal, $step:literal, $min:literal, $max:literal, $default:literal );+ $(;)?) => {
        /// Runtime-tunable AI and physics parameters. Exposed via the dev dashboard (F4).
        /// Persists across race restarts so tweaked values carry over.
        #[derive(Resource)]
        pub struct AiTuningParams {
            $(pub $field: f32,)+
        }

        impl Default for AiTuningParams {
            fn default() -> Self {
                Self { $($field: $default,)+ }
            }
        }

        impl AiTuningParams {
            pub const PARAM_META: &[ParamMeta] = &[
                $(ParamMeta { name: $name, step: $step, min: $min, max: $max },)+
            ];

            /// Get the value of the i-th parameter (field order matches PARAM_META).
            pub fn get(&self, index: usize) -> f32 {
                let values = [$(self.$field,)+];
                values.get(index).copied().unwrap_or(0.0)
            }

            /// Set the i-th parameter, clamping to its valid range.
            #[allow(unused_assignments)]
            pub fn set(&mut self, index: usize, value: f32) {
                if let Some(meta) = Self::PARAM_META.get(index) {
                    let clamped = value.clamp(meta.min, meta.max);
                    let mut i = 0usize;
                    $(
                        if index == i { self.$field = clamped; return; }
                        i += 1;
                    )+
                }
            }
        }
    };
}

tuning_params! {
    safe_lateral_accel,         "Lateral Accel",    2.0,  5.0,   100.0, 32.0;
    curvature_look_ahead_scale, "Curv Look Scale",  2.0,  5.0,   80.0,  30.0;
    min_look_ahead_fraction,    "Min Look Frac",    0.05, 0.1,   1.0,   0.33;
    min_curvature_speed,        "Min Curv Speed",   1.0,  2.0,   40.0,  10.0;
    min_advance_speed_fraction, "Min Advance",      0.05, 0.05,  1.0,   0.25;
    speed_curvature_range,      "Speed Curv Range", 0.25, 0.25,  5.0,   1.25;
    look_ahead_t,               "Look Ahead T",     0.05, 0.05,  1.0,   0.3;
    max_speed,                  "Max Speed",        1.0,  10.0,  100.0, 38.0;
    max_tilt_angle,             "Max Tilt Angle",   0.05, 0.5,   1.57,  1.1;
    battery_sag_factor,         "Battery Sag",      0.05, 0.0,   0.4,   0.15;
    dirty_air_strength,         "Dirty Air Str",    1.0,  0.0,   20.0,  0.0;
    avoidance_radius,           "Avoid Radius",     1.0,  2.0,   15.0,  8.0;
    avoidance_strength,         "Avoid Strength",   1.0,  0.0,   30.0,  0.0;
    feedforward_blend,          "FF Blend",         0.05, 0.0,   1.0,   0.85;
}
