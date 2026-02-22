use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum CameraMode {
    #[default]
    Spectator,
    Fpv,
    Chase,
}

#[derive(Resource, Default)]
pub struct CameraState {
    pub mode: CameraMode,
    pub target_drone: Option<Entity>,
}
