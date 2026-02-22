use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObstacleId(pub String);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TriggerVolumeConfig {
    pub offset: Vec3,
    pub half_extents: Vec3,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObstacleDef {
    pub id: ObstacleId,
    pub glb_node_name: String,
    pub trigger_volume: Option<TriggerVolumeConfig>,
    pub is_gate: bool,
}
