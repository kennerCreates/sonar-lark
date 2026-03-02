use bevy::prelude::*;

use super::{ActiveManeuver, ManeuverCooldown, ManeuverPhaseTag, TiltOverride};
use crate::drone::components::*;

/// Removes `ActiveManeuver` when Recovery phase completes (progress >= 1.0).
/// Resets PID integral to prevent windup kick on re-engagement,
/// jumps `spline_t` to the maneuver's exit point, and inserts a
/// `ManeuverCooldown` to prevent immediate re-triggering.
/// Also removes `ActiveManeuver` from crashed drones (no cooldown needed).
pub fn cleanup_completed_maneuvers(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &ActiveManeuver,
        &DronePhase,
        &mut PositionPid,
        &mut AIController,
    )>,
) {
    for (entity, maneuver, phase, mut pid, mut ai) in &mut query {
        let should_remove = *phase == DronePhase::Crashed
            || (maneuver.phase == ManeuverPhaseTag::Recovery && maneuver.phase_progress >= 1.0);

        if should_remove {
            commands.entity(entity).remove::<ActiveManeuver>();
            pid.integral = Vec3::ZERO;
            if *phase != DronePhase::Crashed {
                ai.spline_t = maneuver.exit_spline_t;
                commands
                    .entity(entity)
                    .insert(ManeuverCooldown { exit_t: maneuver.exit_spline_t });
            }
        }
    }
}

/// Removes `TiltOverride` once the drone has passed the override's exit point
/// on the spline, and inserts a `ManeuverCooldown` to prevent immediate re-triggering.
pub fn cleanup_tilt_overrides(
    mut commands: Commands,
    query: Query<(Entity, &AIController, &TiltOverride)>,
) {
    for (entity, ai, tilt) in &query {
        if ai.spline_t >= tilt.exit_spline_t {
            commands.entity(entity).remove::<TiltOverride>();
            commands
                .entity(entity)
                .insert(ManeuverCooldown { exit_t: tilt.exit_spline_t });
        }
    }
}
