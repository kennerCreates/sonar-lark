pub mod cleanup;
pub mod detection;
pub mod trajectory;
pub mod trigger;

use bevy::math::cubic_splines::CubicCurve;
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManeuverKind {
    SplitS,
    #[allow(dead_code)] // Not yet produced by detect_maneuver; reserved for future courses
    PowerLoop,
    AggressiveBank,
}

/// Active maneuver trajectory: the PID samples from this curve instead of the
/// racing spline. Replaces the old `ActiveManeuver` open-loop override.
#[derive(Component)]
pub struct ManeuverTrajectory {
    pub kind: ManeuverKind,
    /// 3D path through space (CubicHermite curve).
    pub curve: CubicCurve<Vec3>,
    /// Number of segments in the curve (for parameterization: t ∈ [0, curve_len]).
    pub curve_len: f32,
    /// `elapsed_secs` at activation.
    pub start_time: f32,
    /// Total duration in seconds.
    pub duration: f32,
    /// Where to resume on the racing spline after maneuver completes.
    pub exit_spline_t: f32,
    /// Speed to maintain during maneuver (entry speed at activation).
    pub entry_speed: f32,
}

/// Lighter alternative for aggressive banking: PID still active, just with a raised tilt limit.
#[derive(Component)]
pub struct TiltOverride {
    pub max_tilt: f32,
    pub exit_spline_t: f32,
}

/// Deferred maneuver: detected a turn ahead, waiting until the drone reaches
/// `trigger_t` before converting to `ManeuverTrajectory` or `TiltOverride`.
#[derive(Component)]
pub struct PendingManeuver {
    pub kind: ManeuverKind,
    /// Spline parameter at which the maneuver should activate.
    pub trigger_t: f32,
    pub exit_t: f32,
}

/// Tracks the exit spline_t of the most recently completed maneuver for cooldown.
/// Prevents immediate re-triggering after a maneuver finishes.
#[derive(Component)]
pub struct ManeuverCooldown {
    pub exit_t: f32,
}
