pub mod cleanup;
pub mod detection;
pub mod execution;
pub mod profiles;
pub mod trigger;

use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManeuverKind {
    SplitS,
    PowerLoop,
    AggressiveBank,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManeuverPhaseTag {
    Entry,
    Ballistic,
    Recovery,
}

#[derive(Component)]
#[allow(dead_code)] // entry_position / entry_altitude read in Phase 5 (debug)
pub struct ActiveManeuver {
    pub kind: ManeuverKind,
    pub phase: ManeuverPhaseTag,
    /// 0.0 → 1.0 within current phase
    pub phase_progress: f32,
    pub phase_start_time: f32,
    pub phase_duration: f32,
    pub entry_velocity: Vec3,
    pub entry_position: Vec3,
    /// Where to resume normal flight on the spline
    pub exit_spline_t: f32,
    pub entry_yaw_dir: Vec3,
    pub entry_altitude: f32,
}

/// Lighter alternative for aggressive banking: PID still active, just with a raised tilt limit.
#[derive(Component)]
pub struct TiltOverride {
    pub max_tilt: f32,
    pub exit_spline_t: f32,
}

/// Tracks the exit spline_t of the most recently completed maneuver for cooldown.
/// Prevents immediate re-triggering after a maneuver finishes.
#[derive(Component)]
pub struct ManeuverCooldown {
    pub exit_t: f32,
}
