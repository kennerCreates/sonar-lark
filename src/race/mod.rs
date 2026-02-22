pub mod gate;
pub mod progress;
pub mod timing;
pub mod lifecycle;

use bevy::prelude::*;

pub struct RacePlugin;

impl Plugin for RacePlugin {
    fn build(&self, _app: &mut App) {
        // Race systems will be registered in Phase 7
    }
}
