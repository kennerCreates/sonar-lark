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
mod pilot;
mod race;
mod rendering;
mod results;
mod states;

use bevy::picking::mesh_picking::MeshPickingPlugin;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MeshPickingPlugin))
        .init_state::<states::AppState>()
        .add_sub_state::<states::EditorMode>()
        .add_plugins((
            rendering::RenderingPlugin,
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
