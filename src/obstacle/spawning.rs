use bevy::prelude::*;

use super::definition::{ObstacleId, TriggerVolumeConfig};
use crate::palette;
use crate::rendering::{CelMaterial, cel_material_from_color};

#[derive(Component)]
pub struct ObstacleMarker {
    #[allow(dead_code)]
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

/// Run condition: true when the obstacles glTF and all its dependencies are loaded.
pub fn obstacles_gltf_ready(
    handle: Option<Res<ObstaclesGltfHandle>>,
    asset_server: Res<AssetServer>,
) -> bool {
    handle.is_some_and(|h| asset_server.is_loaded_with_dependencies(&h.0))
}

fn gate_color(obstacle_id: &ObstacleId) -> Option<Color> {
    match obstacle_id.0.as_str() {
        "gate_loop"   => Some(palette::DANDELION),
        "gate_ground" => Some(palette::CHERRY),
        "gate_best"   => Some(palette::LAVENDER),
        _ => None,
    }
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
    cel_materials: &mut Assets<CelMaterial>,
    std_materials: &Assets<StandardMaterial>,
    light_dir: Vec3,
    gltf_handle: &ObstaclesGltfHandle,
    obstacle_id: &ObstacleId,
    node_name: &str,
    transform: Transform,
    model_offset: Vec3,
    trigger_config: Option<&TriggerVolumeConfig>,
    gate_index: Option<u32>,
    gate_forward_flipped: bool,
) -> Option<Entity> {
    let gltf = gltf_assets.get(&gltf_handle.0)?;
    let node_handle = gltf.named_nodes.get(node_name)?;
    let node = node_assets.get(node_handle)?;
    let gltf_mesh_handle = node.mesh.as_ref()?;
    let gltf_mesh = mesh_assets.get(gltf_mesh_handle)?;

    // Pre-collect primitive meshes and materials before entering with_children, since
    // cel_materials is &mut and cannot be borrowed inside the closure alongside entity_commands.
    let mesh_transform = Transform {
        translation: model_offset,
        rotation: node.transform.rotation,
        scale: node.transform.scale,
    };
    let override_mat = gate_color(obstacle_id)
        .map(|color| cel_materials.add(cel_material_from_color(color, light_dir)));
    let primitives: Vec<(Handle<Mesh>, MeshMaterial3d<CelMaterial>)> = gltf_mesh
        .primitives
        .iter()
        .map(|p| {
            let mat = match &override_mat {
                Some(m) => MeshMaterial3d(m.clone()),
                None => {
                    let base_color = p.material
                        .as_ref()
                        .and_then(|h| std_materials.get(h))
                        .map(|m| m.base_color)
                        .unwrap_or(palette::CHAINMAIL);
                    MeshMaterial3d(cel_materials.add(cel_material_from_color(base_color, light_dir)))
                }
            };
            (p.mesh.clone(), mat)
        })
        .collect();

    let mut entity_commands = commands.spawn((
        transform,
        Visibility::default(),
        ObstacleMarker {
            id: obstacle_id.clone(),
        },
        DespawnOnExit(crate::states::AppState::Results),
    ));

    if let Some(order) = gate_index {
        entity_commands.insert(crate::race::gate::GateIndex(order));
        if let Some(trigger) = trigger_config {
            let local_fwd = if gate_forward_flipped { -trigger.forward } else { trigger.forward };
            let world_fwd = transform.rotation * local_fwd;
            entity_commands.insert(crate::race::gate::GateForward(world_fwd));
        }
    }

    // Spawn children directly via with_children so their local Transform is never adjusted
    // by set_parent_in_place (which would zero it out because GlobalTransform hasn't been
    // propagated yet for newly-spawned entities).
    entity_commands.with_children(|children| {
        for (mesh, material) in primitives {
            children.spawn((Mesh3d(mesh), material, mesh_transform));
        }

        if let Some(trigger) = trigger_config {
            children.spawn((
                Transform::from_translation(trigger.offset),
                TriggerVolume {
                    half_extents: trigger.half_extents,
                },
            ));
        }
    });

    Some(entity_commands.id())
}
