use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
pub enum AppState {
    #[default]
    Menu,
    Editor,
    Race,
    Results,
    DevMenu,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, SubStates)]
#[source(AppState = AppState::Editor)]
pub enum EditorMode {
    #[default]
    CourseEditor,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, SubStates)]
#[source(AppState = AppState::DevMenu)]
pub enum DevMenuPage {
    #[default]
    PilotGenerator,
    PaletteEditor,
    ObstacleWorkshop,
}

/// Inserted before entering the editor to request auto-loading a specific course.
/// Consumed by `auto_load_pending_course` once glTF assets are ready.
#[derive(Resource)]
pub struct PendingEditorCourse {
    pub path: String,
}

/// Tracks the last course loaded or saved in the editor.
/// Persists across states so the editor can reopen it.
#[derive(Resource)]
#[allow(dead_code)]
pub struct LastEditedCourse {
    pub path: String,
}
