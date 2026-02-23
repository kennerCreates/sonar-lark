use bevy::prelude::*;

use crate::drone::components::{AIController, Drone, POINTS_PER_GATE};

#[derive(Resource, Default, PartialEq, Eq, Clone, Copy, Debug)]
pub enum RacePhase {
    #[default]
    WaitingToStart,
    Racing,
    Finished,
}

/// Run condition: returns true only when `RacePhase::Racing` is active.
pub fn race_is_running(phase: Option<Res<RacePhase>>) -> bool {
    phase.is_some_and(|p| *p == RacePhase::Racing)
}

/// Transitions from Racing → Finished when every drone has completed all waypoints.
pub fn check_race_finished(
    mut phase: ResMut<RacePhase>,
    drones: Query<&AIController, With<Drone>>,
) {
    if *phase != RacePhase::Racing {
        return;
    }
    if drones.is_empty() {
        return;
    }
    let all_finished = drones
        .iter()
        .all(|ai| ai.spline_t >= ai.gate_count as f32 * POINTS_PER_GATE);
    if all_finished {
        *phase = RacePhase::Finished;
        info!("Race finished! All drones completed the course.");
    }
}
