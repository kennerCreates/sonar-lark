use bevy::prelude::*;

#[derive(Resource)]
pub struct RaceClock {
    pub elapsed: f32,
    pub running: bool,
}

impl Default for RaceClock {
    fn default() -> Self {
        Self {
            elapsed: 0.0,
            running: false,
        }
    }
}

pub fn tick_race_clock(time: Res<Time>, mut clock: Option<ResMut<RaceClock>>) {
    if let Some(ref mut clock) = clock {
        if clock.running {
            clock.elapsed += time.delta_secs();
        }
    }
}
