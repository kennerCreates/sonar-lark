pub mod course_topology;
pub mod drone_identity;
pub mod race_participant;

pub use course_topology::{FINISH_EXTENSION, POINTS_PER_GATE};

pub use race_participant::RaceParticipant;

use bevy::picking::Pickable;
use bevy::prelude::*;

use crate::palette;
use crate::rendering::{
    CelLightDir, CelMaterial, cel_material_flat,
    skybox::{self, SkyboxMaterial},
    light_direction_from_transform,
};

pub struct CommonPlugin;

impl Plugin for CommonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_environment);
    }
}

fn setup_environment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    mut skybox_materials: ResMut<Assets<SkyboxMaterial>>,
) {
    // Light
    let light_transform =
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, -0.5, 0.0));
    let light_dir = light_direction_from_transform(&light_transform);
    commands.insert_resource(CelLightDir(light_dir));

    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            ..default()
        },
        light_transform,
    ));

    // Ground plane: dark TRON ground (not pickable — clicks pass through to obstacles)
    // Uses flat material (no cel-shading) since the uniform normal would place the
    // entire plane in one lighting band, distorting the intended color.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(6000.0, 6000.0))),
        MeshMaterial3d(cel_materials.add(cel_material_flat(
            palette::SMOKY_BLACK,
            light_dir,
        ))),
        Pickable::IGNORE,
    ));

    // Skybox
    skybox::spawn_skybox(&mut commands, &mut meshes, &mut skybox_materials);
}
