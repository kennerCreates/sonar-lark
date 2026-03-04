pub mod workshop;
pub mod course_editor;
pub(crate) mod gizmos;
pub mod types;

pub use types::EditorTab;

use bevy::prelude::*;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(course_editor::CourseEditorPlugin);
    }
}
