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

pub fn spawn_obstacle(
    commands: &mut Commands,
    gltf_assets: &Assets<bevy::gltf::Gltf>,
    gltf_handle: &ObstaclesGltfHandle,
    obstacle_id: &ObstacleId,
    scene_name: &str,
    transform: Transform,
    trigger_config: Option<&TriggerVolumeConfig>,
    gate_index: Option<u32>,
) -> Option<Entity> {
    let gltf = gltf_assets.get(&gltf_handle.0)?;
    let scene_handle = gltf.named_scenes.get(scene_name)?;

    let mut entity_commands = commands.spawn((
        SceneRoot(scene_handle.clone()),
        transform,
        ObstacleMarker {
            id: obstacle_id.clone(),
        },
    ));

    if let Some(order) = gate_index {
        entity_commands.insert(crate::race::gate::GateIndex(order));
    }

    let entity = entity_commands.id();

    if let Some(trigger) = trigger_config {
        commands.spawn((
            Transform::from_translation(trigger.offset),
            TriggerVolume {
                half_extents: trigger.half_extents,
            },
        )).set_parent_in_place(entity);
    }

    Some(entity)
}
