use bevy::prelude::*;

use super::{ActiveManeuver, ManeuverCooldown, ManeuverPhaseTag, PendingManeuver, TiltOverride};
use crate::common::{FINISH_EXTENSION, POINTS_PER_GATE};
use crate::drone::components::*;
use crate::race::progress::RaceProgress;

/// Removes `ActiveManeuver` when Recovery phase completes (progress >= 1.0).
/// Resets PID integral to prevent windup kick on re-engagement,
/// jumps `spline_t` to the maneuver's exit point (capped to not exceed the
/// next gate's miss threshold), and inserts a `ManeuverCooldown` to prevent
/// immediate re-triggering.
/// Also removes `ActiveManeuver` from crashed drones (no cooldown needed).
pub fn cleanup_completed_maneuvers(
    mut commands: Commands,
    race_progress: Option<Res<RaceProgress>>,
    mut query: Query<(
        Entity,
        &ActiveManeuver,
        &Drone,
        &DronePhase,
        &mut PositionPid,
        &mut AIController,
    )>,
) {
    for (entity, maneuver, drone, phase, mut pid, mut ai) in &mut query {
        let should_remove = *phase == DronePhase::Crashed
            || (maneuver.phase == ManeuverPhaseTag::Recovery && maneuver.phase_progress >= 1.0);

        if should_remove {
            commands.entity(entity).remove::<ActiveManeuver>();
            pid.integral = Vec3::ZERO;
            if *phase != DronePhase::Crashed {
                let mut exit_t = maneuver.exit_spline_t;

                // Cap exit_t so the spline_t jump can't exceed the next gate's
                // miss threshold. During a flip the drone may not physically
                // cross the gate plane, so an uncapped jump would trigger
                // miss_detection immediately.
                if let Some(ref progress) = race_progress {
                    let idx = drone.index as usize;
                    if let Some(state) = progress.drone_states.get(idx) {
                        let cycle_t = ai.gate_count as f32 * POINTS_PER_GATE;
                        let miss_threshold = if state.next_gate < progress.total_gates {
                            state.next_gate as f32 * POINTS_PER_GATE + 1.5
                        } else {
                            cycle_t + FINISH_EXTENSION
                        };
                        // Small margin so spline_t lands safely below the threshold.
                        exit_t = exit_t.min(miss_threshold - 0.1);
                    }
                }

                ai.spline_t = exit_t;
                commands
                    .entity(entity)
                    .insert(ManeuverCooldown { exit_t });
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

/// Removes `PendingManeuver` from crashed drones and from drones whose spline_t
/// has overshot the trigger point by a wide margin (stale pending).
pub fn cleanup_pending_maneuvers(
    mut commands: Commands,
    query: Query<(Entity, &AIController, &DronePhase, &PendingManeuver)>,
) {
    for (entity, ai, phase, pending) in &query {
        let stale = *phase == DronePhase::Crashed || ai.spline_t > pending.exit_t;
        if stale {
            commands.entity(entity).remove::<PendingManeuver>();
        }
    }
}
