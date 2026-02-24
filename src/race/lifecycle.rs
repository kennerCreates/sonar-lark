use bevy::prelude::*;

use crate::drone::components::{Drone, DronePhase};

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

/// Run condition: returns true when any drone is actively racing or returning.
/// Used to keep AI systems running during the post-race return flight.
pub fn drones_are_active(
    phase: Option<Res<RacePhase>>,
    drones: Query<&DronePhase, With<Drone>>,
) -> bool {
    if phase.is_some_and(|p| *p == RacePhase::Racing) {
        return true;
    }
    drones.iter().any(|dp| *dp == DronePhase::Returning)
}

/// Transitions from Racing → Finished when every drone has passed the last gate.
pub fn check_race_finished(
    mut phase: ResMut<RacePhase>,
    drones: Query<&DronePhase, With<Drone>>,
) {
    if *phase != RacePhase::Racing {
        return;
    }
    if drones.is_empty() {
        return;
    }
    let all_finished = drones.iter().all(|dp| *dp != DronePhase::Racing);
    if all_finished {
        *phase = RacePhase::Finished;
        info!("Race finished! All drones completed the course.");
    }
}
