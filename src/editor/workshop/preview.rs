use bevy::prelude::*;

use crate::obstacle::spawning::ObstaclesGltfHandle;
use crate::palette;
use crate::rendering::{CelLightDir, CelMaterial, cel_material_from_color};

use super::{CameraPreview, PreviewObstacle, WorkshopState};

/// Spawn a preview from a named node in the glTF.
///
/// Parent entity gets `model_offset` as translation and `model_rotation` as rotation.
/// Child meshes get the node's Blender-authored rotation and scale.
pub fn spawn_preview(
    commands: &mut Commands,
    gltf_assets: &Assets<bevy::gltf::Gltf>,
    node_assets: &Assets<bevy::gltf::GltfNode>,
    mesh_assets: &Assets<bevy::gltf::GltfMesh>,
    cel_materials: &mut Assets<CelMaterial>,
    std_materials: &Assets<StandardMaterial>,
    light_dir: Vec3,
    gltf_handle: &ObstaclesGltfHandle,
    node_name: &str,
    model_offset: Vec3,
    model_rotation: Quat,
) -> Option<Entity> {
    let gltf = gltf_assets.get(&gltf_handle.0)?;
    let node_handle = gltf.named_nodes.get(node_name)?;
    let node = node_assets.get(node_handle)?;
    let gltf_mesh_handle = node.mesh.as_ref()?;
    let gltf_mesh = mesh_assets.get(gltf_mesh_handle)?;

    let parent_transform =
        Transform::from_translation(model_offset).with_rotation(model_rotation);
    let child_transform = Transform {
        rotation: node.transform.rotation,
        scale: node.transform.scale,
        ..default()
    };

    // Pre-collect materials before spawning to avoid borrow conflicts
    let primitives: Vec<(Handle<Mesh>, MeshMaterial3d<CelMaterial>)> = gltf_mesh
        .primitives
        .iter()
        .map(|p| {
            let base_color = p.material
                .as_ref()
                .and_then(|h| std_materials.get(h))
                .map(|m| m.base_color)
                .unwrap_or(Color::srgb(0.502, 0.475, 0.502)); // Chainmail #807980
            let mat = MeshMaterial3d(cel_materials.add(cel_material_from_color(base_color, light_dir)));
            (p.mesh.clone(), mat)
        })
        .collect();

    let parent = commands
        .spawn((
            parent_transform,
            Visibility::default(),
            PreviewObstacle,
        ))
        .id();

    for (mesh, material) in primitives {
        commands
            .spawn((
                Mesh3d(mesh),
                material,
                child_transform,
            ))
            .set_parent_in_place(parent);
    }

    Some(parent)
}

pub(super) fn spawn_placeholder_preview(
    mut commands: Commands,
    mut state: ResMut<WorkshopState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    std_materials: Res<Assets<StandardMaterial>>,
    light_dir: Res<CelLightDir>,
    gltf_handle: Option<Res<ObstaclesGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    node_assets: Res<Assets<bevy::gltf::GltfNode>>,
    mesh_assets: Res<Assets<bevy::gltf::GltfMesh>>,
) {
    if state.node_name.is_empty() || state.preview_entity.is_some() {
        return;
    }

    let offset = state.model_offset;
    let rotation = state.model_rotation;

    // Try to spawn from glTF first
    if let Some(handle) = &gltf_handle
        && let Some(entity) = spawn_preview(
            &mut commands,
            &gltf_assets,
            &node_assets,
            &mesh_assets,
            &mut cel_materials,
            &std_materials,
            light_dir.0,
            handle,
            &state.node_name,
            offset,
            rotation,
        )
    {
        state.preview_entity = Some(entity);
        return;
    }

    // No matching glTF node — spawn a placeholder cube
    let entity = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(cel_materials.add(cel_material_from_color(
                palette::CHAINMAIL,
                light_dir.0,
            ))),
            Transform::from_translation(offset).with_rotation(rotation),
            PreviewObstacle,
        ))
        .id();
    state.preview_entity = Some(entity);
}

/// Spawns, updates, or despawns the camera preview mesh based on workshop state.
pub(super) fn update_camera_preview(
    mut commands: Commands,
    state: Res<WorkshopState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    light_dir: Res<CelLightDir>,
    preview_query: Query<&Transform, With<PreviewObstacle>>,
    mut camera_preview: Query<
        (Entity, &mut Transform),
        (With<CameraPreview>, Without<PreviewObstacle>),
    >,
) {
    let should_exist = state.has_camera && state.preview_entity.is_some();

    if !should_exist {
        for (entity, _) in &camera_preview {
            commands.entity(entity).despawn();
        }
        return;
    }

    let preview_pos = state
        .preview_entity
        .and_then(|e| preview_query.get(e).ok())
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    let target = Transform::from_translation(preview_pos + state.camera_offset)
        .with_rotation(state.camera_rotation);

    if let Some((_, mut transform)) = camera_preview.iter_mut().next() {
        *transform = target;
    } else {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.3, 0.3, 0.5))),
            MeshMaterial3d(cel_materials.add(cel_material_from_color(
                palette::SKY,
                light_dir.0,
            ))),
            target,
            Visibility::default(),
            CameraPreview,
        ));
    }
}
