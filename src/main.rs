mod camera;
mod common;
mod course;
mod drone;
mod editor;
mod menu;
mod obstacle;
mod race;
mod results;
mod states;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<states::AppState>()
        .add_sub_state::<states::EditorMode>()
        .add_plugins((
            common::CommonPlugin,
            menu::MenuPlugin,
            obstacle::ObstaclePlugin,
            course::CoursePlugin,
            editor::EditorPlugin,
            drone::DronePlugin,
            race::RacePlugin,
            camera::CameraPlugin,
            results::ResultsPlugin,
        ))
        .run();
}
