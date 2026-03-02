use bevy::prelude::*;

use crate::editor::course_editor::{EditorSelection, PlacedCamera, PlacedObstacle};
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

/// Check whether a gate entity already has a PlacedCamera child.
pub fn gate_has_camera(
    gate_entity: Entity,
    camera_child_query: &Query<&ChildOf, With<PlacedCamera>>,
) -> bool {
    camera_child_query
        .iter()
        .any(|child_of| child_of.parent() == gate_entity)
}

pub fn handle_camera_placement(
    mut commands: Commands,
    mut selection: ResMut<EditorSelection>,
    query: Query<&Interaction, (Changed<Interaction>, With<PlaceCameraButton>)>,
    obstacle_query: Query<&PlacedObstacle>,
    camera_child_query: Query<&ChildOf, With<PlacedCamera>>,
    camera_meshes: Option<Res<CameraEditorMeshes>>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let Some(ref meshes) = camera_meshes else {
            continue;
        };
        let Some(gate_entity) = selection.entity else {
            warn!("Select a gate first to add a camera");
            continue;
        };
        let Ok(placed) = obstacle_query.get(gate_entity) else {
            warn!("Selected entity is not an obstacle");
            continue;
        };
        if placed.gate_order.is_none() {
            warn!("Selected obstacle is not a gate");
            continue;
        };
        if gate_has_camera(gate_entity, &camera_child_query) {
            warn!("Gate already has a camera");
            continue;
        }

        let cam_entity = spawn_gate_camera(
            &mut commands,
            gate_entity,
            meshes,
            false,
            DEFAULT_CAMERA_OFFSET,
            Quat::IDENTITY,
        );
        selection.entity = Some(cam_entity);
        selection.palette_id = None;
    }
}

pub fn handle_remove_camera(
    mut commands: Commands,
    mut selection: ResMut<EditorSelection>,
    query: Query<&Interaction, (Changed<Interaction>, With<RemoveCameraButton>)>,
    camera_query: Query<(), With<PlacedCamera>>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let Some(entity) = selection.entity else {
            continue;
        };
        if camera_query.get(entity).is_err() {
            warn!("Selected entity is not a camera");
            continue;
        }
        commands.entity(entity).despawn();
        selection.entity = None;
    }
}

pub fn handle_camera_primary_toggle(
    query: Query<&Interaction, (Changed<Interaction>, With<CameraPrimaryToggle>)>,
    selection: Res<EditorSelection>,
    mut camera_query: Query<(Entity, &mut PlacedCamera, &mut MeshMaterial3d<CelMaterial>)>,
    camera_meshes: Option<Res<CameraEditorMeshes>>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let Some(entity) = selection.entity else {
            continue;
        };
        let Some(ref meshes) = camera_meshes else {
            continue;
        };

        // Toggle the selected camera's primary status
        let Ok((_, mut cam, _)) = camera_query.get_mut(entity) else {
            continue;
        };
        let new_primary = !cam.is_primary;
        cam.is_primary = new_primary;

        // If becoming primary, clear all others
        if new_primary {
            for (e, mut other_cam, mut other_mat) in &mut camera_query {
                if e != entity && other_cam.is_primary {
                    other_cam.is_primary = false;
                    other_mat.0 = meshes.material.clone();
                }
            }
        }

        // Update the toggled camera's material
        if let Ok((_, _, mut mat)) = camera_query.get_mut(entity) {
            mat.0 = if new_primary {
                meshes.primary_material.clone()
            } else {
                meshes.material.clone()
            };
        }
    }
}

pub fn update_camera_primary_label(
    selection: Res<EditorSelection>,
    camera_query: Query<&PlacedCamera>,
    mut label_query: Query<(&mut Text, &mut TextColor), With<CameraPrimaryLabel>>,
) {
    let Ok((mut text, mut color)) = label_query.single_mut() else {
        return;
    };

    let cam = selection
        .entity
        .and_then(|e| camera_query.get(e).ok());

    if let Some(cam) = cam {
        if cam.is_primary {
            **text = "Primary: Yes".to_string();
            *color = TextColor(palette::SUNSHINE);
        } else {
            **text = "Primary: No".to_string();
            *color = TextColor(palette::SKY);
        }
    } else {
        **text = "Primary: (select a camera)".to_string();
        *color = TextColor(palette::CHAINMAIL);
    }
}
