use bevy::prelude::*;

use super::definition::{CollisionVolumeConfig, ObstacleId, TriggerVolumeConfig};
use crate::palette;
use crate::rendering::{CelLightDir, CelMaterial, cel_material_from_color};

#[derive(Component)]
pub struct ObstacleMarker;

#[derive(Component)]
pub struct TriggerVolume {
    pub half_extents: Vec3,
}

pub struct CollisionVolumeEntry {
    pub offset: Vec3,
    pub half_extents: Vec3,
    pub rotation: Quat,
}

#[derive(Component)]
pub struct ObstacleCollisionVolumes {
    pub volumes: Vec<CollisionVolumeEntry>,
    pub is_gate: bool,
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

pub(crate) fn gate_color(obstacle_id: &ObstacleId) -> Option<Color> {
    match obstacle_id.0.as_str() {
        "gate_loop"   => Some(palette::DANDELION),
        "gate_ground" => Some(palette::CHERRY),
        "gate_best"   => Some(palette::LAVENDER),
        _ => None,
    }
}

/// Bundles the shared asset handles needed by `spawn_obstacle`.
///
/// Construct this in calling systems from their system params, then pass it
/// to [`SpawnObstacleContext::spawn`] for each obstacle.
pub struct SpawnObstacleContext<'a> {
    pub gltf_assets: &'a Assets<bevy::gltf::Gltf>,
    pub node_assets: &'a Assets<bevy::gltf::GltfNode>,
    pub mesh_assets: &'a Assets<bevy::gltf::GltfMesh>,
    pub cel_materials: &'a mut Assets<CelMaterial>,
    pub std_materials: &'a Assets<StandardMaterial>,
    pub light_dir: Vec3,
    pub gltf_handle: &'a ObstaclesGltfHandle,
}

impl<'a> SpawnObstacleContext<'a> {
    pub fn from_res(
        gltf_assets: &'a Assets<bevy::gltf::Gltf>,
        node_assets: &'a Assets<bevy::gltf::GltfNode>,
        mesh_assets: &'a Assets<bevy::gltf::GltfMesh>,
        cel_materials: &'a mut Assets<CelMaterial>,
        std_materials: &'a Assets<StandardMaterial>,
        light_dir: &'a CelLightDir,
        gltf_handle: &'a ObstaclesGltfHandle,
    ) -> Self {
        Self {
            gltf_assets,
            node_assets,
            mesh_assets,
            cel_materials,
            std_materials,
            light_dir: light_dir.0,
            gltf_handle,
        }
    }
}

/// Spawn an obstacle from a named node in the glTF file.
///
/// Looks up the node by name, then spawns its mesh primitives as children
/// of a parent entity with the given transform.
pub fn spawn_obstacle(
    commands: &mut Commands,
    ctx: &mut SpawnObstacleContext,
    obstacle_id: &ObstacleId,
    node_name: &str,
    transform: Transform,
    model_offset: Vec3,
    model_rotation: Quat,
    trigger_config: Option<&TriggerVolumeConfig>,
    gate_index: Option<u32>,
    gate_forward_flipped: bool,
    collision_configs: &[CollisionVolumeConfig],
    color_override: Option<Color>,
) -> Option<Entity> {
    let gltf = ctx.gltf_assets.get(&ctx.gltf_handle.0)?;
    let node_handle = gltf.named_nodes.get(node_name)?;
    let node = ctx.node_assets.get(node_handle)?;
    let gltf_mesh_handle = node.mesh.as_ref()?;
    let gltf_mesh = ctx.mesh_assets.get(gltf_mesh_handle)?;

    // Pre-collect primitive meshes and materials before entering with_children, since
    // cel_materials is &mut and cannot be borrowed inside the closure alongside entity_commands.
    let mesh_transform = Transform {
        translation: model_offset,
        rotation: model_rotation * node.transform.rotation,
        scale: node.transform.scale,
    };
    let override_mat = color_override
        .or_else(|| gate_color(obstacle_id))
        .map(|color| ctx.cel_materials.add(cel_material_from_color(color, ctx.light_dir)));
    let primitives: Vec<(Handle<Mesh>, MeshMaterial3d<CelMaterial>)> = gltf_mesh
        .primitives
        .iter()
        .map(|p| {
            let mat = match &override_mat {
                Some(m) => MeshMaterial3d(m.clone()),
                None => {
                    let base_color = p.material
                        .as_ref()
                        .and_then(|h| ctx.std_materials.get(h))
                        .map(|m| m.base_color)
                        .unwrap_or(palette::CHAINMAIL);
                    MeshMaterial3d(ctx.cel_materials.add(cel_material_from_color(base_color, ctx.light_dir)))
                }
            };
            (p.mesh.clone(), mat)
        })
        .collect();

    let mut entity_commands = commands.spawn((
        transform,
        Visibility::default(),
        ObstacleMarker,
        DespawnOnExit(crate::states::AppState::Results),
    ));

    if let Some(order) = gate_index {
        entity_commands.insert(crate::race::gate::GateIndex(order));
        if let Some(trigger) = trigger_config {
            let local_fwd = if gate_forward_flipped { -trigger.forward } else { trigger.forward };
            let world_fwd = transform.rotation * trigger.rotation * local_fwd;
            entity_commands.insert(crate::race::gate::GateForward(world_fwd));
        }
    }

    if !collision_configs.is_empty() {
        entity_commands.insert(ObstacleCollisionVolumes {
            volumes: collision_configs
                .iter()
                .map(|c| CollisionVolumeEntry {
                    offset: c.offset,
                    half_extents: c.half_extents,
                    rotation: c.rotation,
                })
                .collect(),
            is_gate: trigger_config.is_some(),
        });
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
                Transform::from_translation(trigger.offset).with_rotation(trigger.rotation),
                TriggerVolume {
                    half_extents: trigger.half_extents,
                },
            ));
        }
    });

    Some(entity_commands.id())
}
