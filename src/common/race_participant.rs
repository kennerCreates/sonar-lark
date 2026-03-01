use bevy::prelude::*;

/// Marker for any entity participating in a race (AI drone, future player drone).
/// Carries the participant's index for correlating with RaceProgress.
#[derive(Component)]
pub struct RaceParticipant {
    pub index: u8,
}

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
