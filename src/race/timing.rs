use bevy::prelude::*;

#[derive(Resource)]
pub struct RaceClock {
    pub elapsed: f32,
    pub running: bool,
}
