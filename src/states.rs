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
    ObstacleWorkshop,
    #[default]
    CourseEditor,
}
