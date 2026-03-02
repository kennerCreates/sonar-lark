use bevy::prelude::*;

use super::profiles;
use super::{ActiveManeuver, ManeuverPhaseTag};
use crate::drone::components::*;

/// Advances maneuver phases, computes target orientation and thrust, and writes
/// directly to `DesiredAttitude` — bypassing `position_pid` for maneuvering drones.
pub fn execute_maneuvers(
    time: Res<Time>,
    mut query: Query<(
        &mut ActiveManeuver,
        &mut DesiredAttitude,
        &DroneDynamics,
        &DronePhase,
    )>,
) {
    let now = time.elapsed_secs();

    for (mut maneuver, mut attitude, dynamics, phase) in &mut query {
        if *phase == DronePhase::Crashed {
            continue;
        }

        // Advance phase progress
        let elapsed_in_phase = now - maneuver.phase_start_time;
        maneuver.phase_progress = if maneuver.phase_duration > 0.0 {
            (elapsed_in_phase / maneuver.phase_duration).clamp(0.0, 1.0)
        } else {
            1.0
        };

        // Transition to next phase when current one completes
        if maneuver.phase_progress >= 1.0 {
            let next_phase = match maneuver.phase {
                ManeuverPhaseTag::Entry => Some(ManeuverPhaseTag::Ballistic),
                ManeuverPhaseTag::Ballistic => Some(ManeuverPhaseTag::Recovery),
                ManeuverPhaseTag::Recovery => None, // cleanup system handles removal
            };

            if let Some(next) = next_phase {
                let entry_speed = maneuver.entry_velocity.length();
                maneuver.phase = next;
                maneuver.phase_start_time = now;
                maneuver.phase_duration =
                    profiles::phase_duration(maneuver.kind, next, entry_speed);
                maneuver.phase_progress = 0.0;
            }
        }

        // Compute target orientation and thrust from profiles
        attitude.orientation = profiles::maneuver_target_orientation(
            maneuver.kind,
            maneuver.phase,
            maneuver.phase_progress,
            maneuver.entry_yaw_dir,
            maneuver.entry_velocity,
        );

        let thrust_fraction = profiles::maneuver_thrust_fraction(
            maneuver.kind,
            maneuver.phase,
            maneuver.phase_progress,
        );
        attitude.thrust_magnitude = thrust_fraction * dynamics.max_thrust;
    }
}
