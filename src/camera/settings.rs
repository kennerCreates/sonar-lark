use bevy::prelude::*;

#[derive(Resource)]
pub struct CameraSettings {
    pub zoom_min: f32,
    pub zoom_max: f32,
    pub fov_degrees: f32,
    pub move_speed: f32,
    pub sensitivity: f32,
    pub edge_scroll_speed: f32,
    pub edge_scroll_margin: f32,
    pub zoom_speed: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            zoom_min: 5.0,
            zoom_max: 240.0,
            fov_degrees: 60.0,
            move_speed: 25.0,
            sensitivity: 0.005,
            edge_scroll_speed: 15.0,
            edge_scroll_margin: 20.0,
            zoom_speed: 2.0,
        }
    }
}
