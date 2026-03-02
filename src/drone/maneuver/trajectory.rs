use bevy::math::cubic_splines::{CubicCurve, CubicHermite};
use bevy::prelude::*;

/// Reference speed for duration scaling. Faster drones get shorter maneuvers.
const REF_SPEED: f32 = 30.0;

/// Base duration for a Split-S at reference speed.
const SPLIT_S_BASE_DURATION: f32 = 0.5;

/// Base duration for a Power Loop at reference speed.
const POWER_LOOP_BASE_DURATION: f32 = 0.6;

/// How high above the midpoint the apex rises (fraction of entry-exit distance).
const SPLIT_S_APEX_HEIGHT_FRAC: f32 = 0.35;

/// How high the climb point rises for a power loop (fraction of entry-exit distance).
const POWER_LOOP_CLIMB_FRAC: f32 = 0.5;

/// Tangent magnitude multiplier: scales tangents so the curve matches drone speed.
const TANGENT_SCALE: f32 = 0.5;

/// Compute a Split-S trajectory: the drone rolls inverted, then pulls through
/// a half-loop, exiting at a lower altitude heading roughly opposite.
///
/// Returns `(curve, curve_len, duration)`.
///
/// The trajectory has 3 control points:
/// - Entry: current position, tangent = entry velocity direction
/// - Apex: above midpoint, tangent rotated to create the "over the top" shape
/// - Exit: target exit position, tangent = exit velocity direction
pub fn split_s_trajectory(
    entry_pos: Vec3,
    entry_vel: Vec3,
    exit_pos: Vec3,
    exit_vel: Vec3,
    speed: f32,
) -> (CubicCurve<Vec3>, f32, f32) {
    let midpoint = (entry_pos + exit_pos) * 0.5;
    let span = (exit_pos - entry_pos).length().max(1.0);
    let apex_height = span * SPLIT_S_APEX_HEIGHT_FRAC;

    let apex = midpoint + Vec3::Y * apex_height;

    // Tangent at apex: blend of entry and exit tangent directions, pointing "over"
    let entry_dir = entry_vel.normalize_or(Vec3::NEG_Z);
    let exit_dir = exit_vel.normalize_or(Vec3::NEG_Z);
    let apex_dir = (entry_dir + exit_dir).normalize_or(entry_dir);

    let tangent_mag = span * TANGENT_SCALE;
    let entry_tangent = entry_dir * tangent_mag;
    let apex_tangent = apex_dir * tangent_mag;
    let exit_tangent = exit_dir * tangent_mag;

    let curve = CubicHermite::new(
        [entry_pos, apex, exit_pos],
        [entry_tangent, apex_tangent, exit_tangent],
    )
    .to_curve()
    .expect("split-s trajectory needs >= 2 points");

    let curve_len = 2.0; // 3 points = 2 segments
    let duration = SPLIT_S_BASE_DURATION * (REF_SPEED / speed.max(10.0)).clamp(0.6, 1.4);

    (curve, curve_len, duration)
}

/// Compute a Power Loop trajectory: the drone pulls up into a full backward
/// vertical loop, exiting at roughly the same altitude heading the same direction.
///
/// Returns `(curve, curve_len, duration)`.
///
/// The trajectory has 4 control points:
/// - Entry: current position
/// - Climb: above entry, pitched upward
/// - Inverted apex: above exit, heading back
/// - Exit: target exit position
pub fn power_loop_trajectory(
    entry_pos: Vec3,
    entry_vel: Vec3,
    exit_pos: Vec3,
    exit_vel: Vec3,
    speed: f32,
) -> (CubicCurve<Vec3>, f32, f32) {
    let span = (exit_pos - entry_pos).length().max(1.0);
    let loop_height = span * POWER_LOOP_CLIMB_FRAC;

    let entry_dir = entry_vel.normalize_or(Vec3::NEG_Z);
    let exit_dir = exit_vel.normalize_or(Vec3::NEG_Z);

    // Climb point: above entry, slightly forward
    let climb = entry_pos + Vec3::Y * loop_height + entry_dir * span * 0.15;
    // Inverted apex: above exit, slightly backward (completing the loop)
    let inv_apex = exit_pos + Vec3::Y * loop_height - exit_dir * span * 0.15;

    let tangent_mag = span * TANGENT_SCALE;

    // Entry tangent: forward and slightly up (initiating the pull-up)
    let entry_tangent = (entry_dir + Vec3::Y * 0.5).normalize() * tangent_mag;
    // Climb tangent: mostly upward, curving backward
    let climb_tangent = (Vec3::Y + entry_dir * 0.3).normalize() * tangent_mag;
    // Inverted apex tangent: heading backward and down (top of the loop)
    let inv_apex_tangent = (-exit_dir + Vec3::NEG_Y * 0.3).normalize() * tangent_mag;
    // Exit tangent: forward, level (completing the loop)
    let exit_tangent = exit_dir * tangent_mag;

    let curve = CubicHermite::new(
        [entry_pos, climb, inv_apex, exit_pos],
        [entry_tangent, climb_tangent, inv_apex_tangent, exit_tangent],
    )
    .to_curve()
    .expect("power-loop trajectory needs >= 2 points");

    let curve_len = 3.0; // 4 points = 3 segments
    let duration = POWER_LOOP_BASE_DURATION * (REF_SPEED / speed.max(10.0)).clamp(0.6, 1.4);

    (curve, curve_len, duration)
}
