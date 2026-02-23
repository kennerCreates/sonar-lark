pub mod workshop;
pub mod course_editor;
pub(crate) mod gizmos;

use bevy::prelude::*;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            workshop::WorkshopPlugin,
            course_editor::CourseEditorPlugin,
        ));
    }
}
