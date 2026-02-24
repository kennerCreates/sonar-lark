use bevy::prelude::*;

use crate::drone::components::{AIController, Drone, DronePhase};

use super::progress::{DroneRaceState, RaceProgress};
use super::timing::RaceClock;

#[derive(Resource, Default, PartialEq, Eq, Clone, Copy, Debug)]
pub enum RacePhase {
    #[default]
    WaitingToStart,
    Countdown,
    Racing,
    Finished,
}

#[derive(Resource)]
pub struct CountdownTimer {
    pub remaining: f32,
}

impl Default for CountdownTimer {
    fn default() -> Self {
        Self { remaining: 3.0 }
    }
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

/// Ticks the countdown timer each frame, then transitions to Racing when it expires.
pub fn tick_countdown(
    time: Res<Time>,
    mut phase: ResMut<RacePhase>,
    mut timer: Option<ResMut<CountdownTimer>>,
    mut commands: Commands,
    mut drones: Query<(&mut DronePhase, &AIController), With<Drone>>,
    drone_count: Query<(), With<Drone>>,
) {
    if *phase != RacePhase::Countdown {
        return;
    }
    let Some(ref mut timer) = timer else { return };

    timer.remaining -= time.delta_secs();

    if timer.remaining <= 0.0 {
        *phase = RacePhase::Racing;
        commands.remove_resource::<CountdownTimer>();

        // Start all drones racing
        let mut total_gates = 0u32;
        for (mut drone_phase, ai) in &mut drones {
            *drone_phase = DronePhase::Racing;
            total_gates = ai.gate_count;
        }

        // Start race clock
        commands.insert_resource(RaceClock {
            elapsed: 0.0,
            running: true,
        });

        // Initialize RaceProgress
        let drone_count = drone_count.iter().count();
        let drone_states = (0..drone_count)
            .map(|_| DroneRaceState::default())
            .collect();
        commands.insert_resource(RaceProgress {
            drone_states,
            total_gates,
        });

        info!(
            "GO! Race started with {} drones on {}-gate course",
            drone_count, total_gates
        );
    }
}

/// Transitions from Racing → Finished when every drone has finished or crashed.
pub fn check_race_finished(
    mut phase: ResMut<RacePhase>,
    progress: Option<Res<RaceProgress>>,
    mut clock: Option<ResMut<RaceClock>>,
) {
    if *phase != RacePhase::Racing {
        return;
    }
    let Some(progress) = progress else { return };
    if progress.drone_states.is_empty() {
        return;
    }

    let all_done = progress
        .drone_states
        .iter()
        .all(|s| s.finished || s.crashed);
    if all_done {
        *phase = RacePhase::Finished;
        if let Some(ref mut clock) = clock {
            clock.running = false;
        }
        info!("Race finished! All drones completed or crashed.");
    }
}
