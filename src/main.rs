#![allow(clippy::type_complexity, clippy::too_many_arguments)]

mod camera;
mod common;
mod course;
mod dev_menu;
mod drone;
mod editor;
mod menu;
mod obstacle;
pub mod palette;
mod persistence;
mod pilot;
mod race;
mod rendering;
mod results;
mod states;
pub mod ui_theme;

use bevy::picking::mesh_picking::MeshPickingPlugin;
use bevy::prelude::*;
use bevy::window::WindowResolution;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(1920, 1080),
                    title: "Sonar Lark".into(),
                    ..default()
                }),
                ..default()
            }),
            MeshPickingPlugin,
        ))
        .init_state::<states::AppState>()
        .add_sub_state::<states::EditorMode>()
        .add_plugins((
            rendering::RenderingPlugin,
            ui_theme::UiThemePlugin,
            common::CommonPlugin,
            menu::MenuPlugin,
            obstacle::ObstaclePlugin,
            course::CoursePlugin,
            editor::EditorPlugin,
            pilot::PilotPlugin,
            drone::DronePlugin,
            race::RacePlugin,
            camera::CameraPlugin,
            results::ResultsPlugin,
            dev_menu::DevMenuPlugin,
        ))
        .run();
}
