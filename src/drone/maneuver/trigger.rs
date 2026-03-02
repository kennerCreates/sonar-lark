use bevy::prelude::*;

use super::detection::detect_maneuver;
use super::profiles;
use super::{
    ActiveManeuver, ManeuverCooldown, ManeuverKind, ManeuverPhaseTag, PendingManeuver,
    TiltOverride,
};
use crate::common::{FINISH_EXTENSION, POINTS_PER_GATE};
use crate::drone::components::*;
use crate::race::progress::RaceProgress;

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
            Without<ActiveManeuver>,
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

        // If the drone is already past the trigger point (turn is right here),
        // the activate_pending_maneuvers pass will fire it this same tick.
        commands.entity(entity).insert(PendingManeuver {
            kind: trigger.kind,
            trigger_t: trigger.trigger_t,
            exit_t: trigger.exit_t,
        });

        // Remove cooldown since a new maneuver is being planned
        commands.entity(entity).remove::<ManeuverCooldown>();
    }
}

/// Converts `PendingManeuver` into `ActiveManeuver` or `TiltOverride` once the
/// drone's spline_t reaches the trigger point.
pub fn activate_pending_maneuvers(
    mut commands: Commands,
    time: Res<Time>,
    tuning: Res<AiTuningParams>,
    progress: Option<Res<RaceProgress>>,
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

        // For SplitS, defer activation if an uncrossed gate sits between the drone
        // and the maneuver exit. The drone must cross the gate physically before
        // position_pid is bypassed.
        if pending.kind == ManeuverKind::SplitS
            && let Some(ref progress) = progress
        {
            let drone_idx = drone.index as usize;
            if let Some(state) = progress.drone_states.get(drone_idx) {
                let next_gate_t = state.next_gate as f32 * POINTS_PER_GATE;
                if next_gate_t < pending.exit_t {
                    continue; // Wait for gate crossing
                }
            }
        }

        commands.entity(entity).remove::<PendingManeuver>();

        match pending.kind {
            ManeuverKind::SplitS => {
                // Re-check altitude at activation time — downgrade to AggressiveBank if too low
                if transform.translation.y < tuning.maneuver_altitude_min {
                    info!(
                        "Drone {} SplitS→AggressiveBank (alt {:.1} < min {:.1})",
                        drone.index, transform.translation.y, tuning.maneuver_altitude_min
                    );
                    commands.entity(entity).insert(TiltOverride {
                        max_tilt: AGGRESSIVE_BANK_TILT,
                        exit_spline_t: pending.exit_t,
                    });
                    continue;
                }

                let entry_velocity = dynamics.velocity;
                let yaw_flat = Vec3::new(entry_velocity.x, 0.0, entry_velocity.z);
                let entry_yaw_dir = yaw_flat.normalize_or(Vec3::NEG_Z);
                let entry_speed = entry_velocity.length();

                info!(
                    "Drone {} activated SplitS at spline_t={:.1}, exit_t={:.1}, alt={:.1}",
                    drone.index, ai.spline_t, pending.exit_t, transform.translation.y
                );

                commands.entity(entity).insert(ActiveManeuver {
                    kind: ManeuverKind::SplitS,
                    phase: ManeuverPhaseTag::Entry,
                    phase_progress: 0.0,
                    phase_start_time: now,
                    phase_duration: profiles::phase_duration(
                        ManeuverKind::SplitS,
                        ManeuverPhaseTag::Entry,
                        entry_speed,
                    ),
                    entry_velocity,
                    entry_position: transform.translation,
                    exit_spline_t: pending.exit_t,
                    entry_yaw_dir,
                    entry_altitude: transform.translation.y,
                });
            }
            ManeuverKind::PowerLoop | ManeuverKind::AggressiveBank => {
                info!(
                    "Drone {} activated AggressiveBank at spline_t={:.1}, exit_t={:.1}",
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
