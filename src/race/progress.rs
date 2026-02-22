use bevy::prelude::*;

pub struct DroneRaceState {
    pub next_gate: u32,
    pub gates_passed: u32,
    pub finished: bool,
    pub finish_time: Option<f32>,
    pub crashed: bool,
}

#[derive(Resource, Default)]
pub struct RaceProgress {
    pub drone_states: Vec<DroneRaceState>,
    pub total_gates: u32,
}
