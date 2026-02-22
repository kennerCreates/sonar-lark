pub mod ui;

use bevy::prelude::*;

pub struct ResultsPlugin;

impl Plugin for ResultsPlugin {
    fn build(&self, _app: &mut App) {
        // Results systems will be registered in Phase 8
    }
}
