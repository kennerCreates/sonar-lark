use bevy::prelude::*;

use super::detection::detect_maneuver;
use super::{ActiveManeuver, ManeuverCooldown, PendingManeuver, TiltOverride};
use crate::common::{FINISH_EXTENSION, POINTS_PER_GATE};
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

/// Converts `PendingManeuver` into `TiltOverride` once the drone's spline_t
/// reaches the trigger point. All maneuver kinds use TiltOverride so the
/// position PID stays in control — the drone just gets a raised tilt limit
/// for aggressive banking through tight turns.
pub fn activate_pending_maneuvers(
    mut commands: Commands,
    query: Query<(
        Entity,
        &AIController,
        &DronePhase,
        &PendingManeuver,
        &Drone,
    )>,
) {
    for (entity, ai, phase, pending, drone) in &query {
        if *phase == DronePhase::Crashed {
            commands.entity(entity).remove::<PendingManeuver>();
            continue;
        }

        if ai.spline_t < pending.trigger_t {
            continue;
        }

        commands.entity(entity).remove::<PendingManeuver>();

        info!(
            "Drone {} activated {:?} as TiltOverride at spline_t={:.1}, exit_t={:.1}",
            drone.index, pending.kind, ai.spline_t, pending.exit_t
        );
        commands.entity(entity).insert(TiltOverride {
            max_tilt: AGGRESSIVE_BANK_TILT,
            exit_spline_t: pending.exit_t,
        });
    }
}
