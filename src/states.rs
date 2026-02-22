use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
pub enum AppState {
    #[default]
    Menu,
    Editor,
    Race,
    Results,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, SubStates)]
#[source(AppState = AppState::Editor)]
pub enum EditorMode {
    #[default]
    ObstacleWorkshop,
    CourseEditor,
}
