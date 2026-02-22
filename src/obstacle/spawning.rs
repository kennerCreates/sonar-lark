use bevy::prelude::*;

use super::definition::{ObstacleId, TriggerVolumeConfig};

#[derive(Component)]
pub struct ObstacleMarker {
    pub id: ObstacleId,
}

#[derive(Component)]
pub struct TriggerVolume {
    pub half_extents: Vec3,
}

/// Handle to the loaded obstacles glTF asset.
#[derive(Resource)]
pub struct ObstaclesGltfHandle(pub Handle<bevy::gltf::Gltf>);

pub fn load_obstacles_gltf(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load("models/obstacles.glb");
    commands.insert_resource(ObstaclesGltfHandle(handle));
}

/// Spawn an obstacle from a named node in the glTF file.
///
/// Looks up the node by name, then spawns its mesh primitives as children
/// of a parent entity with the given transform.
pub fn spawn_obstacle(
    commands: &mut Commands,
    gltf_assets: &Assets<bevy::gltf::Gltf>,
    node_assets: &Assets<bevy::gltf::GltfNode>,
    mesh_assets: &Assets<bevy::gltf::GltfMesh>,
    gltf_handle: &ObstaclesGltfHandle,
    obstacle_id: &ObstacleId,
    node_name: &str,
    transform: Transform,
    trigger_config: Option<&TriggerVolumeConfig>,
    gate_index: Option<u32>,
) -> Option<Entity> {
    let gltf = gltf_assets.get(&gltf_handle.0)?;
    let node_handle = gltf.named_nodes.get(node_name)?;
    let node = node_assets.get(node_handle)?;
    let gltf_mesh_handle = node.mesh.as_ref()?;
    let gltf_mesh = mesh_assets.get(gltf_mesh_handle)?;

    let mut entity_commands = commands.spawn((
        transform,
        Visibility::default(),
        ObstacleMarker {
            id: obstacle_id.clone(),
        },
    ));

    if let Some(order) = gate_index {
        entity_commands.insert(crate::race::gate::GateIndex(order));
    }

    let entity = entity_commands.id();

    // Spawn each primitive as a child with its mesh and material
    for primitive in &gltf_mesh.primitives {
        let mut prim_commands = commands.spawn((
            Mesh3d(primitive.mesh.clone()),
            node.transform,
        ));

        if let Some(ref material) = primitive.material {
            prim_commands.insert(MeshMaterial3d(material.clone()));
        }

        prim_commands.set_parent_in_place(entity);
    }

    if let Some(trigger) = trigger_config {
        commands
            .spawn((
                Transform::from_translation(trigger.offset),
                TriggerVolume {
                    half_extents: trigger.half_extents,
                },
            ))
            .set_parent_in_place(entity);
    }

    Some(entity)
}
