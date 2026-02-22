use bevy::prelude::*;

/// Path to the currently selected course file.
#[derive(Resource)]
pub struct SelectedCourse {
    pub path: String,
}
