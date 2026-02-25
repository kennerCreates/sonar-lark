use bevy::picking::Pickable;
use bevy::prelude::*;

pub struct CommonPlugin;

impl Plugin for CommonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_environment);
    }
}

fn setup_environment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Light
    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, -0.5, 0.0)),
    ));

    // Ground plane (not pickable — clicks pass through to obstacles)
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(6000.0, 6000.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.024, 0.502, 0.318), // Jungle #068051
            perceptual_roughness: 1.0,
            reflectance: 0.0,
            ..default()
        })),
        Pickable::IGNORE,
    ));
}
