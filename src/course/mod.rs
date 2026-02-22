pub mod data;
pub mod loader;

use bevy::prelude::*;

pub struct CoursePlugin;

impl Plugin for CoursePlugin {
    fn build(&self, _app: &mut App) {
        // Course loading/saving systems will be registered in later phases
    }
}
