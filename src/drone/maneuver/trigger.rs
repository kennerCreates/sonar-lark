use bevy::prelude::*;

use super::detection::detect_maneuver;
use super::profiles;
use super::{ActiveManeuver, ManeuverCooldown, ManeuverKind, ManeuverPhaseTag, TiltOverride};
use crate::common::{FINISH_EXTENSION, POINTS_PER_GATE};
use crate::drone::components::*;

/// Raised tilt limit for Aggressive Bank (~103 degrees in radians).
const AGGRESSIVE_BANK_TILT: f32 = 1.80;

/// Spline parameter distance past which a stale ManeuverCooldown is removed.
const COOLDOWN_CLEANUP_MARGIN: f32 = 1.0;

/// Scans drones for upcoming tight turns and inserts `ActiveManeuver` (for
/// Split-S/Power Loop) or `TiltOverride` (for Aggressive Bank).
pub fn trigger_maneuvers(
    mut commands: Commands,
    time: Res<Time>,
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
        (Without<ActiveManeuver>, Without<TiltOverride>),
    >,
) {
    if tuning.maneuver_enabled < 0.5 {
        return;
    }

    let now = time.elapsed_secs();

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

        match trigger.kind {
            ManeuverKind::SplitS | ManeuverKind::PowerLoop => {
                let entry_velocity = dynamics.velocity;
                let yaw_flat = Vec3::new(entry_velocity.x, 0.0, entry_velocity.z);
                let entry_yaw_dir = yaw_flat.normalize_or(Vec3::NEG_Z);
                let entry_speed = entry_velocity.length();

                commands.entity(entity).insert(ActiveManeuver {
                    kind: trigger.kind,
                    phase: ManeuverPhaseTag::Entry,
                    phase_progress: 0.0,
                    phase_start_time: now,
                    phase_duration: profiles::phase_duration(
                        trigger.kind,
                        ManeuverPhaseTag::Entry,
                        entry_speed,
                    ),
                    entry_velocity,
                    entry_position: transform.translation,
                    exit_spline_t: trigger.exit_t,
                    entry_yaw_dir,
                    entry_altitude: transform.translation.y,
                });
            }
            ManeuverKind::AggressiveBank => {
                commands.entity(entity).insert(TiltOverride {
                    max_tilt: AGGRESSIVE_BANK_TILT,
                    exit_spline_t: trigger.exit_t,
                });
            }
        }

        // Remove cooldown since a new maneuver is starting
        commands.entity(entity).remove::<ManeuverCooldown>();
    }
}
