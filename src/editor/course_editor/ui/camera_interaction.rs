use bevy::prelude::*;

use crate::editor::course_editor::PlacedCamera;
use crate::palette;
use crate::rendering::{CelLightDir, CelMaterial, cel_material_from_color};

use super::types::*;

/// Default local offset for a gate camera: slightly behind and above the gate center.
pub const DEFAULT_CAMERA_OFFSET: Vec3 = Vec3::new(0.0, 2.0, -5.0);

pub fn setup_camera_editor_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    light_dir: Res<CelLightDir>,
) {
    let mesh = meshes.add(Cuboid::new(0.3, 0.3, 0.5));
    let material = cel_materials.add(cel_material_from_color(palette::SKY, light_dir.0));
    let primary_material =
        cel_materials.add(cel_material_from_color(palette::SUNSHINE, light_dir.0));
    commands.insert_resource(CameraEditorMeshes {
        mesh,
        material,
        primary_material,
    });
}

/// Spawn a camera as a child of the given gate entity. Returns the camera entity id.
pub fn spawn_gate_camera(
    commands: &mut Commands,
    gate_entity: Entity,
    meshes: &CameraEditorMeshes,
    is_primary: bool,
    offset: Vec3,
    rotation: Quat,
) -> Entity {
    let material = if is_primary {
        meshes.primary_material.clone()
    } else {
        meshes.material.clone()
    };
    let cam_transform = Transform::from_translation(offset).with_rotation(rotation);
    let cam_entity = commands
        .spawn((
            cam_transform,
            Visibility::default(),
            Mesh3d(meshes.mesh.clone()),
            MeshMaterial3d(material),
            PlacedCamera {
                is_primary,
                label: None,
            },
        ))
        .id();
    commands.entity(gate_entity).add_child(cam_entity);
    cam_entity
}
