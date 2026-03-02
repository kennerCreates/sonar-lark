use bevy::prelude::*;

use super::detection::detect_maneuver;
use super::trajectory;
use super::{ManeuverCooldown, ManeuverKind, ManeuverTrajectory, PendingManeuver, TiltOverride};
use crate::common::{FINISH_EXTENSION, POINTS_PER_GATE};
use crate::drone::ai::{cyclic_pos, cyclic_vel};
use crate::drone::components::*;

/// Raised tilt limit for Aggressive Bank (~103 degrees in radians).
const AGGRESSIVE_BANK_TILT: f32 = 1.80;

/// Spline parameter distance past which a stale ManeuverCooldown is removed.
const COOLDOWN_CLEANUP_MARGIN: f32 = 1.0;

/// Scans drones for upcoming tight turns and inserts `PendingManeuver`.
/// Drones that already have a pending, active, or tilt-override maneuver are skipped.
pub fn trigger_maneuvers(
    mut commands: Commands,
    tuning: Res<AiTuningParams>,
    query: Query<
        (
            Entity,
            &Transform,
            &AIController,
            &DroneConfig,
            &DroneDynamics,
            &DronePhase,
            Option<&ManeuverCooldown>,
        ),
        (
            Without<ManeuverTrajectory>,
            Without<TiltOverride>,
            Without<PendingManeuver>,
        ),
    >,
) {
    if tuning.maneuver_enabled < 0.5 {
        return;
    }

    for (entity, transform, ai, config, dynamics, phase, cooldown) in &query {
        if !matches!(*phase, DronePhase::Racing | DronePhase::VictoryLap) {
            continue;
        }

        let cycle_t = ai.gate_count as f32 * POINTS_PER_GATE;
        let finish_t = match *phase {
            DronePhase::Racing => Some(cycle_t + FINISH_EXTENSION),
            _ => None,
        };

        let last_exit_t = cooldown.map(|c| c.exit_t);

        // Clean up stale cooldown markers
        if let Some(cd) = cooldown
            && (ai.spline_t - cd.exit_t).abs() > COOLDOWN_CLEANUP_MARGIN
        {
            commands.entity(entity).remove::<ManeuverCooldown>();
        }

        let Some(trigger) = detect_maneuver(
            &ai.spline,
            ai.spline_t,
            cycle_t,
            finish_t,
            transform.translation.y,
            dynamics.velocity.length(),
            &ai.gate_positions,
            transform.translation,
            config.maneuver_threshold_mult,
            tuning.maneuver_turn_threshold,
            tuning.maneuver_altitude_min,
            last_exit_t,
        ) else {
            continue;
        };

        commands.entity(entity).insert(PendingManeuver {
            kind: trigger.kind,
            trigger_t: trigger.trigger_t,
            exit_t: trigger.exit_t,
        });

        commands.entity(entity).remove::<ManeuverCooldown>();
    }
}

/// Converts `PendingManeuver` into `ManeuverTrajectory` (for SplitS/PowerLoop) or
/// `TiltOverride` (for AggressiveBank) once the drone's spline_t reaches the trigger point.
pub fn activate_pending_maneuvers(
    time: Res<Time>,
    mut commands: Commands,
    query: Query<(
        Entity,
        &Transform,
        &AIController,
        &DroneDynamics,
        &DronePhase,
        &PendingManeuver,
        &Drone,
    )>,
) {
    let now = time.elapsed_secs();

    for (entity, transform, ai, dynamics, phase, pending, drone) in &query {
        if *phase == DronePhase::Crashed {
            commands.entity(entity).remove::<PendingManeuver>();
            continue;
        }

        if ai.spline_t < pending.trigger_t {
            continue;
        }

        commands.entity(entity).remove::<PendingManeuver>();

        let cycle_t = ai.gate_count as f32 * POINTS_PER_GATE;
        let speed = dynamics.velocity.length().max(1.0);

        match pending.kind {
            ManeuverKind::SplitS => {
                let exit_pos = cyclic_pos(&ai.spline, pending.exit_t, cycle_t);
                let exit_vel = cyclic_vel(&ai.spline, pending.exit_t, cycle_t)
                    .normalize_or(Vec3::NEG_Z)
                    * speed;

                let (curve, curve_len, duration) = trajectory::split_s_trajectory(
                    transform.translation,
                    dynamics.velocity,
                    exit_pos,
                    exit_vel,
                    speed,
                );

                info!(
                    "Drone {} activated SplitS trajectory at spline_t={:.1}, exit_t={:.1}, dur={:.2}s",
                    drone.index, ai.spline_t, pending.exit_t, duration
                );

                commands.entity(entity).insert(ManeuverTrajectory {
                    kind: ManeuverKind::SplitS,
                    curve,
                    curve_len,
                    start_time: now,
                    duration,
                    exit_spline_t: pending.exit_t,
                    entry_speed: speed,
                });
            }
            ManeuverKind::PowerLoop => {
                let exit_pos = cyclic_pos(&ai.spline, pending.exit_t, cycle_t);
                let exit_vel = cyclic_vel(&ai.spline, pending.exit_t, cycle_t)
                    .normalize_or(Vec3::NEG_Z)
                    * speed;

                let (curve, curve_len, duration) = trajectory::power_loop_trajectory(
                    transform.translation,
                    dynamics.velocity,
                    exit_pos,
                    exit_vel,
                    speed,
                );

                info!(
                    "Drone {} activated PowerLoop trajectory at spline_t={:.1}, exit_t={:.1}, dur={:.2}s",
                    drone.index, ai.spline_t, pending.exit_t, duration
                );

                commands.entity(entity).insert(ManeuverTrajectory {
                    kind: ManeuverKind::PowerLoop,
                    curve,
                    curve_len,
                    start_time: now,
                    duration,
                    exit_spline_t: pending.exit_t,
                    entry_speed: speed,
                });
            }
            ManeuverKind::AggressiveBank => {
                info!(
                    "Drone {} activated AggressiveBank as TiltOverride at spline_t={:.1}, exit_t={:.1}",
                    drone.index, ai.spline_t, pending.exit_t
                );
                commands.entity(entity).insert(TiltOverride {
                    max_tilt: AGGRESSIVE_BANK_TILT,
                    exit_spline_t: pending.exit_t,
                });
            }
        }
    }
}
