use bevy::prelude::*;

/// Marker for any entity participating in a race (AI drone, future player drone).
#[derive(Component)]
pub struct RaceParticipant;

/// Per-drone lifecycle phase.
#[derive(Component, Default, PartialEq, Eq, Clone, Copy, Debug)]
pub enum DronePhase {
    #[default]
    Idle,
    Racing,
    /// Drone has finished the race and is continuing to lap the course.
    VictoryLap,
    /// Drone wanders freely in the Results screen.
    Wandering,
    Crashed,
}
